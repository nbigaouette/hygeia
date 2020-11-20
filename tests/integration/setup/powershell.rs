use super::*;
use hygeia::{
    constants::document_overwrite_env_variable,
    utils::directory::{
        shell::{Powershell, ShellPathProvider},
        PycorsPathsProvider,
    },
};

#[cfg_attr(not(windows), ignore)]
#[test]
fn setup_powershell_success_from_scratch() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let document = home.join("Documents");
    let cwd = home.join("current_dir");
    fs::create_dir_all(&cwd).unwrap();

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home().return_const(home.clone());
    mock.expect_project_home().return_const(hygeia_home.clone());
    mock.expect_document().return_const(document.clone());

    let paths_provider = PycorsPathsProvider::from(mock);

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("powershell")
        .env(project_home_env_variable(), &hygeia_home)
        .env(home_overwrite_env_variable(), &home)
        .env(document_overwrite_env_variable(), &document)
        .env("PATH", hygeia_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(
            predicate::str::contains("POWERSHELL successfully configured!")
                .normalize()
                .trim(),
        )
        .stderr(predicate::str::is_empty().trim());

    let expected_ps_profile = format!(
        r#"$env:PATH += ";{}"
. {}
"#,
        paths_provider.shims().display(),
        paths_provider
            .project_home()
            .join(Powershell::new().autocomplete())
            .display(),
    );

    let ps_profile_content = fs::read_to_string(
        document
            .join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    )
    .unwrap();

    assert_eq!(ps_profile_content, expected_ps_profile);
}
