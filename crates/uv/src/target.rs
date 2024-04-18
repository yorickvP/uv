use pep508_rs::MarkerEnvironment;
use platform_tags::{Arch, Os, Platform};

/// The supported target triples. Each triple consists of an architecture, vendor, and operating
/// system.
///
/// See: <https://doc.rust-lang.org/nightly/rustc/platform-support.html>
#[derive(Debug, Clone, Copy, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum TargetTriple {
    #[value(name = "x86_64-pc-windows-msvc")]
    X8664PcWindowsMsvc,
    #[value(name = "x86_64-unknown-linux-gnu")]
    X8664UnknownLinuxGnu,
    #[value(name = "x86_64-apple-darwin")]
    X8664AppleDarwin,
    #[value(name = "aarch64-apple-darwin")]
    Aarch64AppleDarwin,
    #[value(name = "aarch64-unknown-linux-gnu")]
    Aarch64UnknownLinuxGnu,
    #[value(name = "aarch64-unknown-linux-musl")]
    Aarch64UnknownLinuxMusl,
    #[value(name = "x86_64-unknown-linux-musl")]
    X8664UnknownLinuxMusl,
}

impl TargetTriple {
    /// Return the [`Platform`] for the target.
    pub(crate) fn platform(self) -> Platform {
        match self {
            Self::X8664PcWindowsMsvc => Platform::new(Os::Windows, Arch::X86_64),
            Self::X8664UnknownLinuxGnu => Platform::new(
                Os::Manylinux {
                    major: 2,
                    minor: 17,
                },
                Arch::X86_64,
            ),
            Self::X8664AppleDarwin => Platform::new(
                Os::Macos {
                    major: 10,
                    minor: 12,
                },
                Arch::X86_64,
            ),
            Self::Aarch64AppleDarwin => Platform::new(
                Os::Macos {
                    major: 11,
                    minor: 0,
                },
                Arch::Aarch64,
            ),
            Self::Aarch64UnknownLinuxGnu => Platform::new(
                Os::Manylinux {
                    major: 2,
                    minor: 17,
                },
                Arch::Aarch64,
            ),
            Self::Aarch64UnknownLinuxMusl => {
                Platform::new(Os::Musllinux { major: 1, minor: 2 }, Arch::Aarch64)
            }
            Self::X8664UnknownLinuxMusl => {
                Platform::new(Os::Musllinux { major: 1, minor: 2 }, Arch::X86_64)
            }
        }
    }

    /// Return the `platform_machine` value for the target.
    pub(crate) fn platform_machine(self) -> &'static str {
        match self {
            Self::X8664PcWindowsMsvc => "x86_64",
            Self::X8664UnknownLinuxGnu => "x86_64",
            Self::X8664AppleDarwin => "x86_64",
            Self::Aarch64AppleDarwin => "aarch64",
            Self::Aarch64UnknownLinuxGnu => "aarch64",
            Self::Aarch64UnknownLinuxMusl => "aarch64",
            Self::X8664UnknownLinuxMusl => "x86_64",
        }
    }

    /// Return the `platform_system` value for the target.
    pub(crate) fn platform_system(self) -> &'static str {
        match self {
            Self::X8664PcWindowsMsvc => "Windows",
            Self::X8664UnknownLinuxGnu => "Linux",
            Self::X8664AppleDarwin => "Darwin",
            Self::Aarch64AppleDarwin => "Darwin",
            Self::Aarch64UnknownLinuxGnu => "Linux",
            Self::Aarch64UnknownLinuxMusl => "Linux",
            Self::X8664UnknownLinuxMusl => "Linux",
        }
    }

    /// Return the `platform_version` value for the target.
    pub(crate) fn platform_version(self) -> &'static str {
        match self {
            Self::X8664PcWindowsMsvc => "",
            Self::X8664UnknownLinuxGnu => "",
            Self::X8664AppleDarwin => "",
            Self::Aarch64AppleDarwin => "",
            Self::Aarch64UnknownLinuxGnu => "",
            Self::Aarch64UnknownLinuxMusl => "",
            Self::X8664UnknownLinuxMusl => "",
        }
    }

    /// Return the `platform_release` value for the target.
    pub(crate) fn platform_release(self) -> &'static str {
        match self {
            Self::X8664PcWindowsMsvc => "",
            Self::X8664UnknownLinuxGnu => "",
            Self::X8664AppleDarwin => "",
            Self::Aarch64AppleDarwin => "",
            Self::Aarch64UnknownLinuxGnu => "",
            Self::Aarch64UnknownLinuxMusl => "",
            Self::X8664UnknownLinuxMusl => "",
        }
    }

    /// Return the `os_name` value for the target.
    pub(crate) fn os_name(self) -> &'static str {
        match self {
            Self::X8664PcWindowsMsvc => "nt",
            Self::X8664UnknownLinuxGnu => "posix",
            Self::X8664AppleDarwin => "posix",
            Self::Aarch64AppleDarwin => "posix",
            Self::Aarch64UnknownLinuxGnu => "posix",
            Self::Aarch64UnknownLinuxMusl => "posix",
            Self::X8664UnknownLinuxMusl => "posix",
        }
    }

    /// Return the `sys_platform` value for the target.
    pub(crate) fn sys_platform(self) -> &'static str {
        match self {
            Self::X8664PcWindowsMsvc => "win32",
            Self::X8664UnknownLinuxGnu => "linux",
            Self::X8664AppleDarwin => "darwin",
            Self::Aarch64AppleDarwin => "darwin",
            Self::Aarch64UnknownLinuxGnu => "linux",
            Self::Aarch64UnknownLinuxMusl => "linux",
            Self::X8664UnknownLinuxMusl => "linux",
        }
    }

    /// Return a [`MarkerEnvironment`] compatible with the given [`TargetTriple`], based on
    /// a base [`MarkerEnvironment`].
    ///
    /// The returned [`MarkerEnvironment`] will preserve the base environment's Python version
    /// markers, but override its platform markers.
    pub(crate) fn markers(self, base: &MarkerEnvironment) -> MarkerEnvironment {
        MarkerEnvironment {
            // Platform markers
            os_name: self.os_name().to_string(),
            platform_machine: self.platform_machine().to_string(),
            platform_system: self.platform_system().to_string(),
            sys_platform: self.sys_platform().to_string(),
            platform_release: self.platform_release().to_string(),
            platform_version: self.platform_version().to_string(),
            // Python version markers
            implementation_name: base.implementation_name.clone(),
            implementation_version: base.implementation_version.clone(),
            platform_python_implementation: base.platform_python_implementation.clone(),
            python_full_version: base.python_full_version.clone(),
            python_version: base.python_version.clone(),
        }
    }
}
