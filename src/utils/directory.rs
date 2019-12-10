use std::{env, ffi::OsString, path::PathBuf};

use dirs::home_dir;
use semver::Version;

use crate::{
    constants, AVAILABLE_TOOLCHAIN_CACHE, DEFAULT_DOT_DIR, EXECUTABLE_NAME, EXTRA_PACKAGES_FILENAME,
};

fn dot_dir(name: &str) -> Option<PathBuf> {
    home_dir().map(|p| p.join(name))
}

pub trait PycorsPaths {
    fn new() -> Self
    where
        Self: Sized;

    fn home_env_variable(&self) -> Option<OsString>;

    fn config_home(&self) -> PathBuf {
        let config_home_from_env = self.home_env_variable().map(PathBuf::from);

        let default_dot_dir = dot_dir(&DEFAULT_DOT_DIR);

        // If we can't find our home directory, there is nothing we can do; simply panic.
        config_home_from_env
            .or(default_dot_dir)
            .ok_or_else(|| anyhow::anyhow!("Cannot find {}' home directory", EXECUTABLE_NAME))
            .unwrap()
    }

    fn cache(&self) -> PathBuf {
        self.config_home().join("cache")
    }

    fn downloaded(&self) -> PathBuf {
        self.cache().join("downloaded")
    }

    fn available_toolchains_cache_file(&self) -> PathBuf {
        self.cache().join(AVAILABLE_TOOLCHAIN_CACHE)
    }

    fn extracted(&self) -> PathBuf {
        self.cache().join("extracted")
    }

    fn installed(&self) -> PathBuf {
        self.config_home().join("installed")
    }

    fn shims(&self) -> PathBuf {
        self.config_home().join("shims")
    }

    fn logs(&self) -> PathBuf {
        self.config_home().join("logs")
    }

    fn install_dir(&self, version: &Version) -> PathBuf {
        self.installed().join(format!("{}", version))
    }

    fn default_extra_package_file(&self) -> PathBuf {
        self.config_home().join(EXTRA_PACKAGES_FILENAME)
    }

    #[cfg(not(windows))]
    fn bin_dir(&self, version: &Version) -> PathBuf {
        self.install_dir(version).join("bin")
    }
    #[cfg(windows)]
    fn bin_dir(&self, version: &Version) -> PathBuf {
        self.install_dir(version)
    }
}

pub struct PycorsPathsFromEnv;

impl PycorsPaths for PycorsPathsFromEnv {
    fn new() -> Self
    where
        Self: Sized,
    {
        PycorsPathsFromEnv {}
    }

    fn home_env_variable(&self) -> Option<OsString> {
        env::var_os(constants::home_env_variable())
    }
}

pub mod shell {
    pub mod bash {
        pub mod config {
            use std::path::{Path, PathBuf};

            pub fn dir_relative() -> PathBuf {
                Path::new("shell").join("bash")
            }

            pub fn file_path() -> PathBuf {
                dir_relative().join("config.sh")
            }

            pub fn autocomplete() -> PathBuf {
                dir_relative().join("completion.sh")
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    pub struct PycorsPathsFromFakeEnv {
        pub value: Option<OsString>,
    }

    impl PycorsPaths for PycorsPathsFromFakeEnv {
        fn new() -> Self
        where
            Self: Sized,
        {
            PycorsPathsFromFakeEnv { value: None }
        }

        fn home_env_variable(&self) -> Option<OsString> {
            self.value.clone()
        }
    }

    #[test]
    fn home_default() {
        let paths_provider = PycorsPathsFromFakeEnv::new();
        let default_home = paths_provider.config_home();
        let expected = home_dir().unwrap().join(DEFAULT_DOT_DIR);
        assert_eq!(default_home, expected);
    }

    #[test]
    fn home_from_env_variable() {
        let mut paths_provider = PycorsPathsFromFakeEnv::new();
        let tmp_dir = env::temp_dir();
        paths_provider.value = Some(tmp_dir.clone().into_os_string());
        assert_eq!(paths_provider.config_home(), tmp_dir);
    }

    // #[test]
    // fn cache_

    #[test]
    fn dot_dir_success() {
        let mut paths_provider = PycorsPathsFromFakeEnv::new();
        let dir = dot_dir(".dummy").unwrap();
        let expected = home_dir().unwrap().join(".dummy");
        assert_eq!(dir, expected);
    }

    #[test]
    fn directories() {
        let mut paths_provider = PycorsPathsFromFakeEnv::new();
        let dir = paths_provider.cache();
        let expected = home_dir().unwrap().join(DEFAULT_DOT_DIR).join("cache");
        assert_eq!(dir, expected);

        let dir = paths_provider.downloaded();
        let expected = home_dir()
            .unwrap()
            .join(DEFAULT_DOT_DIR)
            .join("cache")
            .join("downloaded");
        assert_eq!(dir, expected);

        let dir = paths_provider.extracted();
        let expected = home_dir()
            .unwrap()
            .join(DEFAULT_DOT_DIR)
            .join("cache")
            .join("extracted");
        assert_eq!(dir, expected);

        let dir = paths_provider.installed();
        let expected = home_dir().unwrap().join(DEFAULT_DOT_DIR).join("installed");
        assert_eq!(dir, expected);
    }

    #[test]
    fn install_dir_version() {
        let mut paths_provider = PycorsPathsFromFakeEnv::new();
        let version = Version::parse("3.7.2").unwrap();
        let dir = paths_provider.install_dir(&version);
        let expected = home_dir()
            .unwrap()
            .join(DEFAULT_DOT_DIR)
            .join("installed")
            .join("3.7.2");
        assert_eq!(dir, expected);
    }
}
