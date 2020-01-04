use std::{env, ffi::OsString, path::PathBuf};

use dirs::home_dir;
use semver::Version;

use crate::constants::{
    self, AVAILABLE_TOOLCHAIN_CACHE, DEFAULT_DOT_DIR, EXECUTABLE_NAME,
    EXTRA_PACKAGES_FILENAME,
};

fn dot_dir(name: &str) -> Option<PathBuf> {
    home_dir().map(|p| p.join(name))
}
#[cfg_attr(test, mockall::automock)]
pub trait PycorsHomeProviderTrait {
    fn home_env_variable(&self) -> Option<OsString>;
    fn paths(&self) -> Option<OsString>;
}

pub struct PycorsPathsProvider<P>
where
    P: PycorsHomeProviderTrait,
{
    path_provider: P,
}

impl<P> PycorsHomeProviderTrait for PycorsPathsProvider<P>
where
    P: PycorsHomeProviderTrait,
{
    fn home_env_variable(&self) -> Option<OsString> {
        self.path_provider.home_env_variable()
    }
    fn paths(&self) -> Option<OsString> {
        self.path_provider.paths()
    }
}

pub struct PycorsPathsProviderFromEnv;

impl PycorsPathsProviderFromEnv {
    pub fn new() -> PycorsPathsProvider<PycorsPathsProviderFromEnv> {
        PycorsPathsProvider {
            path_provider: PycorsPathsProviderFromEnv {},
        }
    }
}

impl PycorsHomeProviderTrait for PycorsPathsProviderFromEnv {
    fn home_env_variable(&self) -> Option<OsString> {
        env::var_os(constants::home_env_variable())
    }
    fn paths(&self) -> Option<OsString> {
        env::var_os("PATH")
    }
}

impl<P> PycorsPathsProvider<P>
where
    P: PycorsHomeProviderTrait,
{
    #[cfg(test)]
    pub fn from(path_provider: P) -> Self {
        PycorsPathsProvider { path_provider }
    }

    pub fn config_home(&self) -> PathBuf {
        let config_home_from_env = self.path_provider.home_env_variable().map(PathBuf::from);

        let default_dot_dir = dot_dir(&DEFAULT_DOT_DIR);

        // If we can't find our home directory, there is nothing we can do; simply panic.
        config_home_from_env
            .or(default_dot_dir)
            .ok_or_else(|| anyhow::anyhow!("Cannot find {}' home directory", EXECUTABLE_NAME))
            .unwrap()
    }

    pub fn default_extra_package_file(&self) -> PathBuf {
        self.config_home().join(EXTRA_PACKAGES_FILENAME)
    }

    pub fn cache(&self) -> PathBuf {
        self.config_home().join("cache")
    }

    pub fn installed(&self) -> PathBuf {
        self.config_home().join("installed")
    }

    pub fn logs(&self) -> PathBuf {
        self.config_home().join("logs")
    }

    pub fn shims(&self) -> PathBuf {
        self.config_home().join("shims")
    }

    pub fn downloaded(&self) -> PathBuf {
        self.cache().join("downloaded")
    }

    pub fn available_toolchains_cache_file(&self) -> PathBuf {
        self.cache().join(AVAILABLE_TOOLCHAIN_CACHE)
    }

    pub fn extracted(&self) -> PathBuf {
        self.cache().join("extracted")
    }

    pub fn install_dir(&self, version: &Version) -> PathBuf {
        self.installed().join(format!("{}", version))
    }

    #[cfg(not(windows))]
    pub fn bin_dir(&self, version: &Version) -> PathBuf {
        self.install_dir(version).join("bin")
    }
    #[cfg(windows)]
    pub fn bin_dir(&self, version: &Version) -> PathBuf {
        self.install_dir(version)
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
pub mod tests {
    use std::path::Path;

    use super::*;

    fn temp_dir() -> PathBuf {
        env::temp_dir()
            .join(EXECUTABLE_NAME)
            .join("directory")
            .join("tests")
    }

    #[test]
    fn pycors_paths_from_env() {
        // Playing an env variables is subject to race conditions
        // since tests are run in parallel. Simply call the constructor
        // and the function.
        let paths_provider: PycorsPathsProvider<PycorsPathsProviderFromEnv> =
            PycorsPathsProviderFromEnv::new();
        let _ = paths_provider.home_env_variable();
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
        fn path_provider_env() {
            // FIXME: Detect if PYCORS_HOME is set, use it for 'expected' if set.
            let expected = dot_dir(DEFAULT_DOT_DIR).unwrap();
            let paths_provider = PycorsPathsProviderFromEnv::new();
            let to_validate = paths_provider.config_home();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn config_home_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home;

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.config_home();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn config_home_from_env_variable() {
            let pycors_home = temp_dir().join("config_home_from_env_variable");

            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home;

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.config_home();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn default_extra_package_file_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join(EXTRA_PACKAGES_FILENAME);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.default_extra_package_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn default_extra_package_file_from_env_variable() {
            let pycors_home = temp_dir().join("default_extra_package_file_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join(EXTRA_PACKAGES_FILENAME);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.default_extra_package_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn cache_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("cache");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.cache();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn cache_from_env_variable() {
            let pycors_home = temp_dir().join("cache_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join("cache");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.cache();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn installed_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("installed");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.installed();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn installed_from_env_variable() {
            let pycors_home = temp_dir().join("installed_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join("installed");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.installed();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn logs_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("logs");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.logs();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn logs_from_env_variable() {
            let pycors_home = temp_dir().join("logs_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join("logs");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.logs();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn shims_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("shims");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.shims();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn shims_from_env_variable() {
            let pycors_home = temp_dir().join("shims_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join("shims");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.shims();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn downloaded_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("cache").join("downloaded");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.downloaded();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn downloaded_from_env_variable() {
            let pycors_home = temp_dir().join("downloaded_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join("cache").join("downloaded");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.downloaded();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn available_toolchain_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("cache").join(AVAILABLE_TOOLCHAIN_CACHE);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.available_toolchains_cache_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn available_toolchain_from_env_variable() {
            let pycors_home = temp_dir().join("available_toolchain_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join("cache").join(AVAILABLE_TOOLCHAIN_CACHE);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.available_toolchains_cache_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn extracted_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("cache").join("extracted");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.extracted();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn extracted_from_env_variable() {
            let pycors_home = temp_dir().join("extracted_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

            let expected = pycors_home.join("cache").join("extracted");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.extracted();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn install_dir_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();
            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            let mocked_pycors_home = None;

            let expected = pycors_home.join("installed").join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.install_dir(&version);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn install_dir_from_env_variable() {
            let pycors_home = temp_dir().join("install_dir_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());
            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            let expected = pycors_home.join("installed").join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.install_dir(&version);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn bin_dir_from_default() {
            let pycors_home = dot_dir(DEFAULT_DOT_DIR).unwrap();
            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            let mocked_pycors_home = None;

            #[cfg(not(windows))]
            let expected = pycors_home.join("installed").join(version_str).join("bin");
            #[cfg(windows)]
            let expected = pycors_home.join("installed").join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.bin_dir(&version);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn bin_dir_from_env_variable() {
            let pycors_home = temp_dir().join("bin_dir_from_env");
            let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());
            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            #[cfg(not(windows))]
            let expected = pycors_home.join("installed").join(version_str).join("bin");
            #[cfg(windows)]
            let expected = pycors_home.join("installed").join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_home_env_variable()
                .times(1)
                .return_const(mocked_pycors_home);

            let paths_provider = PycorsPathsProvider::from(mock);
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
