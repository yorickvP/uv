use std::path::PathBuf;

use distribution_types::{FlatIndexLocation, IndexUrl};
use install_wheel_rs::linker::LinkMode;
use uv_cache::CacheArgs;
use uv_configuration::{ConfigSettings, IndexStrategy, KeyringProviderType, PackageNameSpecifier};
use uv_normalize::{ExtraName, PackageName};
use uv_resolver::{AnnotationStyle, ExcludeNewer, PreReleaseMode, ResolutionMode};
use uv_toolchain::PythonVersion;
use uv_workspace::{PipOptions, Workspace};

use crate::cli::{
    ColorChoice, GlobalArgs, Maybe, PipCheckArgs, PipCompileArgs, PipFreezeArgs, PipInstallArgs,
    PipListArgs, PipShowArgs, PipSyncArgs, PipUninstallArgs, VenvArgs,
};
use crate::commands::ListFormat;

/// The resolved global settings to use for any invocation of the CLI.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct GlobalSettings {
    pub(crate) quiet: bool,
    pub(crate) verbose: u8,
    pub(crate) color: ColorChoice,
    pub(crate) native_tls: bool,
}

impl GlobalSettings {
    /// Resolve the [`GlobalSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: GlobalArgs, workspace: Option<&Workspace>) -> Self {
        Self {
            quiet: args.quiet,
            verbose: args.verbose,
            color: if args.no_color {
                ColorChoice::Never
            } else {
                args.color
            },
            native_tls: flag(args.native_tls, args.no_native_tls)
                .or(workspace.and_then(|workspace| workspace.options.native_tls))
                .unwrap_or(false),
        }
    }
}

/// The resolved cache settings to use for any invocation of the CLI.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct CacheSettings {
    pub(crate) no_cache: Option<bool>,
    pub(crate) cache_dir: Option<PathBuf>,
}

impl CacheSettings {
    /// Resolve the [`CacheSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: CacheArgs, workspace: Option<&Workspace>) -> Self {
        Self {
            no_cache: args
                .no_cache
                .or(workspace.and_then(|workspace| workspace.options.no_cache)),
            cache_dir: args
                .cache_dir
                .or_else(|| workspace.and_then(|workspace| workspace.options.cache_dir.clone())),
        }
    }
}

