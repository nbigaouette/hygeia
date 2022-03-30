use super::*;

#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
#[cfg(not(windows))]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    process::{ExitStatus, Output},
};

use hygeia_test_helpers::{create_test_temp_dir, mock_executable, MockedOutput};

use crate::{constants::INFO_FILE, utils::directory::MockPycorsHomeProviderTrait};

#[test]
fn version_or_path_from_str_success_major_minor_patch() {
    let v = "3.7.4";
    let vop: ToolchainFile = v.parse().unwrap();
    assert_eq!(
        vop,
        ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
    );
}
#[test]
fn version_or_path_from_str_success_eq_major_minor_patch() {
    let v = "=3.7.4";
    let vop: ToolchainFile = v.parse().unwrap();
    assert_eq!(
        vop,
        ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
    );
}

#[test]
fn version_or_path_from_str_success_tilde_major_minor() {
    let v = "~3.7";
    let vop: ToolchainFile = v.parse().unwrap();
    assert_eq!(
        vop,
        ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
    );
}

#[test]
fn version_or_path_from_str_success_tilde_major() {
    let v = "~3";
    let vop: ToolchainFile = v.parse().unwrap();
    assert_eq!(
        vop,
        ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
    );
}

#[test]
fn version_or_path_from_str_err_path_success() {
    let dir = create_test_temp_dir!();
    let v = dir.to_string_lossy();
    let vop: ToolchainFile = v.parse().unwrap();
    assert_eq!(vop, ToolchainFile::Path(dir));
}

#[test]
fn version_or_path_from_str_err_path_failed_dir_not_found() {
    let dir = create_test_temp_dir!();
    let v = dir.to_string_lossy();
    let vop: ToolchainFile = v.parse().unwrap();
    assert_eq!(vop, ToolchainFile::Path(dir));
}

use std::sync::{Arc, Mutex};
fn with_directory<P, T>(dir: P, c: impl Fn() -> Result<T>) -> Result<T>
where
    P: AsRef<Path>,
{
    lazy_static::lazy_static! {
        static ref CHANGE_DIR_MUTEX: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    }
    let _change_dir_mutex = CHANGE_DIR_MUTEX.lock().unwrap();
    let initial_current_dir = env::current_dir().unwrap();
    env::set_current_dir(dir).unwrap();
    let r = c();
    env::set_current_dir(&initial_current_dir).unwrap();
    r
}

#[test]
fn toolchain_file_load_success_none() {
    let dir = create_test_temp_dir!();

    let vop: Result<Option<ToolchainFile>> = with_directory(dir, ToolchainFile::load);

    assert_eq!(vop.unwrap(), None);
}

#[test]
fn toolchain_file_load_error_not_permitted() {
    #[cfg(windows)]
    {
        println!(
            "Test skipped on Windows since it doesn't support 'std::os::unix::fs::PermissionsExt'"
        );
    }

    #[cfg(not(windows))]
    {
        let v = "3.7.4";
        let dir = create_test_temp_dir!();

        if users::get_current_uid() == 0 {
            eprintln!("WARNING: Running test as root is disabled; root can read any file!");
        } else {
            let mut toolchain_file = File::create(dir.join(TOOLCHAIN_FILE)).unwrap();
            toolchain_file.write_all(v.as_bytes()).unwrap();
            let permissions = fs::Permissions::from_mode(0o200); // -w-------
            toolchain_file.set_permissions(permissions).unwrap();
            std::mem::drop(toolchain_file);

            let vop: Result<Option<ToolchainFile>> = with_directory(dir, ToolchainFile::load);

            let err = vop.unwrap_err();
            assert_eq!(
                err.downcast_ref::<std::io::Error>().unwrap().kind(),
                std::io::ErrorKind::PermissionDenied
            );
        }
    }
}

#[test]
fn toolchain_file_load_error_garbage() {
    let v = "non-Version parsable content";
    let dir = create_test_temp_dir!();

    let mut toolchain_file = File::create(dir.join(TOOLCHAIN_FILE)).unwrap();
    toolchain_file.write_all(v.as_bytes()).unwrap();
    std::mem::drop(toolchain_file);

    let vop: Result<Option<ToolchainFile>> = with_directory(dir, ToolchainFile::load);

    // In case ToolchainFile cannot parse a Version, it will be interpreted as a Path.
    assert_eq!(
        vop.unwrap().unwrap(),
        ToolchainFile::Path(PathBuf::from_str(v).unwrap())
    );
}

