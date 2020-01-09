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
    assert_output
        .success()
        .stdout(
            predicate::str::similar(format!("Python {}", version))
                .trim()
                .normalize(),
        )
        .stderr(predicate::str::is_empty().trim());

    // Make sure the prefix matches the expected installation directory
    if cfg!(not(target_os = "windows")) {
        let mut cmd = Command::new(&bin_file);
        let output = cmd
            .arg("-c")
            .arg(r#""import sysconfig; print(sysconfig.get_config_vars('prefix'))""#)
            .current_dir(&cwd)
            .unwrap();

        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(
                predicate::str::contains(format!(
                    "{}",
                    paths_provider.install_dir(&version).display()
                ))
                .trim()
                .normalize(),
            )
            .stderr(predicate::str::is_empty().trim());
    }
}

mod long_compilation;
mod windows_only;
