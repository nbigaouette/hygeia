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

    fn default_extra_package_file(&self) -> PathBuf {
        self.config_home().join(EXTRA_PACKAGES_FILENAME)
    }

    fn cache(&self) -> PathBuf {
        self.config_home().join("cache")
    }

    fn installed(&self) -> PathBuf {
        self.config_home().join("installed")
    }

    fn logs(&self) -> PathBuf {
        self.config_home().join("logs")
    }

    fn shims(&self) -> PathBuf {
        self.config_home().join("shims")
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

    fn install_dir(&self, version: &Version) -> PathBuf {
        self.installed().join(format!("{}", version))
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
    use std::path::Path;

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
    fn pycors_paths_from_env() {
        // Playing an env variables is subject to race conditions
        // since tests are run in parallel. Simply call the constructor
        // and the function.
        let _ = PycorsPathsFromEnv::new().home_env_variable();
    }

    #[test]
    fn bash_dir_relative() {
        assert_eq!(
            shell::bash::config::dir_relative(),
            Path::new("shell").join("bash")
        );
    }

    #[test]
    fn bash_file_path() {
        assert_eq!(
            shell::bash::config::file_path(),
            Path::new("shell").join("bash").join("config.sh")
        );
    }

    #[test]
    fn bash_autocomplete() {
        assert_eq!(
            shell::bash::config::autocomplete(),
            Path::new("shell").join("bash").join("completion.sh")
        );
    }

    mod pycors_paths_trait {
        use super::*;

        #[test]
        fn home_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.config_home();
            let expected = dot_dir(DEFAULT_DOT_DIR).unwrap();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn home_from_env_variable() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir;
            let to_validate = paths_provider.config_home();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn default_extra_package_file_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.default_extra_package_file();
            let expected = dot_dir(DEFAULT_DOT_DIR)
                .unwrap()
                .join(EXTRA_PACKAGES_FILENAME);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn default_extra_package_file_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join(EXTRA_PACKAGES_FILENAME);
            let to_validate = paths_provider.default_extra_package_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn cache_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.cache();
            let expected = dot_dir(DEFAULT_DOT_DIR).unwrap().join("cache");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn cache_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join("cache");
            let to_validate = paths_provider.cache();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn installed_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.installed();
            let expected = dot_dir(DEFAULT_DOT_DIR).unwrap().join("installed");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn installed_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let to_validate = paths_provider.installed();
            let expected = tmp_dir.join("installed");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn logs_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let expected = dot_dir(DEFAULT_DOT_DIR).unwrap().join("logs");
            let to_validate = paths_provider.logs();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn logs_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join("logs");
            let to_validate = paths_provider.logs();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn shims_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.shims();
            let expected = dot_dir(DEFAULT_DOT_DIR).unwrap().join("shims");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn shims_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join("shims");
            let to_validate = paths_provider.shims();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn downloaded_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.downloaded();
            let expected = dot_dir(DEFAULT_DOT_DIR)
                .unwrap()
                .join("cache")
                .join("downloaded");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn downloaded_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join("cache").join("downloaded");
            let to_validate = paths_provider.downloaded();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn available_toolchain_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.available_toolchains_cache_file();
            let expected = dot_dir(DEFAULT_DOT_DIR)
                .unwrap()
                .join("cache")
                .join(AVAILABLE_TOOLCHAIN_CACHE);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn available_toolchain_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join("cache").join(AVAILABLE_TOOLCHAIN_CACHE);
            let to_validate = paths_provider.available_toolchains_cache_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn extracted_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let to_validate = paths_provider.extracted();
            let expected = dot_dir(DEFAULT_DOT_DIR)
                .unwrap()
                .join("cache")
                .join("extracted");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn extracted_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join("cache").join("extracted");
            let to_validate = paths_provider.extracted();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn install_dir_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let version = Version::parse("3.7.5").unwrap();
            let to_validate = paths_provider.install_dir(&version);
            let expected = dot_dir(DEFAULT_DOT_DIR)
                .unwrap()
                .join("installed")
                .join("3.7.5");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn install_dir_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let version = Version::parse("3.7.5").unwrap();

            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            let expected = tmp_dir.join("installed").join("3.7.5");

            let to_validate = paths_provider.install_dir(&version);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn bin_dir_default() {
            let paths_provider = PycorsPathsFromFakeEnv::new();
            let version = Version::parse("3.7.5").unwrap();
            let to_validate = paths_provider.bin_dir(&version);
            #[cfg(not(windows))]
            let expected = dot_dir(DEFAULT_DOT_DIR)
                .unwrap()
                .join("installed")
                .join("3.7.5")
                .join("bin");
            #[cfg(windows)]
            let expected = dot_dir(DEFAULT_DOT_DIR)
                .unwrap()
                .join("installed")
                .join("3.7.5");
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn bin_dir_from_env() {
            let mut paths_provider = PycorsPathsFromFakeEnv::new();
            let version = Version::parse("3.7.5").unwrap();

            let tmp_dir = env::temp_dir();
            paths_provider.value = Some(tmp_dir.clone().into_os_string());
            #[cfg(not(windows))]
            let expected = tmp_dir.join("installed").join("3.7.5").join("bin");
            #[cfg(windows)]
            let expected = tmp_dir.join("installed").join("3.7.5");

            let to_validate = paths_provider.bin_dir(&version);
            assert_eq!(to_validate, expected);
        }
    }

    #[test]
    fn dot_dir_success() {
        let dir = dot_dir(".dummy").unwrap();
        let expected = home_dir().unwrap().join(".dummy");
        assert_eq!(dir, expected);
    }
}
