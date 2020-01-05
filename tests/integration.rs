use std::{env, fs, path::PathBuf};

use assert_cmd::{assert::OutputAssertExt, Command};
use indoc::indoc;
use predicates::prelude::*;

use pycors::constants::{home_env_variable, EXECUTABLE_NAME};

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
}
