use crate::config::Platform;
use std::path::PathBuf;

xflags::xflags! {
    src "./src/flags.rs"

    cmd task {
        cmd dist {
            optional --config config_path: PathBuf
            optional --release
            optional -p, --platform platform: Platform
            optional --kernel_features kernel_features: String
        }

        cmd qemu {
            // XXX: shared with dist command. Should be the same.
            optional --config config_path: PathBuf
            optional --release
            optional -p,--platform platform: Platform
            optional --kernel_features kernel_features: String

            optional --display
            optional --debug_int_firehose
            optional --debug_mmu_firehose
            optional --debug_cpu_firehose
        }

        cmd boot {
            // XXX: shared with dist command. Should be the same.
            optional --config config_path: PathBuf
            optional --release
            optional -p,--platform platform: Platform
            optional --kernel_features kernel_features: String
        }

        cmd opensbi {
            optional -p, --platform platform: Platform
        }

        cmd devicetree {
            required path: PathBuf
        }

        cmd clean {}
    }
}

pub struct DistOptions {
    pub config_path: PathBuf,
    pub platform: Option<Platform>,
    pub release: bool,
    pub kernel_features: Option<String>,
}

impl From<&Dist> for DistOptions {
    fn from(flags: &Dist) -> DistOptions {
        DistOptions {
            config_path: flags.config.clone().unwrap_or(PathBuf::from("Poplar.toml")),
            release: flags.release,
            kernel_features: flags.kernel_features.clone(),
            platform: flags.platform,
        }
    }
}

impl From<&Boot> for DistOptions {
    fn from(flags: &Boot) -> DistOptions {
        DistOptions {
            config_path: flags.config.clone().unwrap_or(PathBuf::from("Poplar.toml")),
            release: flags.release,
            kernel_features: flags.kernel_features.clone(),
            platform: flags.platform,
        }
    }
}

impl From<&Qemu> for DistOptions {
    fn from(flags: &Qemu) -> DistOptions {
        DistOptions {
            config_path: flags.config.clone().unwrap_or(PathBuf::from("Poplar.toml")),
            release: flags.release,
            kernel_features: flags.kernel_features.clone(),
            platform: flags.platform,
        }
    }
}

// generated start
// The following code is generated by `xflags` macro.
// Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
#[derive(Debug)]
pub struct Task {
    pub subcommand: TaskCmd,
}

#[derive(Debug)]
pub enum TaskCmd {
    Dist(Dist),
    Qemu(Qemu),
    Boot(Boot),
    Opensbi(Opensbi),
    Devicetree(Devicetree),
    Clean(Clean),
}

#[derive(Debug)]
pub struct Dist {
    pub config: Option<PathBuf>,
    pub release: bool,
    pub platform: Option<Platform>,
    pub kernel_features: Option<String>,
}

#[derive(Debug)]
pub struct Qemu {
    pub config: Option<PathBuf>,
    pub release: bool,
    pub platform: Option<Platform>,
    pub kernel_features: Option<String>,
    pub display: bool,
    pub debug_int_firehose: bool,
    pub debug_mmu_firehose: bool,
    pub debug_cpu_firehose: bool,
}

#[derive(Debug)]
pub struct Boot {
    pub config: Option<PathBuf>,
    pub release: bool,
    pub platform: Option<Platform>,
    pub kernel_features: Option<String>,
}

#[derive(Debug)]
pub struct Opensbi {
    pub platform: Option<Platform>,
}

#[derive(Debug)]
pub struct Devicetree {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct Clean;

impl Task {
    #[allow(dead_code)]
    pub fn from_env_or_exit() -> Self {
        Self::from_env_or_exit_()
    }

    #[allow(dead_code)]
    pub fn from_env() -> xflags::Result<Self> {
        Self::from_env_()
    }

    #[allow(dead_code)]
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
        Self::from_vec_(args)
    }
}
// generated end
