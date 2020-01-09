use super::*;

// NOTE: Do not overwrite the 'PATH' environment variable in 'Command' calls: toolchains
//       are being compiled, they thus need access to compilers and co.

use rstest::rstest;

#[ignore]
#[rstest(
    version,
    case::version_381("3.8.1"),
    case::version_380("3.8.0"),
    case::version_376("3.7.6"),
    case::version_375("3.7.5"),
    case::version_374("3.7.4"),
    case::version_373("3.7.3"),
    case::version_372("3.7.2"),
    case::version_371("3.7.1"),
    case::version_370("3.7.0"),
    case::version_3610("3.6.10"),
    case::version_369("3.6.9"),
    case::version_368("3.6.8"),
    case::version_367("3.6.7"),
    case::version_366("3.6.6"),
    case::version_365("3.6.5"),
    case::version_364("3.6.4"),
    case::version_363("3.6.3"),
    case::version_362("3.6.2"),
    case::version_361("3.6.1"),
    case::version_360("3.6.0"),
    case::version_359("3.5.9"),
    case::version_358("3.5.8"),
    case::version_357("3.5.7"),
    case::version_356("3.5.6"),
    case::version_355("3.5.5"),
    case::version_354("3.5.4"),
    case::version_353("3.5.3"),
    case::version_352("3.5.2"),
    case::version_351("3.5.1"),
    case::version_350("3.5.0"),
    case::version_3410("3.4.10"),
    case::version_349("3.4.9"),
    case::version_348("3.4.8"),
    case::version_347("3.4.7"),
    case::version_346("3.4.6"),
    case::version_345("3.4.5"),
    case::version_344("3.4.4"),
    case::version_343("3.4.3"),
    case::version_342("3.4.2"),
    case::version_341("3.4.1"),
    case::version_340("3.4.0"),
    case::version_337("3.3.7"),
    case::version_336("3.3.6"),
    case::version_335("3.3.5"),
    case::version_334("3.3.4"),
    case::version_333("3.3.3"),
    case::version_332("3.3.2"),
    case::version_331("3.3.1"),
    case::version_330("3.3.0"),
    case::version_326("3.2.6"),
    case::version_325("3.2.5"),
    case::version_324("3.2.4"),
    case::version_323("3.2.3"),
    case::version_322("3.2.2"),
    case::version_321("3.2.1"),
    case::version_320("3.2.0"),
    case::version_315("3.1.5"),
    case::version_314("3.1.4"),
    case::version_313("3.1.3"),
    case::version_312("3.1.2"),
    case::version_311("3.1.1"),
    case::version_310("3.1.0"),
    case::version_301("3.0.1"),
    case::version_300("3.0.0")
)]
fn install_python(version: &str) {
    let home = create_test_temp_dir!(version);
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
        .arg(format!("={}", version))
        .env(project_home_env_variable(), &pycors_home)
        .env(home_overwrite_env_variable(), &home)
        .env("RUST_LOG", "")
        .current_dir(&cwd)
        .unwrap();
    let assert_output = output.assert();
    assert_output
        .success()
        .stdout(predicate::str::is_empty().trim())
        .stderr(predicate::str::is_empty().trim());

    // Make sure installation worked
    assert_python_successfully_installed(&paths_provider, version, &cwd);
}