#[test]
fn toolchain_file_load_success_some() {
    let v = "3.7.4";
    let dir = create_test_temp_dir!();

    let mut toolchain_file = File::create(dir.join(TOOLCHAIN_FILE)).unwrap();
    toolchain_file.write_all(v.as_bytes()).unwrap();
    std::mem::drop(toolchain_file);

    let new_current_dir = dir.join("first").join("second").join("third");
    fs::create_dir_all(&new_current_dir).unwrap();

    let vop: Result<Option<ToolchainFile>> = with_directory(new_current_dir, ToolchainFile::load);

    let vop = vop.unwrap().unwrap();

    assert_eq!(
        vop,
        ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
    );
}

#[test]
fn extract_version_from_command_success_py3() {
    let expected_version = String::from("Python 3.7.5");
    let output = Output {
        status: ExitStatus::from_raw(0),
        stdout: expected_version.as_bytes().to_vec(),
        stderr: b"".to_vec(),
    };
    let python_path = Path::new("/usr/local/python");
    let extracted_version = extract_version_from_command(python_path, Ok(output)).unwrap();
    assert_eq!(extracted_version, expected_version);
}

#[test]
fn extract_version_from_command_success_py2() {
    let expected_version = String::from("Python 2.7.10");
    let output = Output {
        status: ExitStatus::from_raw(0),
        stdout: b"".to_vec(),
        stderr: expected_version.as_bytes().to_vec(),
    };
    let python_path = Path::new("/usr/local/python2");
    let extracted_version = extract_version_from_command(python_path, Ok(output)).unwrap();
    assert_eq!(extracted_version, expected_version);
}

#[test]
fn selected_toolchain_from_toolchain_file_version_req_installed() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();
    let toolchain_file: ToolchainFile = ToolchainFile::VersionReq(version_req);
    let installed_toolchains: &[InstalledToolchain] = &[InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("3.7.4").unwrap(),
    }];
    let selected_toolchain =
        SelectedToolchain::from_toolchain_file(&toolchain_file, installed_toolchains);
    assert_eq!(
        selected_toolchain,
        SelectedToolchain::InstalledToolchain(InstalledToolchain {
            location: installed_toolchains[0].location.clone(),
            version: installed_toolchains[0].version.clone(),
        })
    );
}

#[test]
fn selected_toolchain_from_toolchain_file_version_req_not_installed() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();
    let toolchain_file: ToolchainFile = ToolchainFile::VersionReq(version_req.clone());
    let installed_toolchains: &[InstalledToolchain] = &[];
    let selected_toolchain =
        SelectedToolchain::from_toolchain_file(&toolchain_file, installed_toolchains);
    assert_eq!(
        selected_toolchain,
        SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
            version: Some(version_req),
            location: None,
        }),
    );
}

#[test]
fn selected_toolchain_from_toolchain_file_path_not_installed() {
    let dir = create_test_temp_dir!().canonicalize().unwrap();

    let toolchain_file: ToolchainFile = ToolchainFile::Path(dir.clone());
    let installed_toolchains: &[InstalledToolchain] = &[InstalledToolchain {
        location: dir,
        version: Version::parse("3.7.4").unwrap(),
    }];
    let selected_toolchain =
        SelectedToolchain::from_toolchain_file(&toolchain_file, installed_toolchains);
    assert_eq!(
        selected_toolchain,
        SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
            location: Some(installed_toolchains[0].location.clone()),
            version: None,
        })
    );
}

#[test]
fn selected_toolchain_installed_toolchain_version_req() {
    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert_eq!(
        selected_toolchain.version_req().unwrap(),
        VersionReq::parse("=3.7.4").unwrap()
    );
}

#[test]
fn selected_toolchain_not_installed_toolchain_version_req_some() {
    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: Some(VersionReq::parse("=3.7.4").unwrap()),
    });
    assert_eq!(
        selected_toolchain.version_req().unwrap(),
        VersionReq::parse("=3.7.4").unwrap()
    );
}

#[test]
fn selected_toolchain_not_installed_toolchain_version_req_none() {
    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: Some(PathBuf::from("/usr/bin")),
        version: None,
    });
    assert_eq!(selected_toolchain.version_req(), None);
}

#[test]
fn selected_toolchain_installed_toolchain_is_installed_true() {
    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert!(selected_toolchain.is_installed());
}

