use super::*;

#[test]
fn from_none_exact() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _location_375_dir = installed(&hygeia_home, "3.7.5", true).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("select")
        .arg("=3.7.5")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::is_empty().trim())
        .stderr(predicate::str::is_empty().trim());

    let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
    assert_eq!(file_content.trim(), "=3.7.5");
}

#[test]
fn from_none_tilde() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _location_375_dir = installed(&hygeia_home, "3.7.5", true).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("select")
        .arg("~3.7")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::is_empty().trim())
        .stderr(predicate::str::is_empty().trim());

    let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
    assert_eq!(file_content.trim(), "=3.7.5");
}

#[test]
fn from_some_exact() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _location_375_dir = installed(&hygeia_home, "3.7.5", true).unwrap();
    let _location_380_dir = installed(&hygeia_home, "3.8.0", true).unwrap();
    select("=3.8.0", &cwd);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("select")
        .arg("=3.7.5")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::is_empty().trim())
        .stderr(predicate::str::is_empty().trim());

    let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
    assert_eq!(file_content.trim(), "=3.7.5");
}

#[test]
fn from_none_not_installed() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _location_380_dir = installed(&hygeia_home, "3.8.0", true).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("select")
        .arg("=3.7.5")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd);
    let assert_output = output.assert();
    assert_output
        .failure()
        .stdout(predicate::str::is_empty().trim())
        .stderr(predicate::str::similar("Error: Python version =3.7.5 not found!").trim());

    assert!(!cwd.join(TOOLCHAIN_FILE).exists());
}

#[test]
fn from_some_not_installed() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _location_380_dir = installed(&hygeia_home, "3.8.0", true).unwrap();
    select("=3.8.0", &cwd);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("select")
        .arg("=3.7.5")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd);
    let assert_output = output.assert();
    assert_output
        .failure()
        .stdout(predicate::str::is_empty().trim())
        .stderr(predicate::str::similar("Error: Python version =3.7.5 not found!").trim());

    let file_content = fs::read_to_string(cwd.join(TOOLCHAIN_FILE)).unwrap();
    assert_eq!(file_content.trim(), "=3.8.0");
}
