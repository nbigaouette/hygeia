use semver::Version;
use std::{env, path::PathBuf};

use crate::constants::{
    self, AVAILABLE_TOOLCHAIN_CACHE, DEFAULT_DOT_DIR, EXECUTABLE_NAME, EXTRA_PACKAGES_FILENAME,
    SHIMS_DIRECTORY_IDENTIFIER_FILE,
};

#[cfg_attr(test, mockall::automock)]
pub trait PycorsHomeProviderTrait {
    fn home(&self) -> Option<PathBuf>;
    fn document(&self) -> Option<PathBuf>;
    fn project_home(&self) -> Option<PathBuf>;
    fn paths(&self) -> Vec<PathBuf>;
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
    fn home(&self) -> Option<PathBuf> {
        self.path_provider.home()
    }
    fn document(&self) -> Option<PathBuf> {
        self.path_provider.document()
    }
    fn project_home(&self) -> Option<PathBuf> {
        self.path_provider.project_home()
    }
    fn paths(&self) -> Vec<PathBuf> {
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
    fn home(&self) -> Option<PathBuf> {
        match env::var_os(constants::home_overwrite_env_variable()) {
            Some(home) => Some(PathBuf::from(home)),
            None => dirs_next::home_dir(),
        }
    }
    fn document(&self) -> Option<PathBuf> {
        match env::var_os(constants::document_overwrite_env_variable()) {
            Some(document) => Some(PathBuf::from(document)),
            None => dirs_next::document_dir(),
        }
    }

    fn project_home(&self) -> Option<PathBuf> {
        env::var_os(constants::project_home_env_variable()).map(PathBuf::from)
    }
    fn paths(&self) -> Vec<PathBuf> {
        match env::var_os("PATH") {
            Some(p) => env::split_paths(&p).collect(),
            None => Vec::new(),
        }
    }
}

impl<P> PycorsPathsProvider<P>
where
    P: PycorsHomeProviderTrait,
{
    pub fn from(path_provider: P) -> Self {
        PycorsPathsProvider { path_provider }
    }

    pub fn project_home(&self) -> PathBuf {
        let config_home_from_env = self.path_provider.project_home();

        match config_home_from_env {
            Some(config_home_from_env) => config_home_from_env,
            None => {
                self.home()
                    .map(|p| p.join(DEFAULT_DOT_DIR))
                    .unwrap_or_else(|| {
                        // If we can't find our home directory, there is nothing we can do; simply panic.
                        panic!("Cannot find {}'s home directory", EXECUTABLE_NAME)
                    })
            }
        }
    }

    pub fn default_extra_package_file(&self) -> PathBuf {
        self.project_home().join(EXTRA_PACKAGES_FILENAME)
    }

    pub fn cache(&self) -> PathBuf {
        self.project_home().join("cache")
    }

    pub fn installed(&self) -> PathBuf {
        self.project_home().join("installed").join("cpython")
    }

    pub fn logs(&self) -> PathBuf {
        self.project_home().join("logs")
    }

    pub fn shims(&self) -> PathBuf {
        self.project_home().join("shims")
    }

    pub fn downloaded(&self) -> PathBuf {
        self.cache().join("downloaded")
    }

    pub fn available_toolchains_cache_file(&self) -> PathBuf {
        self.cache().join(AVAILABLE_TOOLCHAIN_CACHE)
    }

    pub fn shims_directory_identifier_file(&self) -> PathBuf {
        self.shims().join(SHIMS_DIRECTORY_IDENTIFIER_FILE)
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

    use std::path::{Path, PathBuf};

    pub trait ShellPathProvider {
        fn new() -> Self;
        fn dir_relative(&self) -> PathBuf;
        fn file_path(&self) -> PathBuf;
        fn autocomplete(&self) -> PathBuf;
        fn shell_type(&self) -> structopt::clap::Shell;
        fn shell_rcs(&self) -> &'static [&'static str];
    }
    pub struct Bash;
    pub struct Zsh;
    pub struct Powershell;
    pub struct Fish;

    impl ShellPathProvider for Bash {
        fn new() -> Self {
            Bash {}
        }
        fn dir_relative(&self) -> PathBuf {
            Path::new("shell").join("bash")
        }
        fn file_path(&self) -> PathBuf {
            self.dir_relative().join("config.sh")
        }
        fn autocomplete(&self) -> PathBuf {
            self.dir_relative().join("completion.sh")
        }
        fn shell_type(&self) -> structopt::clap::Shell {
            structopt::clap::Shell::Bash
        }
        fn shell_rcs(&self) -> &'static [&'static str] {
            &[".bashrc", ".bash_profile"]
        }
    }

