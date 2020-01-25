use super::*;

#[test]
fn with_empty_dir() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    let _ = fs::create_dir_all(&cwd);
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("list")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .current_dir(&cwd) // Change to a clean directory without a '.python-version'
        .unwrap();
    let assert_output = output.assert();
    assert_output
            .success()
            .stdout(predicate::str::similar(indoc!("
                +--------+---------+---------------------+----------+
                | Active | Version | Installed by hygeia | Location |
                +--------+---------+---------------------+----------+"
            )).trim().normalize()
            )
        // .stderr(predicate::str::is_empty().trim())
        ;
}

#[test]
fn two_custom_no_system() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    select("=3.7.5", &cwd);
    let location_380_dir = installed(&hygeia_home, "3.8.0", false).unwrap();
    let location_375_dir = installed(&hygeia_home, "3.7.5", true).unwrap();
    let location_374_dir = installed(&hygeia_home, "3.7.4", true).unwrap();

    // The 'Location' column expands to the path
    let dashes = "-".repeat(location_380_dir.len());
    let spaces = " ".repeat(location_380_dir.len() - 9);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("list")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
            .success()
            .stdout(predicate::str::similar(format!(
"+--------+---------+---------------------+-{}-+
| Active | Version | Installed by hygeia | Location {} |
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
        // .stderr(predicate::str::is_empty().trim())
        ;
}

#[test]
fn selected_but_not_installed() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    select("=3.7.5", &cwd);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("list")
        .env(project_home_env_variable(), &hygeia_home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
            .success()
            .stdout(predicate::str::similar(indoc!("
                +--------+---------+---------------------+----------+
                | Active | Version | Installed by hygeia | Location |
                +--------+---------+---------------------+----------+
                |   ✗    |  3.7.5  |                     |          |
                +--------+---------+---------------------+----------+"
            )).trim().normalize()
            )
        // .stderr(predicate::str::is_empty().trim())
        ;
}
