use super::*;
use pycors::utils::directory::{shell, PycorsPathsProvider};

#[cfg_attr(not(windows), ignore)]
#[test]
fn setup_powershell_success_from_scratch() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
    fs::create_dir_all(&cwd).unwrap();

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home().return_const(home.clone());
    mock.expect_project_home().return_const(pycors_home.clone());
    let paths_provider = PycorsPathsProvider::from(mock);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("powershell")
        .env(project_home_env_variable(), &pycors_home)
        .env(home_overwrite_env_variable(), &home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output.success().stdout("").stderr("");

    let expected_ps_profile = format!(
        r#"$env:Path += ";{}"
        . {}"#,
        paths_provider.shims().display(),
        paths_provider
            .project_home()
            .join(shell::powershell::config::autocomplete())
            .display(),
    );


}
