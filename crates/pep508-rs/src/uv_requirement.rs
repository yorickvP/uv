#![allow(missing_docs)]

use indexmap::IndexMap;
use std::fmt::{Display, Formatter};

use crate::{MarkerEnvironment, MarkerTree, Requirement, VerbatimUrl, VersionOrUrl};
use pep440_rs::VersionSpecifiers;
use uv_normalize::{ExtraName, PackageName};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UvRequirements {
    pub dependencies: Vec<UvRequirement>,
    pub optional_dependencies: IndexMap<ExtraName, Vec<UvRequirement>>,
}

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
pub struct UvRequirement {
    pub name: PackageName,
    pub extras: Vec<ExtraName>,
    pub marker: Option<MarkerTree>,
    pub source: UvSource,
}

impl UvRequirement {
    /// Returns whether the markers apply for the given environment.
    pub fn evaluate_markers(&self, env: &MarkerEnvironment, extras: &[ExtraName]) -> bool {
        if let Some(marker) = &self.marker {
            marker.evaluate(env, extras)
        } else {
            true
        }
    }

    pub fn from_requirement(requirement: Requirement) -> Self {
        let source = match requirement.version_or_url {
            None => UvSource::Registry {
                version: VersionSpecifiers::empty(),
                index: None,
            },
            // The most popular case: Just a name, a version range and maybe extras.
            Some(VersionOrUrl::VersionSpecifier(version)) => UvSource::Registry {
                version,
                index: None,
            },
            Some(VersionOrUrl::Url(_url)) => {
                todo!("match on the url type")
            }
        };
        UvRequirement {
            name: requirement.name,
            extras: requirement.extras,
            marker: requirement.marker,
            source,
        }
    }
}

impl Display for UvRequirement {
    /// Note: This is for user display, not for requirements.txt
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.extras.is_empty() {
            write!(
                f,
                "[{}]",
                self.extras
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )?
        }
        match &self.source {
            UvSource::Registry { version, index } => {
                write!(f, "{}", version)?;
                if let Some(index) = index {
                    write!(f, " (index: {})", index)?;
                }
            }
            UvSource::Url { url } => {
                write!(f, " @ {}", url)?;
            }
            UvSource::Git { .. } => todo!(),
            UvSource::Path { .. } => todo!(),
        }
        if let Some(marker) = &self.marker {
            write!(f, " ; {}", marker)?;
        }
        Ok(())
    }
}

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
pub enum UvSource {
    Registry {
        version: VersionSpecifiers,
        index: Option<String>,
    },
    Url {
        url: VerbatimUrl,
    },
    Git {
        git: VerbatimUrl,
        version: Option<GitVersion>,
    },
    Path {
        path: String,
    },
}

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
pub enum GitVersion {
    Rev(String),
    Tag(String),
    Branch(String),
}