/// The resolved settings to use for a `pip compile` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipCompileSettings {
    // CLI-only settings.
    pub(crate) src_file: Vec<PathBuf>,
    pub(crate) constraint: Vec<PathBuf>,
    pub(crate) r#override: Vec<PathBuf>,
    pub(crate) refresh: bool,
    pub(crate) refresh_package: Vec<PackageName>,
    pub(crate) upgrade: bool,
    pub(crate) upgrade_package: Vec<PackageName>,

    // Shared settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipCompileSettings {
    /// Resolve the [`PipCompileSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipCompileArgs, workspace: Option<Workspace>) -> Self {
        let PipCompileArgs {
            src_file,
            constraint,
            r#override,
            extra,
            all_extras,
            no_all_extras,
            no_deps,
            deps,
            resolution,
            prerelease,
            pre,
            output_file,
            no_strip_extras,
            strip_extras,
            no_annotate,
            annotate,
            no_header,
            header,
            annotation_style,
            custom_compile_command,
            offline,
            no_offline,
            refresh,
            refresh_package,
            link_mode,
            index_url,
            extra_index_url,
            no_index,
            index_strategy,
            keyring_provider,
            find_links,
            python,
            system,
            no_system,
            upgrade,
            upgrade_package,
            generate_hashes,
            no_generate_hashes,
            legacy_setup_py,
            no_legacy_setup_py,
            no_build_isolation,
            build_isolation,
            no_build,
            build,
            only_binary,
            config_setting,
            python_version,
            exclude_newer,
            no_emit_package,
            emit_index_url,
            no_emit_index_url,
            emit_find_links,
            no_emit_find_links,
            emit_marker_expression,
            no_emit_marker_expression,
            emit_index_annotation,
            no_emit_index_annotation,
            compat_args: _,
        } = args;

        Self {
            // CLI-only settings.
            src_file,
            constraint,
            r#override,
            refresh,
            refresh_package: refresh_package.unwrap_or_default(),
            upgrade,
            upgrade_package: upgrade_package.unwrap_or_default(),

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    offline: flag(offline, no_offline),
                    index_url: index_url.and_then(Maybe::into_option),
                    extra_index_url: extra_index_url.map(|extra_index_urls| {
                        extra_index_urls
                            .into_iter()
                            .filter_map(Maybe::into_option)
                            .collect()
                    }),
                    no_index: Some(no_index),
                    find_links,
                    index_strategy,
                    keyring_provider,
                    no_build: flag(no_build, build),
                    only_binary,
                    no_build_isolation: flag(no_build_isolation, build_isolation),
                    extra,
                    all_extras: flag(all_extras, no_all_extras),
                    no_deps: flag(no_deps, deps),
                    resolution,
                    prerelease: if pre {
                        Some(PreReleaseMode::Allow)
                    } else {
                        prerelease
                    },
                    output_file,
                    no_strip_extras: flag(no_strip_extras, strip_extras),
                    no_annotate: flag(no_annotate, annotate),
                    no_header: flag(no_header, header),
                    custom_compile_command,
                    generate_hashes: flag(generate_hashes, no_generate_hashes),
                    legacy_setup_py: flag(legacy_setup_py, no_legacy_setup_py),
                    config_settings: config_setting.map(|config_settings| {
                        config_settings.into_iter().collect::<ConfigSettings>()
                    }),
                    python_version,
                    exclude_newer,
                    no_emit_package,
                    emit_index_url: flag(emit_index_url, no_emit_index_url),
                    emit_find_links: flag(emit_find_links, no_emit_find_links),
                    emit_marker_expression: flag(emit_marker_expression, no_emit_marker_expression),
                    emit_index_annotation: flag(emit_index_annotation, no_emit_index_annotation),
                    annotation_style,
                    link_mode,
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip sync` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipSyncSettings {
    // CLI-only settings.
    pub(crate) src_file: Vec<PathBuf>,
    pub(crate) reinstall: bool,
    pub(crate) reinstall_package: Vec<PackageName>,
    pub(crate) refresh: bool,
    pub(crate) refresh_package: Vec<PackageName>,

    // Shared settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipSyncSettings {
    /// Resolve the [`PipSyncSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipSyncArgs, workspace: Option<Workspace>) -> Self {
        let PipSyncArgs {
            src_file,
            reinstall,
            reinstall_package,
            offline,
            refresh,
            no_offline,
            refresh_package,
            link_mode,
            index_url,
            extra_index_url,
            find_links,
            no_index,
            index_strategy,
            require_hashes,
            no_require_hashes,
            keyring_provider,
            python,
            system,
            no_system,
            break_system_packages,
            no_break_system_packages,
            legacy_setup_py,
            no_legacy_setup_py,
            no_build_isolation,
            build_isolation,
            no_build,
            build,
            no_binary,
            only_binary,
            compile_bytecode,
            no_compile_bytecode,
            config_setting,
            strict,
            no_strict,
            compat_args: _,
        } = args;

        Self {
            // CLI-only settings.
            src_file,
            reinstall,
            reinstall_package,
            refresh,
            refresh_package,

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    break_system_packages: flag(break_system_packages, no_break_system_packages),
                    offline: flag(offline, no_offline),
                    index_url: index_url.and_then(Maybe::into_option),
                    extra_index_url: extra_index_url.map(|extra_index_urls| {
                        extra_index_urls
                            .into_iter()
                            .filter_map(Maybe::into_option)
                            .collect()
                    }),
                    no_index: Some(no_index),
                    find_links,
                    index_strategy,
                    keyring_provider,
                    no_build: flag(no_build, build),
                    no_binary,
                    only_binary,
                    no_build_isolation: flag(no_build_isolation, build_isolation),
                    strict: flag(strict, no_strict),
                    legacy_setup_py: flag(legacy_setup_py, no_legacy_setup_py),
                    config_settings: config_setting.map(|config_settings| {
                        config_settings.into_iter().collect::<ConfigSettings>()
                    }),
                    link_mode,
                    compile_bytecode: flag(compile_bytecode, no_compile_bytecode),
                    require_hashes: flag(require_hashes, no_require_hashes),
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip install` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipInstallSettings {
    // CLI-only settings.
    pub(crate) package: Vec<String>,
    pub(crate) requirement: Vec<PathBuf>,
    pub(crate) editable: Vec<String>,
    pub(crate) constraint: Vec<PathBuf>,
    pub(crate) r#override: Vec<PathBuf>,
    pub(crate) upgrade: bool,
    pub(crate) upgrade_package: Vec<PackageName>,
    pub(crate) reinstall: bool,
    pub(crate) reinstall_package: Vec<PackageName>,
    pub(crate) refresh: bool,
    pub(crate) refresh_package: Vec<PackageName>,
    pub(crate) dry_run: bool,
    // Shared settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipInstallSettings {
    /// Resolve the [`PipInstallSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipInstallArgs, workspace: Option<Workspace>) -> Self {
        let PipInstallArgs {
            package,
            requirement,
            editable,
            constraint,
            r#override,
            extra,
            all_extras,
            no_all_extras,
            upgrade,
            upgrade_package,
            reinstall,
            reinstall_package,
            offline,
            refresh,
            no_offline,
            refresh_package,
            no_deps,
            deps,
            link_mode,
            resolution,
            prerelease,
            pre,
            index_url,
            extra_index_url,
            find_links,
            no_index,
            index_strategy,
            require_hashes,
            no_require_hashes,
            keyring_provider,
            python,
            system,
            no_system,
            break_system_packages,
            no_break_system_packages,
            legacy_setup_py,
            no_legacy_setup_py,
            no_build_isolation,
            build_isolation,
            no_build,
            build,
            no_binary,
            only_binary,
            compile_bytecode,
            no_compile_bytecode,
            config_setting,
            strict,
            no_strict,
            exclude_newer,
            dry_run,
        } = args;

        Self {
            // CLI-only settings.
            package,
            requirement,
            editable,
            constraint,
            r#override,
            upgrade,
            upgrade_package: upgrade_package.unwrap_or_default(),
            reinstall,
            reinstall_package: reinstall_package.unwrap_or_default(),
            refresh,
            refresh_package: refresh_package.unwrap_or_default(),
            dry_run,

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    break_system_packages: flag(break_system_packages, no_break_system_packages),
                    offline: flag(offline, no_offline),
                    index_url: index_url.and_then(Maybe::into_option),
                    extra_index_url: extra_index_url.map(|extra_index_urls| {
                        extra_index_urls
                            .into_iter()
                            .filter_map(Maybe::into_option)
                            .collect()
                    }),
                    no_index: Some(no_index),
                    find_links,
                    index_strategy,
                    keyring_provider,
                    no_build: flag(no_build, build),
                    no_binary,
                    only_binary,
                    no_build_isolation: flag(no_build_isolation, build_isolation),
                    strict: flag(strict, no_strict),
                    extra,
                    all_extras: flag(all_extras, no_all_extras),
                    no_deps: flag(no_deps, deps),
                    resolution,
                    prerelease: if pre {
                        Some(PreReleaseMode::Allow)
                    } else {
                        prerelease
                    },
                    legacy_setup_py: flag(legacy_setup_py, no_legacy_setup_py),
                    config_settings: config_setting.map(|config_settings| {
                        config_settings.into_iter().collect::<ConfigSettings>()
                    }),
                    exclude_newer,
                    link_mode,
                    compile_bytecode: flag(compile_bytecode, no_compile_bytecode),
                    require_hashes: flag(require_hashes, no_require_hashes),
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip uninstall` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipUninstallSettings {
    // CLI-only settings.
    pub(crate) package: Vec<String>,
    pub(crate) requirement: Vec<PathBuf>,
    // Shared settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipUninstallSettings {
    /// Resolve the [`PipUninstallSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipUninstallArgs, workspace: Option<Workspace>) -> Self {
        let PipUninstallArgs {
            package,
            requirement,
            python,
            keyring_provider,
            system,
            no_system,
            break_system_packages,
            no_break_system_packages,
            offline,
            no_offline,
        } = args;

        Self {
            // CLI-only settings.
            package,
            requirement,

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    break_system_packages: flag(break_system_packages, no_break_system_packages),
                    offline: flag(offline, no_offline),
                    keyring_provider,
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip freeze` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipFreezeSettings {
    // CLI-only settings.
    pub(crate) exclude_editable: bool,
    // Shared settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipFreezeSettings {
    /// Resolve the [`PipFreezeSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipFreezeArgs, workspace: Option<Workspace>) -> Self {
        let PipFreezeArgs {
            exclude_editable,
            strict,
            no_strict,
            python,
            system,
            no_system,
        } = args;

        Self {
            // CLI-only settings.
            exclude_editable,

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    strict: flag(strict, no_strict),
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip list` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipListSettings {
    // CLI-only settings.
    pub(crate) editable: bool,
    pub(crate) exclude_editable: bool,
    pub(crate) exclude: Vec<PackageName>,
    pub(crate) format: ListFormat,

    // CLI-only settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipListSettings {
    /// Resolve the [`PipListSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipListArgs, workspace: Option<Workspace>) -> Self {
        let PipListArgs {
            editable,
            exclude_editable,
            exclude,
            format,
            strict,
            no_strict,
            python,
            system,
            no_system,
            compat_args: _,
        } = args;

        Self {
            // CLI-only settings.
            editable,
            exclude_editable,
            exclude,
            format,

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    strict: flag(strict, no_strict),
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip show` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipShowSettings {
    // CLI-only settings.
    pub(crate) package: Vec<PackageName>,

    // CLI-only settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipShowSettings {
    /// Resolve the [`PipShowSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipShowArgs, workspace: Option<Workspace>) -> Self {
        let PipShowArgs {
            package,
            strict,
            no_strict,
            python,
            system,
            no_system,
        } = args;

        Self {
            // CLI-only settings.
            package,

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    strict: flag(strict, no_strict),
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip check` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipCheckSettings {
    // CLI-only settings.

    // Shared settings.
    pub(crate) shared: PipSharedSettings,
}

impl PipCheckSettings {
    /// Resolve the [`PipCheckSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: PipCheckArgs, workspace: Option<Workspace>) -> Self {
        let PipCheckArgs {
            python,
            system,
            no_system,
        } = args;

        Self {
            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for a `pip check` invocation.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct VenvSettings {
    // CLI-only settings.
    pub(crate) seed: bool,
    pub(crate) name: PathBuf,
    pub(crate) prompt: Option<String>,
    pub(crate) system_site_packages: bool,

    // CLI-only settings.
    pub(crate) shared: PipSharedSettings,
}

impl VenvSettings {
    /// Resolve the [`VenvSettings`] from the CLI and workspace configuration.
    pub(crate) fn resolve(args: VenvArgs, workspace: Option<Workspace>) -> Self {
        let VenvArgs {
            python,
            system,
            no_system,
            seed,
            name,
            prompt,
            system_site_packages,
            link_mode,
            index_url,
            extra_index_url,
            no_index,
            index_strategy,
            keyring_provider,
            offline,
            no_offline,
            exclude_newer,
            compat_args: _,
        } = args;

        Self {
            // CLI-only settings.
            seed,
            name,
            prompt,
            system_site_packages,

            // Shared settings.
            shared: PipSharedSettings::combine(
                PipOptions {
                    python,
                    system: flag(system, no_system),
                    offline: flag(offline, no_offline),
                    index_url: index_url.and_then(Maybe::into_option),
                    extra_index_url: extra_index_url.map(|extra_index_urls| {
                        extra_index_urls
                            .into_iter()
                            .filter_map(Maybe::into_option)
                            .collect()
                    }),
                    no_index: Some(no_index),
                    index_strategy,
                    keyring_provider,
                    exclude_newer,
                    link_mode,
                    ..PipOptions::default()
                },
                workspace,
            ),
        }
    }
}

/// The resolved settings to use for an invocation of the `pip` CLI.
///
/// Represents the shared settings that are used across all `pip` commands.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub(crate) struct PipSharedSettings {
    pub(crate) python: Option<String>,
    pub(crate) system: bool,
    pub(crate) break_system_packages: bool,
    pub(crate) offline: bool,
    pub(crate) index_url: Option<IndexUrl>,
    pub(crate) extra_index_url: Vec<IndexUrl>,
    pub(crate) no_index: bool,
    pub(crate) find_links: Vec<FlatIndexLocation>,
    pub(crate) index_strategy: IndexStrategy,
    pub(crate) keyring_provider: KeyringProviderType,
    pub(crate) no_build: bool,
    pub(crate) no_binary: Vec<PackageNameSpecifier>,
    pub(crate) only_binary: Vec<PackageNameSpecifier>,
    pub(crate) no_build_isolation: bool,
    pub(crate) strict: bool,
    pub(crate) extra: Vec<ExtraName>,
    pub(crate) all_extras: bool,
    pub(crate) no_deps: bool,
    pub(crate) resolution: ResolutionMode,
    pub(crate) prerelease: PreReleaseMode,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) no_strip_extras: bool,
    pub(crate) no_annotate: bool,
    pub(crate) no_header: bool,
    pub(crate) custom_compile_command: Option<String>,
    pub(crate) generate_hashes: bool,
    pub(crate) legacy_setup_py: bool,
    pub(crate) config_setting: ConfigSettings,
    pub(crate) python_version: Option<PythonVersion>,
    pub(crate) exclude_newer: Option<ExcludeNewer>,
    pub(crate) no_emit_package: Vec<PackageName>,
    pub(crate) emit_index_url: bool,
    pub(crate) emit_find_links: bool,
    pub(crate) emit_marker_expression: bool,
    pub(crate) emit_index_annotation: bool,
    pub(crate) annotation_style: AnnotationStyle,
    pub(crate) link_mode: LinkMode,
    pub(crate) compile_bytecode: bool,
    pub(crate) require_hashes: bool,
}

impl PipSharedSettings {
    /// Resolve the [`PipSharedSettings`] from the CLI and workspace configuration.
    pub(crate) fn combine(args: PipOptions, workspace: Option<Workspace>) -> Self {
        let PipOptions {
            python,
            system,
            break_system_packages,
            offline,
            index_url,
            extra_index_url,
            no_index,
            find_links,
            index_strategy,
            keyring_provider,
            no_build,
            no_binary,
            only_binary,
            no_build_isolation,
            strict,
            extra,
            all_extras,
            no_deps,
            resolution,
            prerelease,
            output_file,
            no_strip_extras,
            no_annotate,
            no_header,
            custom_compile_command,
            generate_hashes,
            legacy_setup_py,
            config_settings,
            python_version,
            exclude_newer,
            no_emit_package,
            emit_index_url,
            emit_find_links,
            emit_marker_expression,
            emit_index_annotation,
            annotation_style,
            link_mode,
            compile_bytecode,
            require_hashes,
        } = workspace
            .and_then(|workspace| workspace.options.pip)
            .unwrap_or_default();

        Self {
            extra: args.extra.or(extra).unwrap_or_default(),
            all_extras: args.all_extras.or(all_extras).unwrap_or_default(),
            no_deps: args.no_deps.or(no_deps).unwrap_or_default(),
            resolution: args.resolution.or(resolution).unwrap_or_default(),
            prerelease: args.prerelease.or(prerelease).unwrap_or_default(),
            output_file: args.output_file.or(output_file),
            no_strip_extras: args.no_strip_extras.or(no_strip_extras).unwrap_or_default(),
            no_annotate: args.no_annotate.or(no_annotate).unwrap_or_default(),
            no_header: args.no_header.or(no_header).unwrap_or_default(),
            custom_compile_command: args.custom_compile_command.or(custom_compile_command),
            annotation_style: args
                .annotation_style
                .or(annotation_style)
                .unwrap_or_default(),
            offline: args.offline.or(offline).unwrap_or_default(),
            index_url: args.index_url.or(index_url),
            extra_index_url: args.extra_index_url.or(extra_index_url).unwrap_or_default(),
            no_index: args.no_index.or(no_index).unwrap_or_default(),
            index_strategy: args.index_strategy.or(index_strategy).unwrap_or_default(),
            keyring_provider: args
                .keyring_provider
                .or(keyring_provider)
                .unwrap_or_default(),
            find_links: args.find_links.or(find_links).unwrap_or_default(),
            generate_hashes: args.generate_hashes.or(generate_hashes).unwrap_or_default(),
            legacy_setup_py: args.legacy_setup_py.or(legacy_setup_py).unwrap_or_default(),
            no_build_isolation: args
                .no_build_isolation
                .or(no_build_isolation)
                .unwrap_or_default(),
            no_build: args.no_build.or(no_build).unwrap_or_default(),
            only_binary: args.only_binary.or(only_binary).unwrap_or_default(),
            config_setting: args.config_settings.or(config_settings).unwrap_or_default(),
            python_version: args.python_version.or(python_version),
            exclude_newer: args.exclude_newer.or(exclude_newer),
            no_emit_package: args.no_emit_package.or(no_emit_package).unwrap_or_default(),
            emit_index_url: args.emit_index_url.or(emit_index_url).unwrap_or_default(),
            emit_find_links: args.emit_find_links.or(emit_find_links).unwrap_or_default(),
            emit_marker_expression: args
                .emit_marker_expression
                .or(emit_marker_expression)
                .unwrap_or_default(),
            emit_index_annotation: args
                .emit_index_annotation
                .or(emit_index_annotation)
                .unwrap_or_default(),
            link_mode: args.link_mode.or(link_mode).unwrap_or_default(),
            require_hashes: args.require_hashes.or(require_hashes).unwrap_or_default(),
            python: args.python.or(python),
            system: args.system.or(system).unwrap_or_default(),
            break_system_packages: args
                .break_system_packages
                .or(break_system_packages)
                .unwrap_or_default(),
            no_binary: args.no_binary.or(no_binary).unwrap_or_default(),
            compile_bytecode: args
                .compile_bytecode
                .or(compile_bytecode)
                .unwrap_or_default(),
            strict: args.strict.or(strict).unwrap_or_default(),
        }
    }
}

/// Given a boolean flag pair (like `--upgrade` and `--no-upgrade`), resolve the value of the flag.
fn flag(yes: bool, no: bool) -> Option<bool> {
    match (yes, no) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
        (..) => unreachable!("Clap should make this impossible"),
    }
}