#[test]
fn selected_toolchain_installed_toolchain_is_installed_false() {
    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: None,
    });
    assert!(!selected_toolchain.is_installed());
}

#[test]
fn selected_toolchain_installed_toolchain_same_version_true() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert!(selected_toolchain.same_version(&version_req));
}

#[test]
fn selected_toolchain_installed_toolchain_same_version_false() {
    let version_req = VersionReq::parse("=2.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert!(!selected_toolchain.same_version(&version_req));
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_version_version_true() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: Some(VersionReq::parse("=3.7.4").unwrap()),
    });
    assert!(selected_toolchain.same_version(&version_req));
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_version_version_false() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: Some(VersionReq::parse("3.7.4").unwrap()),
    });
    assert!(!selected_toolchain.same_version(&version_req));
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_version_none_false() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: None,
    });
    assert!(!selected_toolchain.same_version(&version_req));
}

#[test]
fn selected_toolchain_installed_toolchain_same_location_true() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: location.clone(),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert!(selected_toolchain.same_location(&location));
}

#[test]
fn selected_toolchain_installed_toolchain_same_location_false() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/local/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert!(!selected_toolchain.same_location(&location));
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_location_some_true() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: Some(location.clone()),
        version: None,
    });
    assert!(selected_toolchain.same_location(&location));
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_location_some_false() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: Some(location.join("different")),
        version: None,
    });
    assert!(!selected_toolchain.same_location(&location));
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_location_none_false() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: None,
    });
    assert!(!selected_toolchain.same_location(&location));
}

#[test]
fn get_python_versions_from_path_hygeia_home_dir_absent() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_hygeia_home = Some(hygeia_home.clone());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(0)
        .return_const(mocked_hygeia_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let python_versions = get_python_versions_from_path(&hygeia_home, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_shim_dir_absent() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");
    fs::create_dir_all(&hygeia_home).unwrap();

    let mocked_home = Some(home);
    let mocked_hygeia_home = Some(hygeia_home.clone());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(1)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let python_versions = get_python_versions_from_path(&hygeia_home, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_shim_skipped() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home);
    let mocked_hygeia_home = Some(hygeia_home);

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(2) // We need the shim dir to call function, hence +1
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    let python_versions = get_python_versions_from_path(&shims_dir, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_2717_and_374_and_375() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home);
    let mocked_hygeia_home = Some(hygeia_home.clone());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(2)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    mock_executable(
        &hygeia_home,
        "python3",
        MockedOutput {
            out: Some("Python 3.7.5"),
            err: None,
        },
    )
    .unwrap();

    mock_executable(
        &hygeia_home,
        "python",
        MockedOutput {
            out: Some("Python 3.7.4"),
            err: None,
        },
    )
    .unwrap();

    // NOTE: Python 2 prints its version to stderr, not stdout.
    mock_executable(
        &hygeia_home,
        "python2",
        MockedOutput {
            out: None,
            err: Some("Python 2.7.17"),
        },
    )
    .unwrap();

    let python_versions = get_python_versions_from_path(&hygeia_home, &paths_provider);

    let expected_versions: HashMap<Version, PathBuf> = [
        (Version::parse("3.7.4").unwrap(), hygeia_home.clone()),
        (Version::parse("3.7.5").unwrap(), hygeia_home.clone()),
        (Version::parse("2.7.17").unwrap(), hygeia_home),
    ]
    .iter()
    .cloned()
    .collect();

    assert_eq!(python_versions, expected_versions);
}

#[test]
fn get_python_versions_from_path_single_word_wont_parse() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home);
    let mocked_hygeia_home = Some(hygeia_home.clone());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(2)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    mock_executable(
        &hygeia_home,
        "python",
        MockedOutput {
            out: Some("single_word_wont_parse"),
            err: None,
        },
    )
    .unwrap();

    let python_versions = get_python_versions_from_path(&hygeia_home, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_non_version_wont_parse() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home);
    let mocked_hygeia_home = Some(hygeia_home.clone());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(2)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    mock_executable(
        &hygeia_home,
        "python",
        MockedOutput {
            out: Some("Python not_a_version"),
            err: None,
        },
    )
    .unwrap();

    let python_versions = get_python_versions_from_path(&hygeia_home, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_failure_to_run() {
    #[cfg(windows)]
    {
        println!(
            "Test skipped on Windows since it doesn't support 'std::os::unix::fs::PermissionsExt'"
        );
    }

    #[cfg(not(windows))]
    {
        let home = create_test_temp_dir!();
        let hygeia_home = home.join(".hygeia");
        fs::create_dir_all(&hygeia_home).unwrap();

        let mocked_home = Some(home);
        let mocked_hygeia_home = Some(hygeia_home.clone());

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_project_home()
            .times(2)
            .return_const(mocked_hygeia_home);
        mock.expect_home().times(0).return_const(mocked_home);
        let paths_provider = PycorsPathsProvider::from(mock);

        let shims_dir = paths_provider.shims();
        fs::create_dir_all(&shims_dir).unwrap();

        let filename_to_print = hygeia_home.join("python");
        let mut f = File::create(filename_to_print).unwrap();
        f.write_all(b"This is not an executable.").unwrap();
        // Make file executable
        let permissions = fs::Permissions::from_mode(0o755);
        f.set_permissions(permissions).unwrap();
        std::mem::drop(f);

        let python_versions = get_python_versions_from_path(&hygeia_home, &paths_provider);

        assert!(python_versions.is_empty());
    }
}

#[test]
fn get_python_versions_from_path_python_2715_plus_should_parse_issue_102() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home);
    let mocked_hygeia_home = Some(hygeia_home.clone());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(2)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    mock_executable(
        &hygeia_home,
        "python",
        MockedOutput {
            out: Some("Python 2.7.15+"),
            err: None,
        },
    )
    .unwrap();

    let python_versions = get_python_versions_from_path(&hygeia_home, &paths_provider);

    let expected_versions: HashMap<Version, PathBuf> =
        [(Version::parse("2.7.15").unwrap(), hygeia_home)]
            .iter()
            .cloned()
            .collect();

    assert_eq!(python_versions, expected_versions);
}

