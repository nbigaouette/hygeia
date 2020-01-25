use super::*;

#[test]
fn none_found() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    fs::create_dir_all(&cwd).unwrap();
    // select("=3.7.5", &cwd);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("path")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar("\n").normalize())
    // .stderr(predicate::str::is_empty().trim())
    ;
}

#[test]
fn some_select() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _location_380_dir = installed(&hygeia_home, "3.8.0", false).unwrap();
    let location_375_dir = installed(&hygeia_home, "3.7.5", true).unwrap();
    let _location_374_dir = installed(&hygeia_home, "3.7.4", true).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    select("=3.7.5", &cwd);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("path")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar(location_375_dir))
        .stderr(predicate::str::is_empty().trim());
}

#[test]
fn some_latest() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let location_380_dir = installed(&hygeia_home, "3.8.0", false).unwrap();
    let _location_375_dir = installed(&hygeia_home, "3.7.5", true).unwrap();
    let _location_374_dir = installed(&hygeia_home, "3.7.4", true).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("path")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar(location_380_dir))
        .stderr(predicate::str::is_empty().trim());
}

#[test]
fn some_version_overwrite() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _location_380_dir = installed(&hygeia_home, "3.8.0", false).unwrap();
    let location_375_dir = installed(&hygeia_home, "3.7.5", true).unwrap();
    let _location_374_dir = installed(&hygeia_home, "3.7.4", true).unwrap();
    select("=3.8.0", &cwd);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("path")
        .arg("--version")
        .arg("~3.7")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::similar(location_375_dir))
        .stderr(predicate::str::is_empty().trim());
}
