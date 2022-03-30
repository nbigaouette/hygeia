use super::*;

#[cfg_attr(windows, ignore)]
#[test]
fn setup_fish_success_from_scratch() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    fs::create_dir_all(&cwd).unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("fish")
        .env(project_home_env_variable(), &hygeia_home)
        .env(home_overwrite_env_variable(), &home)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(
            predicate::str::contains("FISH successfully configured!")
                .normalize()
                .trim(),
        )
        .stderr(predicate::str::is_empty().trim());
    Command::new("hygeia").output().unwrap();
}
