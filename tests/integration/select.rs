use super::*;

#[test]
fn from_none_exact() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
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
fn from_none_tilde() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
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
fn from_some_exact() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
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
fn from_none_not_installed() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
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
fn from_some_not_installed() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
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
