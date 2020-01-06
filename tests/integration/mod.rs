// NOTE: The module (integration) cannot be named the same as the test
// executable name (integration_tests) due to a bug in rustfmt.
// See: https://github.com/rust-lang/rustfmt/issues/3794

use std::{env, fs, io::Write, path::Path};

use assert_cmd::{assert::OutputAssertExt, Command};
use indoc::indoc;
use predicates::prelude::*;

use pycors::{
    constants::{home_env_variable, EXECUTABLE_NAME, INFO_FILE, TOOLCHAIN_FILE},
    Result,
};

// https://stackoverflow.com/a/40234666
macro_rules! function_path {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

macro_rules! create_test_temp_dir {
    () => {{
        let dir = env::temp_dir()
            .join(EXECUTABLE_NAME)
            .join("integration_tests");

        if !dir.exists() {
            fs::create_dir_all(&dir).unwrap();
        }
        let mut dir = dir.canonicalize().unwrap();
        for component in function_path!().split("::").skip(1) {
            dir.push(component);
        }

        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }

        fs::create_dir_all(&dir).unwrap();

        dir
    }};
}

mod help;
mod install;
mod list;
mod path;
mod run;
mod select;
mod setup;
mod version;

fn select(version: &str, cwd: &Path) {
    let _ = fs::create_dir_all(&cwd);
    let mut f = fs::File::create(cwd.join(TOOLCHAIN_FILE)).unwrap();
    f.write_all(version.as_bytes()).unwrap();
}

fn installed(pycors_home: &Path, version: &str, installed_by_us: bool) -> Result<String> {
    let installed_dir = pycors_home.join("installed");
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