    impl ShellPathProvider for Zsh {
        fn new() -> Self {
            Zsh {}
        }
        fn dir_relative(&self) -> PathBuf {
            Path::new("shell").join("zsh")
        }
        fn file_path(&self) -> PathBuf {
            self.dir_relative().join("config.sh")
        }
        fn autocomplete(&self) -> PathBuf {
            self.dir_relative()
                .join(format!("_{}", crate::constants::EXECUTABLE_NAME))
        }
        fn shell_type(&self) -> structopt::clap::Shell {
            structopt::clap::Shell::Zsh
        }
        fn shell_rcs(&self) -> &'static [&'static str] {
            &[".zshrc"]
        }
    }

    impl ShellPathProvider for Powershell {
        fn new() -> Self {
            Powershell {}
        }
        fn dir_relative(&self) -> PathBuf {
            Path::new("shell").join("powershell")
        }
        fn file_path(&self) -> PathBuf {
            self.dir_relative().join("config.ps1")
        }
        fn autocomplete(&self) -> PathBuf {
            self.dir_relative().join("completion.ps1")
        }
        fn shell_type(&self) -> structopt::clap::Shell {
            structopt::clap::Shell::PowerShell
        }
        fn shell_rcs(&self) -> &'static [&'static str] {
            unimplemented!()
        }
    }

    impl ShellPathProvider for Fish {
        fn new() -> Self {
            Fish {}
        }
        fn dir_relative(&self) -> PathBuf {
            Path::new("shell").join("fish")
        }
        fn file_path(&self) -> PathBuf {
            self.dir_relative().join("config.fish")
        }
        fn autocomplete(&self) -> PathBuf {
            self.dir_relative().join("completion.fish")
        }
        fn shell_type(&self) -> structopt::clap::Shell {
            structopt::clap::Shell::Fish
        }
        fn shell_rcs(&self) -> &'static [&'static str] {
            unimplemented!()
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::path::Path;

    use super::*;

    use crate::utils::directory::shell::ShellPathProvider;

    use hygeia_test_helpers::create_test_temp_dir;

    #[test]
    fn hygeia_paths_from_env() {
        // Playing an env variables is subject to race conditions
        // since tests are run in parallel. Simply call the constructor
        // and the function.
        let paths_provider: PycorsPathsProvider<PycorsPathsProviderFromEnv> =
            PycorsPathsProviderFromEnv::new();
        let _ = paths_provider.project_home();
    }

    #[test]
    fn bash_dir_relative() {
        assert_eq!(
            shell::Bash::new().dir_relative(),
            Path::new("shell").join("bash")
        );
    }

    #[test]
    fn bash_file_path() {
        assert_eq!(
            shell::Bash::new().file_path(),
            Path::new("shell").join("bash").join("config.sh")
        );
    }

    #[test]
    fn bash_autocomplete() {
        assert_eq!(
            shell::Bash::new().autocomplete(),
            Path::new("shell").join("bash").join("completion.sh")
        );
    }

    #[test]
    fn zsh_dir_relative() {
        assert_eq!(
            shell::Zsh::new().dir_relative(),
            Path::new("shell").join("zsh")
        );
    }

    #[test]
    fn zsh_file_path() {
        assert_eq!(
            shell::Zsh::new().file_path(),
            Path::new("shell").join("zsh").join("config.sh")
        );
    }

    #[test]
    fn zsh_autocomplete() {
        assert_eq!(
            shell::Zsh::new().autocomplete(),
            Path::new("shell").join("zsh").join("_hygeia")
        );
    }

    mod hygeia_paths_trait {
        use super::*;
        use crate::constants::project_home_env_variable;

        fn default_home_full_path() -> PathBuf {
            dirs_next::home_dir().unwrap()
        }

        fn default_dot_full_path() -> PathBuf {
            default_home_full_path().join(DEFAULT_DOT_DIR)
        }

        #[test]
        fn path_provider_env() {
            let expected = match env::var(project_home_env_variable()) {
                Ok(dir) => PathBuf::from(dir),
                Err(_) => default_dot_full_path(),
            };

            let paths_provider = PycorsPathsProviderFromEnv::new();
            let to_validate = paths_provider.project_home();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn config_home_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home;

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.project_home();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn config_home_from_env_variable() {
            let home = default_home_full_path();
            let hygeia_home = create_test_temp_dir!().join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home;

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.project_home();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn default_extra_package_file_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join(EXTRA_PACKAGES_FILENAME);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.default_extra_package_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn default_extra_package_file_from_env_variable() {
            let home = default_home_full_path();
            let hygeia_home = create_test_temp_dir!().join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join(EXTRA_PACKAGES_FILENAME);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.default_extra_package_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn cache_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.cache();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn cache_from_env_variable() {
            let home = default_home_full_path();
            let hygeia_home = create_test_temp_dir!().join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.cache();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn installed_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("installed").join("cpython");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.installed();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn installed_from_env_variable() {
            let home = default_home_full_path();
            let hygeia_home = create_test_temp_dir!().join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("installed").join("cpython");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.installed();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn logs_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("logs");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.logs();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn logs_from_env_variable() {
            let home = default_home_full_path();
            let hygeia_home = create_test_temp_dir!().join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("logs");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.logs();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn shims_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("shims");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.shims();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn shims_from_env_variable() {
            let home = create_test_temp_dir!();
            let hygeia_home = home.join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("shims");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.shims();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn downloaded_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache").join("downloaded");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.downloaded();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn downloaded_from_env_variable() {
            let home = create_test_temp_dir!();
            let hygeia_home = home.join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache").join("downloaded");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.downloaded();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn available_toolchain_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache").join(AVAILABLE_TOOLCHAIN_CACHE);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.available_toolchains_cache_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn available_toolchain_from_env_variable() {
            let home = create_test_temp_dir!();
            let hygeia_home = home.join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache").join(AVAILABLE_TOOLCHAIN_CACHE);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.available_toolchains_cache_file();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn extracted_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache").join("extracted");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.extracted();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn extracted_from_env_variable() {
            let home = create_test_temp_dir!();
            let hygeia_home = home.join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let expected = hygeia_home.join("cache").join("extracted");

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.extracted();
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn install_dir_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            let expected = hygeia_home
                .join("installed")
                .join("cpython")
                .join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.install_dir(&version);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn install_dir_from_env_variable() {
            let home = create_test_temp_dir!();
            let hygeia_home = home.join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            let expected = hygeia_home
                .join("installed")
                .join("cpython")
                .join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.install_dir(&version);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn bin_dir_from_default() {
            let home = default_home_full_path();
            let hygeia_home = default_dot_full_path();

            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            #[cfg(not(windows))]
            let expected = hygeia_home
                .join("installed")
                .join("cpython")
                .join(version_str)
                .join("bin");
            #[cfg(windows)]
            let expected = hygeia_home
                .join("installed")
                .join("cpython")
                .join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.bin_dir(&version);
            assert_eq!(to_validate, expected);
        }

        #[test]
        fn bin_dir_from_env_variable() {
            let home = create_test_temp_dir!();
            let hygeia_home = home.join(".hygeia");

            let mocked_home = Some(home);
            let mocked_hygeia_home = Some(hygeia_home.clone());

            let version_str = "3.7.5";
            let version = Version::parse(version_str).unwrap();

            #[cfg(not(windows))]
            let expected = hygeia_home
                .join("installed")
                .join("cpython")
                .join(version_str)
                .join("bin");
            #[cfg(windows)]
            let expected = hygeia_home
                .join("installed")
                .join("cpython")
                .join(version_str);

            let mut mock = MockPycorsHomeProviderTrait::new();
            mock.expect_project_home()
                .times(1)
                .return_const(mocked_hygeia_home);
            mock.expect_home().times(0).return_const(mocked_home);

            let paths_provider = PycorsPathsProvider::from(mock);
            let to_validate = paths_provider.bin_dir(&version);
            assert_eq!(to_validate, expected);
        }
    }
}