#[test]
fn is_a_custom_install_true() {
    let dir = create_test_temp_dir!();
    let info_filename = dir.join(INFO_FILE);
    // Create file in directory
    let mut f = File::create(info_filename).unwrap();
    f.write_all(b"").unwrap();
    assert!(is_a_custom_install(&dir.join("bin")));
}

#[test]
fn is_a_custom_install_false() {
    let dir = create_test_temp_dir!();
    assert!(!is_a_custom_install(&dir.join("bin")));
}

#[test]
fn find_installed_toolchains_absent_dir() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home.clone());
    let mocked_hygeia_home = Some(hygeia_home);

    let mocked_usr_bin = home.join("usr_bin");
    let mocked_usr_local_bin = home.join("usr_local_bin");
    let mocked_paths = vec![mocked_usr_bin, mocked_usr_local_bin];

    // Make sure directory does not exists
    fs::remove_dir(&home).unwrap();

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(1)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    mock.expect_paths().times(1).return_const(mocked_paths);
    let paths_provider = PycorsPathsProvider::from(mock);

    let found_installed_toolchains = find_installed_toolchains(&paths_provider).unwrap();

    assert!(found_installed_toolchains.is_empty());
}

#[test]
fn find_installed_toolchains_empty_installed_dir() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home.clone());
    let mocked_hygeia_home = Some(hygeia_home);

    let mocked_usr_bin = home.join("usr_bin");
    let mocked_usr_local_bin = home.join("usr_local_bin");
    let mocked_paths = vec![mocked_usr_bin, mocked_usr_local_bin];

    // Make sure directory does not exists
    fs::remove_dir(&home).unwrap();

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(2)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    mock.expect_paths().times(1).return_const(mocked_paths);
    let paths_provider = PycorsPathsProvider::from(mock);

    let installed_dir = paths_provider.installed();
    fs::create_dir_all(&installed_dir).unwrap();

    let found_installed_toolchains = find_installed_toolchains(&paths_provider).unwrap();

    assert!(found_installed_toolchains.is_empty());
}

