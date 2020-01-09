use super::*;

use anyhow::Context;

use pycors_test_helpers::EXECUTABLE_EXTENSION;

fn assert_pip_successfully_installed<P>(paths_provider: &PycorsPathsProvider<P>)
where
    P: PycorsHomeProviderTrait,
{
    let log_path = paths_provider
        .logs()
        .join("Python_v3.7.5_step_Install_pip.log");
    let pip_installation_log_content = fs::read_to_string(&log_path)
        .with_context(|| format!("Failed to read log file {:?}", log_path))
        .unwrap();

    assert!(
        predicate::str::contains("Successfully installed pip").eval(&pip_installation_log_content)
    );
}

fn assert_python_successfully_installed<P, S, T>(
    paths_provider: &PycorsPathsProvider<P>,
    version: S,
    cwd: T,
) where
    P: PycorsHomeProviderTrait,
    S: AsRef<str>,
    T: AsRef<Path>,
{
    let version = Version::parse(version.as_ref()).unwrap();

    let bin_file = paths_provider
        .bin_dir(&version)
        .join(format!("python{}", EXECUTABLE_EXTENSION));

    // Make sure we can run the installed toolchain
    let mut cmd = Command::new(&bin_file);
    let output = cmd.arg("--version").current_dir(&cwd).unwrap();

    let assert_output = output.assert();

    let predicate_empty = predicate::str::is_empty().trim();
    let predicate_version = predicate::str::similar(format!("Python {}", version))
        .trim()
        .normalize();
    let predicate_prefix = predicate::str::contains(format!(
        "{}",
        paths_provider.install_dir(&version).display()
    ))
    .trim()
    .normalize();

    if version >= Version::parse("3.3.8").unwrap() {
        assert_output
            .success()
            .stdout(predicate_version)
            .stderr(predicate_empty);
    } else {
        assert_output
            .success()
            .stdout(predicate_empty)
            .stderr(predicate_version);
    };

    // Make sure the prefix matches the expected installation directory
    if cfg!(not(target_os = "windows")) {
        let config_bin_file = paths_provider
            .bin_dir(&version)
            .join(format!("python3-config{}", EXECUTABLE_EXTENSION));

        let mut cmd = Command::new(&config_bin_file);
        let output = cmd.arg("--prefix").current_dir(&cwd).unwrap();

        let assert_output = output.assert();
        if version >= Version::parse("3.3.8").unwrap() {
            assert_output
                .success()
                .stdout(predicate_prefix)
                .stderr(predicate_empty);
        } else {
            assert_output
                .success()
                .stdout(predicate_empty)
                .stderr(predicate_prefix);
        }
    }
}

mod long_compilation;
mod long_install_many;
mod windows_only;
