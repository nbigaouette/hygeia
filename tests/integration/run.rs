use super::*;

#[test]
fn run_success_python_version_from_selected() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");

    select("=3.7.5", &cwd);

    let _location_380_dir = installed(&pycors_home, "3.8.0", false).unwrap();
    let location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
    let _location_374_dir = installed(&pycors_home, "3.7.4", true).unwrap();

    mock_executable(
        &location_375_dir,
        "python",
        MockedOutput {
            out: Some("Python 3.7.5"),
            err: None,
        },
    )
    .unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("run")
        .arg("python --version")
        .env(project_home_env_variable(), &pycors_home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar("Python 3.7.5").trim().normalize())
        .stderr("");
}

#[test]
fn run_success_python_version_from_overwrite() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");

    select("=3.7.5", &cwd);

    let location_380_dir = installed(&pycors_home, "3.8.0", false).unwrap();
    let location_375_dir = installed(&pycors_home, "3.7.5", true).unwrap();
    let location_374_dir = installed(&pycors_home, "3.7.4", true).unwrap();

    mock_executable(
        &location_374_dir,
        "python",
        MockedOutput {
            out: Some("Python 3.7.4"),
            err: None,
        },
    )
    .unwrap();
    mock_executable(
        &location_375_dir,
        "python",
        MockedOutput {
            out: Some("Python 3.7.5"),
            err: None,
        },
    )
    .unwrap();
    mock_executable(
        &location_380_dir,
        "python",
        MockedOutput {
            out: Some("Python 3.8.0"),
            err: None,
        },
    )
    .unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("run")
        .arg("--version")
        .arg("~3.7")
        .arg("python --version")
        .env(project_home_env_variable(), &pycors_home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar("Python 3.7.5").trim().normalize())
        .stderr("");

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("run")
        .arg("--version")
        .arg("=3.7.4")
        .arg("python --version")
        .env(project_home_env_variable(), &pycors_home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar("Python 3.7.4").trim().normalize())
        .stderr("");

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("run")
        .arg("--version")
        .arg("=3.7.5")
        .arg("python --version")
        .env(project_home_env_variable(), &pycors_home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar("Python 3.7.5").trim().normalize())
        .stderr("");

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("run")
        .arg("--version")
        .arg("=3.8.0")
        .arg("python --version")
        .env(project_home_env_variable(), &pycors_home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar("Python 3.8.0").trim().normalize())
        .stderr("");
}