#[test]
fn find_installed_toolchains_dummy_custom_installs() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home.clone());
    let mocked_hygeia_home = Some(hygeia_home);

    let mocked_usr_bin = home.join("usr_bin");
    let mocked_usr_local_bin = home.join("usr_local_bin");
    let mocked_paths = vec![mocked_usr_bin, mocked_usr_local_bin];

    // Make sure directory does not exists
    fs::remove_dir(&home).unwrap();

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(4)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    mock.expect_paths().times(1).return_const(mocked_paths);
    let paths_provider = PycorsPathsProvider::from(mock);

    let installed_dir = paths_provider.installed();
    fs::create_dir_all(&installed_dir).unwrap();

    mock_executable(
        installed_dir.join("3.7.5").join("bin"),
        "python3",
        MockedOutput {
            out: Some("Python 3.7.5"),
            err: None,
        },
    )
    .unwrap();

    mock_executable(
        installed_dir.join("3.7.4").join("bin"),
        "python3",
        MockedOutput {
            out: Some("Python 3.7.4"),
            err: None,
        },
    )
    .unwrap();

    let found_installed_toolchains = find_installed_toolchains(&paths_provider).unwrap();

    assert_eq!(found_installed_toolchains.len(), 2);

    // Windows pre-built binaries don't have a 'bin' subdirectory, so the paths
    // returned by `paths_provider.bin_dir()` (used by `find_installed_toolchains()`) will
    // not have a `bin` suffix on Windows.
    let expected_installed_dir = |version| {
        let dir = installed_dir.join(version);
        #[cfg(not(windows))]
        let dir = dir.join("bin");
        dir
    };

    assert_eq!(
        found_installed_toolchains[0],
        InstalledToolchain {
            location: expected_installed_dir("3.7.5"),
            version: Version::parse("3.7.5").unwrap()
        }
    );

    assert_eq!(
        found_installed_toolchains[1],
        InstalledToolchain {
            location: expected_installed_dir("3.7.4"),
            version: Version::parse("3.7.4").unwrap()
        }
    );
}

#[test]
fn find_installed_toolchains_dummy_system_installs() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home.clone());
    let mocked_hygeia_home = Some(hygeia_home);

    let mocked_usr_bin = home.join("usr_bin");
    let mocked_usr_local_bin = home.join("usr_local_bin");
    let mocked_paths = vec![mocked_usr_bin.clone(), mocked_usr_local_bin.clone()];

    // Make sure directory does not exists
    fs::remove_dir(&home).unwrap();

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(4)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    mock.expect_paths().times(1).return_const(mocked_paths);
    let paths_provider = PycorsPathsProvider::from(mock);

    let installed_dir = paths_provider.installed();
    fs::create_dir_all(&installed_dir).unwrap();

    mock_executable(
        mocked_usr_local_bin.clone(),
        "python3",
        MockedOutput {
            out: Some("Python 3.7.5"),
            err: None,
        },
    )
    .unwrap();

    mock_executable(
        mocked_usr_local_bin.clone(),
        "python",
        MockedOutput {
            out: Some("Python 3.7.5"),
            err: None,
        },
    )
    .unwrap();

    mock_executable(
        mocked_usr_bin.clone(),
        "python",
        MockedOutput {
            out: Some("Python 2.7.17"),
            err: None,
        },
    )
    .unwrap();

    mock_executable(
        mocked_usr_bin.clone(),
        "python2.7",
        MockedOutput {
            out: Some("Python 2.7.17"),
            err: None,
        },
    )
    .unwrap();

    let found_installed_toolchains = find_installed_toolchains(&paths_provider).unwrap();

    assert_eq!(found_installed_toolchains.len(), 2);

    assert_eq!(
        found_installed_toolchains[0],
        InstalledToolchain {
            location: mocked_usr_local_bin,
            version: Version::parse("3.7.5").unwrap()
        }
    );

    assert_eq!(
        found_installed_toolchains[1],
        InstalledToolchain {
            location: mocked_usr_bin,
            version: Version::parse("2.7.17").unwrap()
        }
    );
}

#[test]
fn find_compatible_toolchain_macos_default() {
    let installed_toolchains: &[InstalledToolchain] = &[InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("2.7.17").unwrap(),
    }];

    // No Python 3 available by default on macOS (Mojave)
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("3").unwrap(), installed_toolchains),
        None
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("^3").unwrap(), installed_toolchains),
        None
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("=3.7.5").unwrap(), installed_toolchains),
        None
    );

    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("2").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("^2").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~2").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("^2.7").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~2.7").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("=2.7.17").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );
}

