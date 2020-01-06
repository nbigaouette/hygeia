use super::*;

#[test]
fn with_empty_dir() {
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

#[test]
fn two_custom_no_system() {
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
fn selected_but_not_installed() {
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
