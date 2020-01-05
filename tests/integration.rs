use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use assert_cmd::{assert::OutputAssertExt, Command};
use indoc::indoc;
use predicates::prelude::*;

use pycors::{
    constants::{home_env_variable, EXECUTABLE_NAME, INFO_FILE, TOOLCHAIN_FILE},
    Result,
};

pub fn temp_dir(subdir: &str) -> PathBuf {
    let dir = env::temp_dir()
        .join(EXECUTABLE_NAME)
        .join("integration_tests");

    if !dir.exists() {
        fs::create_dir_all(&dir).unwrap();
    }
    let dir = dir.canonicalize().unwrap().join(subdir);

    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }

    fs::create_dir_all(&dir).unwrap();

    dir
}

mod integration {
    use super::*;

    fn test_version(output: std::process::Output) {
        let assert_output = output.assert();

        assert_output
            .success()
            .stdout(
                predicate::str::similar(format!(
                    "{} {}\n",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                ))
                .normalize(),
            )
            .stderr("");
    }

    #[test]
    fn version_long() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd.arg("--version").unwrap();
        test_version(output);
    }

    #[test]
    fn version_short() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd.arg("-V").unwrap();
        test_version(output);
    }

    fn test_help(output: std::process::Output) {
        let assert_output = output.assert();

        assert_output
            .success()
            .stdout(
                predicate::str::starts_with(format!(
                    "{} {}\n",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                ))
                .normalize()
                .and(predicate::str::contains("USAGE:"))
                .and(predicate::str::contains("FLAGS:"))
                .and(predicate::str::contains("SUBCOMMANDS:")),
            )
            .stderr("");
    }

    #[test]
    fn help_long() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd.arg("--help").unwrap();
        test_help(output);
    }

    #[test]
    fn help_short() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd.arg("-h").unwrap();
        test_help(output);
    }

    #[test]
    fn list_with_empty_dir() {
        let pycors_home = temp_dir("list_with_empty_dir");
        let cwd = pycors_home.join("current_dir");
        let _ = fs::create_dir_all(&cwd);
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("list")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .current_dir(&cwd) // Change to a clean directory without a '.python-version'
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar(indoc!("
                +--------+---------+---------------------+----------+
                | Active | Version | Installed by pycors | Location |
                +--------+---------+---------------------+----------+"
            )).trim().normalize()
            )
        // .stderr("")
        ;
    }

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

    #[test]
    fn list_two_custom_no_system() {
        let pycors_home = temp_dir("list_two_custom_no_system");
        let cwd = pycors_home.join("current_dir");
        select("=3.7.5", &cwd);
        let location_380_dir = installed(&pycors_home, "3.8.0", false).unwrap();
        let location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
        let location_374_dir = installed(&pycors_home, "3.7.4", true).unwrap();

        // The 'Location' column expands to the path
        let dashes = "-".repeat(location_380_dir.len());
        let spaces = " ".repeat(location_380_dir.len() - 9);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("list")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar(format!(
"+--------+---------+---------------------+-{}-+
| Active | Version | Installed by pycors | Location {} |
+--------+---------+---------------------+-{}-+
|        |  3.8.0  |                     | {} |
+--------+---------+---------------------+-{}-+
|   ✓    |  3.7.5  |          ✓          | {} |
+--------+---------+---------------------+-{}-+
|        |  3.7.4  |          ✓          | {} |
+--------+---------+---------------------+-{}-+
",
                dashes,
                spaces,
                dashes,
                location_380_dir,
                dashes,
                location_375_dir,
                dashes,
                location_374_dir,
                dashes,
            )).normalize()
            )
        // .stderr("")
        ;
    }

    #[test]
    fn list_selected_but_not_installed() {
        let pycors_home = temp_dir("list_selected_but_not_installed");
        let cwd = pycors_home.join("current_dir");
        select("=3.7.5", &cwd);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("list")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar(indoc!("
                +--------+---------+---------------------+----------+
                | Active | Version | Installed by pycors | Location |
                +--------+---------+---------------------+----------+
                |   ✗    |  3.7.5  |                     |          |
                +--------+---------+---------------------+----------+"
            )).trim().normalize()
            )
        // .stderr("")
        ;
    }

    #[test]
    fn path_none() {
        let pycors_home = temp_dir("path_none");
        let cwd = pycors_home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();
        // select("=3.7.5", &cwd);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("path")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar("\n").normalize())
        // .stderr("")
        ;
    }

    #[test]
    fn path_some_select() {
        let pycors_home = temp_dir("path_some_select");
        let cwd = pycors_home.join("current_dir");
        let _location_380_dir = installed(&pycors_home, "3.8.0", false).unwrap();
        let location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
        let _location_374_dir = installed(&pycors_home, "3.7.4", true).unwrap();
        fs::create_dir_all(&cwd).unwrap();
        select("=3.7.5", &cwd);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("path")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar(location_375_dir).trim())
            .stderr("");
    }

    #[test]
    fn path_some_latest() {
        let pycors_home = temp_dir("path_some_latest");
        let cwd = pycors_home.join("current_dir");
        let location_380_dir = installed(&pycors_home, "3.8.0", false).unwrap();
        let _location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
        let _location_374_dir = installed(&pycors_home, "3.7.4", true).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("path")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar(location_380_dir).trim())
            .stderr("");
    }

    #[test]
    fn path_some_version_overwrite() {
        let pycors_home = temp_dir("path_some_version_overwrite");
        let cwd = pycors_home.join("current_dir");
        let _location_380_dir = installed(&pycors_home, "3.8.0", false).unwrap();
        let location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
        let _location_374_dir = installed(&pycors_home, "3.7.4", true).unwrap();
        select("=3.8.0", &cwd);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("path")
            .arg("--version")
            .arg("~3.7")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar(location_375_dir).trim())
            .stderr("");
    }

    #[test]
    fn select_from_none_exact() {
        let pycors_home = temp_dir("select_from_none_exact");
        let cwd = pycors_home.join("current_dir");
        let _location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("select")
            .arg("=3.7.5")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
        assert_eq!(file_content.trim(), "= 3.7.5");
    }

    #[test]
    fn select_from_none_tilde() {
        let pycors_home = temp_dir("select_from_none_tilde");
        let cwd = pycors_home.join("current_dir");
        let _location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("select")
            .arg("~3.7")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
        assert_eq!(file_content.trim(), "= 3.7.5");
    }

    #[test]
    fn select_from_some_exact() {
        let pycors_home = temp_dir("select_from_some_exact");
        let cwd = pycors_home.join("current_dir");
        let _location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
        let _location_380_dir = installed(&pycors_home, "3.8.0", true).unwrap();
        select("=3.8.0", &cwd);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("select")
            .arg("=3.7.5")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
        assert_eq!(file_content.trim(), "= 3.7.5");
    }

    #[test]
    fn select_from_none_not_installed() {
        let pycors_home = temp_dir("select_from_none_not_installed");
        let cwd = pycors_home.join("current_dir");
        let _location_380_dir = installed(&pycors_home, "3.8.0", true).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("select")
            .arg("=3.7.5")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd);
        let assert_output = output.assert();
        assert_output
            .failure()
            .stdout("")
            .stderr(predicate::str::similar("Error: Python version =3.7.5 not found!").trim());

        assert!(!cwd.join(TOOLCHAIN_FILE).exists());
    }

    #[test]
    fn select_from_some_not_installed() {
        let pycors_home = temp_dir("select_from_some_not_installed");
        let cwd = pycors_home.join("current_dir");
        let _location_380_dir = installed(&pycors_home, "3.8.0", true).unwrap();
        select("= 3.8.0", &cwd);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("select")
            .arg("=3.7.5")
            .env(home_env_variable(), &pycors_home)
            .env("PATH", pycors_home.join("usr_bin"))
            .env("RUST_LOG", "")
            .current_dir(&cwd);
        let assert_output = output.assert();
        assert_output
            .failure()
            .stdout("")
            .stderr(predicate::str::similar("Error: Python version =3.7.5 not found!").trim());

        let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
        assert_eq!(file_content.trim(), "= 3.8.0");
    }
}