#[test]
fn find_compatible_toolchain_multiple() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let installed_toolchains: &[InstalledToolchain] = &[
        InstalledToolchain {
            location: PathBuf::from("/usr/local/bin"),
            version: Version::parse("3.7.5").unwrap(),
        },
        InstalledToolchain {
            location: hygeia_home
                .join("installed")
                .join("cpython")
                .join("3.7.4")
                .join("bin"),
            version: Version::parse("3.7.4").unwrap(),
        },
        InstalledToolchain {
            location: hygeia_home
                .join("installed")
                .join("cpython")
                .join("3.8.0")
                .join("bin"),
            version: Version::parse("3.8.0").unwrap(),
        },
        InstalledToolchain {
            location: PathBuf::from("/usr/bin"),
            version: Version::parse("2.7.17").unwrap(),
        },
    ];

    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~3.7").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("=3.7.5").unwrap(), installed_toolchains),
        Some(&installed_toolchains[0])
    );

    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("=3.7.4").unwrap(), installed_toolchains),
        Some(&installed_toolchains[1])
    );

    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("3").unwrap(), installed_toolchains),
        Some(&installed_toolchains[2])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("^3").unwrap(), installed_toolchains),
        Some(&installed_toolchains[2])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~3").unwrap(), installed_toolchains),
        Some(&installed_toolchains[2])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~3.8").unwrap(), installed_toolchains),
        Some(&installed_toolchains[2])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("=3.8.0").unwrap(), installed_toolchains),
        Some(&installed_toolchains[2])
    );

    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("2").unwrap(), installed_toolchains),
        Some(&installed_toolchains[3])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("^2").unwrap(), installed_toolchains),
        Some(&installed_toolchains[3])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~2").unwrap(), installed_toolchains),
        Some(&installed_toolchains[3])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~2.7").unwrap(), installed_toolchains),
        Some(&installed_toolchains[3])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("=2.7.17").unwrap(), installed_toolchains),
        Some(&installed_toolchains[3])
    );
}

#[test]
fn find_compatible_toolchain_same_system_custom() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let installed_toolchains: &[InstalledToolchain] = &[
        InstalledToolchain {
            location: PathBuf::from("/usr/local/bin"),
            version: Version::parse("3.7.5").unwrap(),
        },
        InstalledToolchain {
            location: hygeia_home
                .join("installed")
                .join("cpython")
                .join("3.7.5")
                .join("bin"),
            version: Version::parse("3.7.5").unwrap(),
        },
        InstalledToolchain {
            location: hygeia_home
                .join("installed")
                .join("cpython")
                .join("4.0.0")
                .join("bin"),
            version: Version::parse("4.0.0").unwrap(),
        },
        InstalledToolchain {
            location: PathBuf::from("/usr/bin"),
            version: Version::parse("2.7.17").unwrap(),
        },
    ];

    // Tag as "custom installs" for proper priority
    for location in &[
        &installed_toolchains[1].location,
        &installed_toolchains[2].location,
    ] {
        fs::create_dir_all(&location).unwrap();
        let info_filename = location.parent().unwrap().join(INFO_FILE);
        // Create file in directory
        let mut f = File::create(info_filename).unwrap();
        f.write_all(b"").unwrap();
    }

    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("3").unwrap(), installed_toolchains),
        Some(&installed_toolchains[1])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("^3").unwrap(), installed_toolchains),
        Some(&installed_toolchains[1])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~3").unwrap(), installed_toolchains),
        Some(&installed_toolchains[1])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("~3.7").unwrap(), installed_toolchains),
        Some(&installed_toolchains[1])
    );
    assert_eq!(
        find_compatible_toolchain(&VersionReq::parse("=3.7.5").unwrap(), installed_toolchains),
        Some(&installed_toolchains[1])
    );
}

#[test]
fn compatible_toolchain_builder_load_from_string() {
    let home = create_test_temp_dir!();
    let hygeia_home = home.join(".hygeia");

    let mocked_home = Some(home.clone());
    let mocked_hygeia_home = Some(hygeia_home);
    let mocked_usr_bin = home.join("usr_bin");
    let mocked_usr_local_bin = home.join("usr_local_bin");
    let mocked_paths = vec![mocked_usr_bin, mocked_usr_local_bin];

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(1)
        .return_const(mocked_hygeia_home);
    mock.expect_home().times(0).return_const(mocked_home);
    mock.expect_paths().times(1).return_const(mocked_paths);
    let paths_provider = PycorsPathsProvider::from(mock);
    let compatible_toolchain = CompatibleToolchainBuilder::new()
        // .load_from_file()
        .load_from_string("=3.7.5")
        // .pick_latest_if_none_found()
        // .overwrite(VersionReq::parse("3.7.5").unwrap())
        .compatible_version(paths_provider)
        .unwrap();

    assert!(compatible_toolchain.is_none());
}
