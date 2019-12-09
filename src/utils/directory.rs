use std::{env, ffi::OsString, path::PathBuf};

use dirs::home_dir;
use semver::Version;

use crate::{
    Result, {constants, DEFAULT_DOT_DIR, EXECUTABLE_NAME, EXTRA_PACKAGES_FILENAME},
};

pub fn dot_dir(name: &str) -> Option<PathBuf> {
    home_dir().map(|p| p.join(name))
}

pub trait PycorsPaths {
    fn home_env_variable() -> Option<OsString>;

    fn config_home() -> Result<PathBuf> {
        let env_var = Self::home_env_variable();

        let config_home_from_env = if env_var.is_some() {
            let cwd = env::current_dir()?;
            env_var.clone().map(|home| cwd.join(home))
        } else {
            None
        };

        let default_dot_dir = dot_dir(&DEFAULT_DOT_DIR);

        let home = match config_home_from_env.or(default_dot_dir) {
            None => Err(anyhow::anyhow!(
                "Cannot find {}' home directory",
                EXECUTABLE_NAME
            )),
            Some(home) => Ok(home),
        }?;

        Ok(home)
    }

    fn cache() -> Result<PathBuf> {
        Ok(Self::config_home()?.join("cache"))
    }

    fn downloaded() -> Result<PathBuf> {
        Ok(Self::cache()?.join("downloaded"))
    }

    fn extracted() -> Result<PathBuf> {
        Ok(Self::cache()?.join("extracted"))
    }

    fn installed() -> Result<PathBuf> {
        Ok(Self::config_home()?.join("installed"))
    }

    fn shims() -> Result<PathBuf> {
        Ok(Self::config_home()?.join("shims"))
    }

    fn logs() -> Result<PathBuf> {
        Ok(Self::config_home()?.join("logs"))
    }

    fn install_dir(version: &Version) -> Result<PathBuf> {
        Ok(Self::installed()?.join(format!("{}", version)))
    }

    fn default_extra_package_file() -> Result<PathBuf> {
        Ok(Self::config_home()?.join(EXTRA_PACKAGES_FILENAME))
    }

    #[cfg(not(windows))]
    fn bin_dir(version: &Version) -> Result<PathBuf> {
        Ok(Self::install_dir(version)?.join("bin"))
    }
    #[cfg(windows)]
    fn bin_dir(version: &Version) -> Result<PathBuf> {
        Ok(Self::install_dir(version)?)
    }
}

pub struct PycorsPathsFromEnv;

impl PycorsPaths for PycorsPathsFromEnv {
    fn home_env_variable() -> Option<OsString> {
        env::var_os(constants::home_env_variable())
    }
}

pub mod shell {
    pub mod bash {
        pub mod config {
            use std::path::{Path, PathBuf};

            use crate::{utils::directory::PycorsPaths, Result};

            pub fn dir_relative() -> PathBuf {
                Path::new("shell").join("bash")
            }

            pub fn dir_absolute<P>() -> Result<PathBuf>
            where
                P: PycorsPaths,
            {
                Ok(P::config_home()?.join(dir_relative()))
            }

            pub fn file_name() -> &'static str {
                "config.sh"
            }

            pub fn file_absolute<P>() -> Result<PathBuf>
            where
                P: PycorsPaths,
            {
                Ok(dir_absolute::<P>()?.join(file_name()))
            }

            pub fn autocomplete<P>() -> Result<PathBuf>
            where
                P: PycorsPaths,
            {
                Ok(dir_absolute::<P>()?.join("completion.sh"))
            }
        }
    }
}
