use super::*;

pub fn dot_dir(name: &str) -> Option<PathBuf> {
    home_dir().map(|p| p.join(name))
}

pub fn config_home() -> Result<PathBuf> {
    let env_var = env::var_os(home_env_variable());

    let config_home_from_env = if env_var.is_some() {
        let cwd = env::current_dir()?;
        env_var.clone().map(|home| cwd.join(home))
    } else {
        None
    };

    let default_dot_dir = dot_dir(&DEFAULT_DOT_DIR);

    let home = match config_home_from_env.or(default_dot_dir) {
        None => Err(anyhow!("Cannot find {}' home directory", EXECUTABLE_NAME)),
        Some(home) => Ok(home),
    }?;

    Ok(home)
}

pub fn cache() -> Result<PathBuf> {
    Ok(config_home()?.join("cache"))
}

pub fn downloaded() -> Result<PathBuf> {
    Ok(cache()?.join("downloaded"))
}

pub fn extracted() -> Result<PathBuf> {
    Ok(cache()?.join("extracted"))
}

pub fn installed() -> Result<PathBuf> {
    Ok(config_home()?.join("installed"))
}

pub fn shims() -> Result<PathBuf> {
    Ok(config_home()?.join("shims"))
}

pub fn logs() -> Result<PathBuf> {
    Ok(config_home()?.join("logs"))
}

pub fn install_dir(version: &Version) -> Result<PathBuf> {
    Ok(installed()?.join(format!("{}", version)))
}

#[cfg(not(windows))]
pub fn bin_dir(version: &Version) -> Result<PathBuf> {
    Ok(install_dir(version)?.join("bin"))
}
#[cfg(windows)]
pub fn bin_dir(version: &Version) -> Result<PathBuf> {
    Ok(install_dir(version)?)
}

pub mod shell {
    pub mod bash {
        pub mod config {
            use std::path::{Path, PathBuf};

            use super::super::super::config_home;

            use crate::Result;

            pub fn dir_relative() -> PathBuf {
                Path::new("shell").join("bash")
            }

            pub fn dir_absolute() -> Result<PathBuf> {
                Ok(config_home()?.join(dir_relative()))
            }

            pub fn file_name() -> &'static str {
                "config.sh"
            }

            pub fn file_absolute() -> Result<PathBuf> {
                Ok(dir_absolute()?.join(file_name()))
            }

            pub fn autocomplete() -> Result<PathBuf> {
                Ok(dir_absolute()?.join("completion.sh"))
            }
        }
    }
}
