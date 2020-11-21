use std::path::Path;

use super::*;

#[cfg_attr(windows, ignore)]
#[test]
fn setup_bash_success_from_scratch() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    let cwd = home.join("current_dir");
    fs::create_dir_all(&cwd).unwrap();

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("bash")
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
            predicate::str::contains("BASH successfully configured!")
                .normalize()
                .trim(),
        )
        .stderr(predicate::str::is_empty().trim());

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
export HYGEIA_HOME="{}"
source "{}"
# End of hygeia config block.
#############################################################################
"#,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        hygeia_home.display(),
        Path::new("${HYGEIA_HOME}")
            .join("shell")
            .join("bash")
            .join("config.sh")
            .display(),
    );

    let bashrc_content = fs::read_to_string(home.join(".bashrc")).unwrap();
    let bash_profile_content = fs::read_to_string(home.join(".bash_profile")).unwrap();

    assert_eq!(bashrc_content, expected_bashrc);
    assert_eq!(bash_profile_content, expected_bashrc);
}

#[cfg_attr(windows, ignore)]
#[test]
fn setup_bash_success_twice() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
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
export HYGEIA_HOME="{}"
source "{}"
# End of hygeia config block.
#############################################################################
"#,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        EXECUTABLE_NAME,
        hygeia_home.display(),
        Path::new("${HYGEIA_HOME}")
            .join("shell")
            .join("bash")
            .join("config.sh")
            .display(),
    );

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .arg("setup")
        .arg("bash")
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
            predicate::str::contains("BASH successfully configured!")
                .normalize()
                .trim(),
        )
        .stderr(predicate::str::is_empty().trim());

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
            predicate::str::contains("BASH successfully configured!")
                .normalize()
                .trim(),
        )
        .stderr(predicate::str::is_empty().trim());

    // After a second 'hygeia setup bash', the extra lines should be above since
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
