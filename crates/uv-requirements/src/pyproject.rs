use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use glob::Pattern;
use indexmap::IndexMap;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use pep508_rs::{GitVersion, Requirement, UvRequirement, UvRequirements, UvSource, VerbatimUrl};
use uv_normalize::{ExtraName, PackageName};

use crate::ExtrasSpecification;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Pep621Error {
    #[error(transparent)]
    Pep508(#[from] pep508_rs::Pep508Error),
    #[error("Missing entry `{0}`")]
    MissingEntry(&'static str),
    #[error("TODO")]
    LoweringError(#[from] anyhow::Error),
}

/// A `pyproject.toml` as specified in PEP 517.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PyProjectToml {
    /// Project metadata
    pub(crate) project: Option<Project>,
    /// Uv additions
    pub(crate) tool: Option<Tool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tool {
    pub(crate) uv: Option<Uv>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Uv {
    pub(crate) sources: Option<HashMap<PackageName, Source>>,
    pub(crate) workspace: Option<UvWorkspace>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct UvWorkspace {
    pub(crate) members: Option<Vec<SerdePattern>>,
    pub(crate) exclude: Option<Vec<SerdePattern>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct SerdePattern(#[serde(with = "string")] pub(crate) Pattern);

impl Deref for SerdePattern {
    type Target = Pattern;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum Source {
    Git {
        git: String,
        // Only one of the three may be used, we validate this later for a better error message.
        rev: Option<String>,
        tag: Option<String>,
        branch: Option<String>,
    },
    Url {
        url: String,
    },
    Path {
        patch: String,
        /// `false` by default.
        editable: Option<bool>,
    },
    Registry {
        // TODO(konstin): The string is more-or-less a placeholder
        index: String,
    },
    Workspace {
        workspace: bool,
        /// `true` by default.
        editable: Option<bool>,
    },
    /// Show a better error message for invalid combinations of options.
    CatchAll {
        git: String,
        rev: Option<String>,
        tag: Option<String>,
        branch: Option<String>,
        url: String,
        patch: String,
        index: String,
        workspace: bool,
    },
}

/// PEP 621 project metadata.
///
/// This is a subset of the full metadata specification, and only includes the fields that are
/// relevant for extracting static requirements.
///
/// See <https://packaging.python.org/en/latest/specifications/pyproject-toml>.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Project {
    /// The name of the project
    pub(crate) name: PackageName,
    /// Project dependencies
    pub(crate) dependencies: Option<Vec<String>>,
    /// Optional dependencies
    pub(crate) optional_dependencies: Option<IndexMap<ExtraName, Vec<String>>>,
    /// Specifies which fields listed by PEP 621 were intentionally unspecified
    /// so another tool can/will provide such metadata dynamically.
    pub(crate) dynamic: Option<Vec<String>>,
}

/// The PEP 621 project metadata, with static requirements extracted in advance, joined
/// with `tool.uv.sources`.
#[derive(Debug)]
pub(crate) struct UvMetadata {
    /// The name of the project.
    pub(crate) name: PackageName,
    /// The requirements extracted from the project.
    pub(crate) requirements: Vec<UvRequirement>,
    /// The extras used to collect requirements.
    pub(crate) used_extras: FxHashSet<ExtraName>,
}

impl UvMetadata {
    /// Extract the static [`UvMetadata`] from a [`Project`] and [`ExtrasSpecification`], if
    /// possible.
    ///
    /// If the project specifies dynamic dependencies, or if the project specifies dynamic optional
    /// dependencies and the extras are requested, the requirements cannot be extracted.
    ///
    /// Returns an error if the requirements are not valid PEP 508 requirements.
    pub(crate) fn try_from(
        pyproject: PyProjectToml,
        extras: &ExtrasSpecification,
        workspace_sources: &HashMap<PackageName, Source>,
        workspace_packages: &HashMap<PackageName, PathBuf>,
    ) -> Result<Option<Self>, Pep621Error> {
        let project_sources = pyproject
            .tool
            .as_ref()
            .and_then(|tool| tool.uv.as_ref())
            .and_then(|uv| uv.sources.clone());

        let has_sources = project_sources.is_some() || !workspace_sources.is_empty();

        let Some(project) = pyproject.project else {
            return if has_sources {
                Err(Pep621Error::MissingEntry("[project]"))
            } else {
                Ok(None)
            };
        };
        if let Some(dynamic) = project.dynamic.as_ref() {
            // If the project specifies dynamic dependencies, we can't extract the requirements.
            if dynamic.iter().any(|field| field == "dependencies") {
                return if has_sources {
                    Err(Pep621Error::MissingEntry("[project.dependencies]"))
                } else {
                    Ok(None)
                };
            }
            // If we requested extras, and the project specifies dynamic optional dependencies, we can't
            // extract the requirements.
            if !extras.is_empty() && dynamic.iter().any(|field| field == "optional-dependencies") {
                return if has_sources {
                    Err(Pep621Error::MissingEntry("[project.optional-dependencies]"))
                } else {
                    Ok(None)
                };
            }
        }

        let name = project.name;

        let uv_requirements = lower_requirements(
            &project.dependencies.unwrap_or_default(),
            &project.optional_dependencies.unwrap_or_default(),
            &project_sources.unwrap_or_default(),
            workspace_sources,
            workspace_packages,
        )?;

        // Parse out the project requirements.
        let mut requirements = uv_requirements.dependencies;

        // Include any optional dependencies specified in `extras`.
        let mut used_extras = FxHashSet::default();
        if !extras.is_empty() {
            // Include the optional dependencies if the extras are requested.
            for (extra, optional_requirements) in &uv_requirements.optional_dependencies {
                if extras.contains(extra) {
                    used_extras.insert(extra.clone());
                    requirements.extend(flatten_extra(
                        &name,
                        optional_requirements,
                        &uv_requirements.optional_dependencies,
                    ));
                }
            }
        }

        Ok(Some(Self {
            name,
            requirements,
            used_extras,
        }))
    }
}

pub(crate) fn lower_requirements(
    dependencies: &[String],
    optional_dependencies: &IndexMap<ExtraName, Vec<String>>,
    project_sources: &HashMap<PackageName, Source>,
    workspace_sources: &HashMap<PackageName, Source>,
    workspace_packages: &HashMap<PackageName, PathBuf>,
) -> anyhow::Result<UvRequirements> {
    let dependencies = dependencies
        .iter()
        .map(|dependency| {
            let requirement = Requirement::from_str(dependency)?;
            let name = requirement.name.clone();
            lower_requirement(
                requirement,
                project_sources,
                workspace_sources,
                workspace_packages,
            )
            .with_context(|| format!("Failed to parse entry for requirement {name}"))
        })
        .collect::<anyhow::Result<_>>()?;
    let optional_dependencies = optional_dependencies
        .iter()
        .map(|(extra_name, dependencies)| {
            let dependencies: Vec<_> = dependencies
                .iter()
                .map(|dependency| {
                    let requirement = Requirement::from_str(dependency)?;
                    let name = requirement.name.clone();
                    lower_requirement(
                        requirement,
                        project_sources,
                        workspace_sources,
                        workspace_packages,
                    )
                    .with_context(|| format!("Failed to parse entry for requirement {name}"))
                })
                .collect::<anyhow::Result<_>>()?;
            Ok((extra_name.clone(), dependencies))
        })
        .collect::<anyhow::Result<_>>()?;
    Ok(UvRequirements {
        dependencies,
        optional_dependencies,
    })
}

/// Combine `project.dependencies`/`project.optional-dependencies` with `tool.uv.sources`.
pub(crate) fn lower_requirement(
    requirement: Requirement,
    project_sources: &HashMap<PackageName, Source>,
    workspace_sources: &HashMap<PackageName, Source>,
    workspace_packages: &HashMap<PackageName, PathBuf>,
) -> anyhow::Result<UvRequirement> {
    let source = project_sources
        .get(&requirement.name)
        .or(workspace_sources.get(&requirement.name))
        .cloned();
    if !matches!(
        source,
        Some(Source::Workspace {
            // By using toml, we technically support `workspace = false`.
            workspace: true,
            ..
        })
    ) && workspace_packages.contains_key(&requirement.name)
    {
        bail!("The package is a workspace package, to use it you have to specify `{} = {{ workspace = true }} in `tool.uv.sources`.", requirement.name)
    }

    let Some(source) = source else {
        return if requirement.version_or_url.is_none() {
            Err(anyhow!("You need to specify a version constraint"))
        } else {
            Ok(UvRequirement::from_requirement(requirement))
        };
    };

    let source = match source {
        Source::Git {
            git,
            rev,
            tag,
            branch,
        } => {
            let git_ref = match (rev, tag, branch) {
                (None, None, None) => None,
                (Some(rev), None, None) => Some(GitVersion::Rev(rev)),
                (None, Some(tag), None) => Some(GitVersion::Tag(tag)),
                (None, None, Some(branch)) => Some(GitVersion::Branch(branch)),
                _ => bail!("You can only use one of rev, tag or branch."),
            };

            UvSource::Git {
                git: VerbatimUrl::from_str(&git)?,
                version: git_ref,
            }
        }
        Source::Url { url } => UvSource::Url {
            url: VerbatimUrl::from_str(&url)?,
        },
        Source::Path { .. } => todo!(),
        Source::Registry { .. } => todo!(),
        Source::Workspace { .. } => todo!(),
        Source::CatchAll { .. } => {
            // This is better than a serde error about not matching any enum variant
            bail!(
                "You can't combine these options in `tool.uv.sources` for {}",
                requirement.name
            )
        }
    };
    Ok(UvRequirement {
        name: requirement.name,
        extras: requirement.extras,
        marker: requirement.marker,
        source,
    })
}

/// Given an extra in a project that may contain references to the project
/// itself, flatten it into a list of requirements.
///
/// For example:
/// ```toml
/// [project]
/// name = "my-project"
/// version = "0.0.1"
/// dependencies = [
///     "tomli",
/// ]
///
/// [project.optional-dependencies]
/// test = [
///     "pep517",
/// ]
/// dev = [
///     "my-project[test]",
/// ]
/// ```
fn flatten_extra(
    project_name: &PackageName,
    requirements: &[UvRequirement],
    extras: &IndexMap<ExtraName, Vec<UvRequirement>>,
) -> Vec<UvRequirement> {
    fn inner(
        project_name: &PackageName,
        requirements: &[UvRequirement],
        extras: &IndexMap<ExtraName, Vec<UvRequirement>>,
        seen: &mut FxHashSet<ExtraName>,
    ) -> Vec<UvRequirement> {
        let mut flattened = Vec::with_capacity(requirements.len());
        for requirement in requirements {
            if requirement.name == *project_name {
                for extra in &requirement.extras {
                    // Avoid infinite recursion on mutually recursive extras.
                    if !seen.insert(extra.clone()) {
                        continue;
                    }

                    // Flatten the extra requirements.
                    for (other_extra, extra_requirements) in extras {
                        if other_extra == extra {
                            flattened.extend(inner(project_name, extra_requirements, extras, seen));
                        }
                    }
                }
            } else {
                flattened.push(requirement.clone());
            }
        }
        flattened
    }

    inner(
        project_name,
        requirements,
        extras,
        &mut FxHashSet::default(),
    )
}

/// <https://github.com/serde-rs/serde/issues/1316#issue-332908452>
mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub(super) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}
