use super::*;

#[cfg_attr(not(windows), ignore)]
#[test]
fn simple_install_from_scratch_success() {
    #[cfg(not(windows))]
    {
        eprintln!(
            "WARNING: Test {} skipped on non-Windows platform to prevent long compilation.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let hygeia_home = home.join(".hygeia");
        let paths = vec![home.join("usr_bin"), home.join("usr_local_bin")];

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(hygeia_home.clone());
        mock.expect_paths().return_const(paths.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &hygeia_home)
            .env(home_overwrite_env_variable(), &home)
            .env("PATH", env::join_paths(paths.iter()).unwrap())
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(
                predicate::str::contains("🐍 Python 3.7.5 successfully installed!")
                    .normalize()
                    .trim(),
            )
            .stderr(predicate::str::is_empty().trim());

        // Make sure pip was installed successfully
        assert_pip_successfully_installed(&paths_provider);

        // Make sure no '.python-version' was created
        assert!(!cwd.join(TOOLCHAIN_FILE).exists());

        // Make sure we can run the installed toolchain
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("run")
            .arg("--version")
            .arg("=3.7.5")
            .arg("python --version")
            .env(project_home_env_variable(), &hygeia_home)
            .env("PATH", env::join_paths(paths.iter()).unwrap())
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(predicate::str::similar("Python 3.7.5").trim().normalize())
            .stderr(predicate::str::is_empty().trim());
    }
}

#[cfg_attr(not(windows), ignore)]
#[test]
fn simple_install_from_scratch_select_success() {
    #[cfg(not(windows))]
    {
        eprintln!(
            "WARNING: Test {} skipped on non-Windows platform to prevent long compilation.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let hygeia_home = home.join(".hygeia");
        let paths = vec![home.join("usr_bin"), home.join("usr_local_bin")];

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(hygeia_home.clone());
        mock.expect_paths().return_const(paths.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .arg("--select")
            .env(project_home_env_variable(), &hygeia_home)
            .env(home_overwrite_env_variable(), &home)
            .env("PATH", env::join_paths(paths.iter()).unwrap())
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(
                predicate::str::contains("🐍 Python 3.7.5 successfully installed!")
                    .normalize()
                    .trim(),
            )
            .stderr(predicate::str::is_empty().trim());

        // Make sure pip was installed successfully
        assert_pip_successfully_installed(&paths_provider);

        // Make sure '.python-version' file was created
        let selected_file = cwd.join(TOOLCHAIN_FILE);
        let selected_file_content = fs::read_to_string(&selected_file)
            .with_context(|| format!("Failed to read selected file {:?}", selected_file))
            .unwrap();
        assert_eq!(selected_file_content, "= 3.7.5\n");
    }
}

#[cfg_attr(not(windows), ignore)]
#[test]
fn install_twice_noop() {
    #[cfg(not(windows))]
    {
        eprintln!(
            "WARNING: Test {} skipped on non-Windows platform to prevent long compilation.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let hygeia_home = home.join(".hygeia");
        let paths = vec![home.join("usr_bin"), home.join("usr_local_bin")];

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(hygeia_home.clone());
        mock.expect_paths().return_const(paths.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &hygeia_home)
            .env(home_overwrite_env_variable(), &home)
            .env("PATH", env::join_paths(paths.iter()).unwrap())
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(
                predicate::str::contains("🐍 Python 3.7.5 successfully installed!")
                    .normalize()
                    .trim(),
            )
            .stderr(predicate::str::is_empty().trim());

        // Make sure pip was installed successfully
        assert_pip_successfully_installed(&paths_provider);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &hygeia_home)
            .env(home_overwrite_env_variable(), &home)
            .env("PATH", env::join_paths(paths.iter()).unwrap())
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(
                predicate::str::contains("🐍 Python 3.7.5 successfully installed!")
                    .normalize()
                    .trim(),
            )
            .stderr(predicates::str::contains(
                "Python version 3.7.5 already installed!",
            ));
    }
}

#[cfg_attr(not(windows), ignore)]
#[test]
fn install_twice_forced() {
    #[cfg(not(windows))]
    {
        eprintln!(
            "WARNING: Test {} skipped on non-Windows platform to prevent long compilation.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let hygeia_home = home.join(".hygeia");
        let paths = vec![home.join("usr_bin"), home.join("usr_local_bin")];

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(hygeia_home.clone());
        mock.expect_paths().return_const(paths.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &hygeia_home)
            .env(home_overwrite_env_variable(), &home)
            .env("PATH", env::join_paths(paths.iter()).unwrap())
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(
                predicate::str::contains("🐍 Python 3.7.5 successfully installed!")
                    .normalize()
                    .trim(),
            )
            .stderr(predicate::str::is_empty().trim());

        // Make sure pip was installed successfully
        assert_pip_successfully_installed(&paths_provider);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .arg("--force")
            .env(project_home_env_variable(), &hygeia_home)
            .env(home_overwrite_env_variable(), &home)
            .env("PATH", env::join_paths(paths.iter()).unwrap())
            .env("RUST_LOG", "hygeia=info")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout(
                predicate::str::contains("🐍 Python 3.7.5 successfully installed!")
                    .normalize()
                    .trim(),
            )
            .stderr(predicates::str::contains(
                "skipped: file get-pip.py already downloaded.",
            ));
    }
}
