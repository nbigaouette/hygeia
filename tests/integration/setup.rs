use super::*;

#[test]
fn setup_bash_success_from_scratch() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
    fs::create_dir_all(&cwd).unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("bash")
        .env(project_home_env_variable(), &pycors_home)
        .env(home_overwrite_env_variable(), &home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output.success().stdout("").stderr("");

    let expected_bashrc = format!(
        r#"

#############################################################################
# Start of {} config block.
# These lines were added by {} and are required for it to function
# properly (including the comments!)
# See https://github.com/nbigaouette/{}
# WARNING: Those lines _need_ to be at the end of the file: {} needs to
#          appear as soon as possible in the $PATH environment variable to
#          to function properly.
export PYCORS_HOME="{}"
source ${{PYCORS_HOME}}/shell/bash/config.sh
# End of pycors config block.
#############################################################################
"#,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        pycors_home.display()
    );

    let bashrc_content = fs::read_to_string(home.join(".bashrc")).unwrap();
    let bash_profile_content = fs::read_to_string(home.join(".bash_profile")).unwrap();

    assert_eq!(bashrc_content, expected_bashrc);
    assert_eq!(bash_profile_content, expected_bashrc);
}

#[test]
fn setup_bash_success_twice() {
    let home = create_test_temp_dir!();
    let pycors_home = home.join(".pycors");
    let cwd = home.join("current_dir");
    fs::create_dir_all(&cwd).unwrap();

    let expected_bashrc_block = format!(
        r#"

#############################################################################
# Start of {} config block.
# These lines were added by {} and are required for it to function
# properly (including the comments!)
# See https://github.com/nbigaouette/{}
# WARNING: Those lines _need_ to be at the end of the file: {} needs to
#          appear as soon as possible in the $PATH environment variable to
#          to function properly.
export PYCORS_HOME="{}"
source ${{PYCORS_HOME}}/shell/bash/config.sh
# End of pycors config block.
#############################################################################
"#,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        pycors_home.display()
    );

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("bash")
        .env(project_home_env_variable(), &pycors_home)
        .env(home_overwrite_env_variable(), &home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output.success().stdout("").stderr("");

    let bashrc_content = fs::read_to_string(home.join(".bashrc")).unwrap();

    assert_eq!(bashrc_content, expected_bashrc_block);

    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(home.join(".bashrc"))
        .unwrap();
    f.write_all(b"First line\nSecond line\nThird line\n")
        .unwrap();
    std::mem::drop(f);

    // After writing the extra lines, the files should have them at the end.
    let bashrc_content = fs::read_to_string(home.join(".bashrc")).unwrap();
    assert_eq!(
        bashrc_content,
        format!(
            "{}First line\nSecond line\nThird line\n",
            expected_bashrc_block
        )
    );

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("bash")
        .env(project_home_env_variable(), &pycors_home)
        .env(home_overwrite_env_variable(), &home)
        .env("PATH", pycors_home.join("usr_bin"))
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output.success().stdout("").stderr("");

    // After a second 'pycors setup bash', the extra lines should be above since
    // our block is extracted and moved to the end of the file.
    let bashrc_content = fs::read_to_string(home.join(".bashrc")).unwrap();
    assert_eq!(
        bashrc_content,
        format!(
            "\n\nFirst line\nSecond line\nThird line\n{}",
            expected_bashrc_block
        )
    );
}
