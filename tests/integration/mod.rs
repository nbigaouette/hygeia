// NOTE: The module (integration) cannot be named the same as the test
// executable name (integration_tests) due to a bug in rustfmt.
// See: https://github.com/rust-lang/rustfmt/issues/3794

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use assert_cmd::{assert::OutputAssertExt, Command};
use indoc::indoc;
use predicates::prelude::*;
use semver::Version;

use hygeia::{
    constants::{
        home_overwrite_env_variable, project_home_env_variable, EXECUTABLE_NAME, INFO_FILE,
        TOOLCHAIN_FILE,
    },
    utils::directory::{PycorsHomeProviderTrait, PycorsPathsProvider},
    Result,
};

use hygeia_test_helpers::{create_test_temp_dir, function_path, mock_executable, MockedOutput};

mod help;
mod install;
mod list;
mod path;
mod run;
mod select;
mod setup;

mockall::mock! {
    PycorsHomeProviderTrait {}     // Name of the mock struct, less the "Mock" prefix
    impl PycorsHomeProviderTrait for PycorsHomeProviderTrait {   // definition of the trait to mock
        fn home(&self) -> Option<PathBuf>;
        fn document(&self) -> Option<PathBuf>;
        fn project_home(&self) -> Option<PathBuf>;
        fn paths(&self) -> Vec<PathBuf>;
    }
}

fn select(version: &str, cwd: &Path) {
    let _ = fs::create_dir_all(&cwd);
    let mut f = fs::File::create(cwd.join(TOOLCHAIN_FILE)).unwrap();
    f.write_all(version.as_bytes()).unwrap();
}

fn installed(hygeia_home: &Path, version: &str, installed_by_us: bool) -> Result<String> {
    let installed_dir = hygeia_home.join("installed").join("cpython");
    let installation_dir = installed_dir.join(version);

    #[cfg(windows)]
    let location_dir = installation_dir.clone();
    #[cfg(not(windows))]
    let location_dir = installation_dir.join("bin");

    fs::create_dir_all(&location_dir)?;

    // Simulate first one being installed by us
    if installed_by_us {
        let mut f = fs::File::create(installation_dir.join(INFO_FILE))?;
        f.write_all(b"")?;
    }

    Ok(location_dir.to_string_lossy().to_string())
}
