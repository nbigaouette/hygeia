use super::*;

// NOTE: Do not overwrite the 'PATH' environment variable in 'Command' calls: toolchains
//       are being compiled, they thus need access to compilers and co.

#[test]
fn simple_install_from_scratch_success() {
    #[cfg(windows)]
    {
        eprintln!(
            "WARNING: Test {} skipped on Windows since not meant to compile there.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let pycors_home = home.join(".pycors");

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(pycors_home.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &pycors_home)
            .env(home_overwrite_env_variable(), &home)
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        // Make sure installation worked
        assert_python_successfully_installed(&paths_provider, "3.7.5", &cwd);

        // Make sure no '.python-version' was created
        assert!(!cwd.join(TOOLCHAIN_FILE).exists());
    }
}

#[test]
fn simple_install_from_scratch_select_success() {
    #[cfg(windows)]
    {
        eprintln!(
            "WARNING: Test {} skipped on Windows since not meant to compile there.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let pycors_home = home.join(".pycors");

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(pycors_home.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .arg("--select")
            .env(project_home_env_variable(), &pycors_home)
            .env(home_overwrite_env_variable(), &home)
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        // Make sure installation worked
        assert_python_successfully_installed(&paths_provider, "3.7.5", &cwd);

        // Make sure '.python-version' file was created
        let selected_file = cwd.join(TOOLCHAIN_FILE);
        let selected_file_content = fs::read_to_string(&selected_file)
            .with_context(|| format!("Failed to read selected file {:?}", selected_file))
            .unwrap();
        assert_eq!(selected_file_content, "= 3.7.5\n");
    }
}

#[test]
fn install_twice_noop() {
    #[cfg(windows)]
    {
        eprintln!(
            "WARNING: Test {} skipped on Windows since not meant to compile there.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let pycors_home = home.join(".pycors");

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(pycors_home.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &pycors_home)
            .env(home_overwrite_env_variable(), &home)
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        // Make sure installation worked
        assert_python_successfully_installed(&paths_provider, "3.7.5", &cwd);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &pycors_home)
            .env(home_overwrite_env_variable(), &home)
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output
            .success()
            .stdout("")
            .stderr(predicates::str::contains(
                "Python version 3.7.5 already installed!",
            ));
    }
}

#[test]
fn install_twice_forced() {
    #[cfg(windows)]
    {
        eprintln!(
            "WARNING: Test {} skipped on Windows since not meant to compile there.",
            function_path!()
        );
        return;
    }
    #[allow(unreachable_code)]
    {
        let home = create_test_temp_dir!();
        let pycors_home = home.join(".pycors");

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home().return_const(home.clone());
        mock.expect_project_home().return_const(pycors_home.clone());
        let paths_provider = PycorsPathsProvider::from(mock);

        let cwd = home.join("current_dir");
        fs::create_dir_all(&cwd).unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .env(project_home_env_variable(), &pycors_home)
            .env(home_overwrite_env_variable(), &home)
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        // Make sure installation worked
        assert_python_successfully_installed(&paths_provider, "3.7.5", &cwd);

        // Since we'll force a reinstallation, delete the installed directory.
        fs::remove_dir_all(
            paths_provider
                .install_dir(&Version::parse("3.7.5").unwrap())
                .join("bin"),
        )
        .unwrap();

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        let output = cmd
            .arg("install")
            .arg("=3.7.5")
            .arg("--force")
            .env(project_home_env_variable(), &pycors_home)
            .env(home_overwrite_env_variable(), &home)
            .env("RUST_LOG", "")
            .current_dir(&cwd)
            .unwrap();
        let assert_output = output.assert();
        assert_output.success().stdout("").stderr("");

        // Make sure installation worked
        assert_python_successfully_installed(&paths_provider, "3.7.5", &cwd);
    }
}
