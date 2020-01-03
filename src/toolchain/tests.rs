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

use crate::utils::directory::MockPycorsHomeProviderTrait;

fn temp_dir(subdir: &str) -> PathBuf {
    let dir = env::temp_dir()
        .join(crate::constants::EXECUTABLE_NAME)
        .join("toolchain");

    if !dir.exists() {
        fs::create_dir_all(&dir).unwrap();
    }
    let dir = dir.canonicalize().unwrap().join(subdir);

    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }

    fs::create_dir_all(&dir).unwrap();

    dir
}

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
    let dir = temp_dir("version_or_path_from_str_err_path_success");
    let v = dir.to_string_lossy();
    let vop: ToolchainFile = v.parse().unwrap();
    assert_eq!(vop, ToolchainFile::Path(dir));
}

#[test]
fn version_or_path_from_str_err_path_failed_dir_not_found() {
    let dir = temp_dir("version_or_path_from_str_err_path_failed_dir_not_found");
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
    let dir = temp_dir("toolchain_file_load_success_none");

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
        let dir = temp_dir("toolchain_file_load_error_not_permitted");

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

#[test]
fn toolchain_file_load_error_garbage() {
    let v = "non-Version parsable content";
    let dir = temp_dir("toolchain_file_load_error_garbage");

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
    let dir = temp_dir("toolchain_file_load");

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
    let extracted_version = extract_version_from_command(&python_path, Ok(output)).unwrap();
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
    let extracted_version = extract_version_from_command(&python_path, Ok(output)).unwrap();
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
    let dir = temp_dir("selected_toolchain_from_toolchain_file_path_installed");
    let dir = dir.canonicalize().unwrap();

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
    assert_eq!(selected_toolchain.is_installed(), true);
}

#[test]
fn selected_toolchain_installed_toolchain_is_installed_false() {
    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: None,
    });
    assert_eq!(selected_toolchain.is_installed(), false);
}

#[test]
fn selected_toolchain_installed_toolchain_same_version_true() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert_eq!(selected_toolchain.same_version(&version_req), true);
}

#[test]
fn selected_toolchain_installed_toolchain_same_version_false() {
    let version_req = VersionReq::parse("=2.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert_eq!(selected_toolchain.same_version(&version_req), false);
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_version_version_true() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: Some(VersionReq::parse("=3.7.4").unwrap()),
    });
    assert_eq!(selected_toolchain.same_version(&version_req), true);
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_version_version_false() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: Some(VersionReq::parse("3.7.4").unwrap()),
    });
    assert_eq!(selected_toolchain.same_version(&version_req), false);
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_version_none_false() {
    let version_req = VersionReq::parse("=3.7.4").unwrap();

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: None,
    });
    assert_eq!(selected_toolchain.same_version(&version_req), false);
}

#[test]
fn selected_toolchain_installed_toolchain_same_location_true() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: location.clone(),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert_eq!(selected_toolchain.same_location(&location), true);
}

#[test]
fn selected_toolchain_installed_toolchain_same_location_false() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::InstalledToolchain(InstalledToolchain {
        location: PathBuf::from("/usr/local/bin"),
        version: Version::parse("3.7.4").unwrap(),
    });
    assert_eq!(selected_toolchain.same_location(&location), false);
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_location_some_true() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: Some(location.clone()),
        version: None,
    });
    assert_eq!(selected_toolchain.same_location(&location), true);
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_location_some_false() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: Some(location.clone().join("different")),
        version: None,
    });
    assert_eq!(selected_toolchain.same_location(&location), false);
}

#[test]
fn selected_toolchain_not_installed_toolchain_same_location_none_false() {
    let location = PathBuf::from("/usr/bin");

    let selected_toolchain = SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
        location: None,
        version: None,
    });
    assert_eq!(selected_toolchain.same_location(&location), false);
}

#[test]
fn get_python_versions_from_path_pycors_home_dir_absent() {
    let pycors_home = temp_dir("get_python_versions_from_path_pycors_home_dir_absent");
    fs::remove_dir_all(&pycors_home).unwrap();
    let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home_env_variable()
        .times(0)
        .return_const(mocked_pycors_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let python_versions = get_python_versions_from_path(&pycors_home, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_shim_dir_absent() {
    let pycors_home = temp_dir("get_python_versions_from_path_shim_dir_absent");
    let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home_env_variable()
        .times(1)
        .return_const(mocked_pycors_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let python_versions = get_python_versions_from_path(&pycors_home, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_shim_skipped() {
    let pycors_home = temp_dir("get_python_versions_from_path_shim_skipped");
    let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home_env_variable()
        .times(1 + 1) // We need the shim dir to call function, hence +1
        .return_const(mocked_pycors_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    let python_versions = get_python_versions_from_path(&shims_dir, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_2717_and_374_and_375() {
    let pycors_home = temp_dir("get_python_versions_from_path_2717_and_374_and_375");
    let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home_env_variable()
        .times(1 + 1)
        .return_const(mocked_pycors_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    #[cfg(windows)]
    let exec_extension = ".exe";
    #[cfg(not(windows))]
    let exec_extension = "";

    let filename_to_print = pycors_home.join(format!(
        "python3{}_pycors_tests_to_print_stdout.txt",
        exec_extension
    ));
    let mut f = File::create(filename_to_print).unwrap();
    f.write_all(b"Python 3.7.5").unwrap();
    std::mem::drop(f);

    let filename_to_print = pycors_home.join(format!(
        "python{}_pycors_tests_to_print_stdout.txt",
        exec_extension
    ));
    let mut f = File::create(filename_to_print).unwrap();
    f.write_all(b"Python 3.7.4").unwrap();
    std::mem::drop(f);

    // NOTE: Python 2 prints its version to stderr, not stdout.
    let filename_to_print = pycors_home.join(format!(
        "python2{}_pycors_tests_to_print_stderr.txt",
        exec_extension
    ));
    let mut f = File::create(filename_to_print).unwrap();
    f.write_all(b"Python 2.7.17").unwrap();
    std::mem::drop(f);

    std::process::Command::new("cargo")
        .args(&["build", "--package", "print_file_to_stdout"])
        .output()
        .with_context(|| "Failed to execute 'cargo build --package print_file_to_stdout")
        .unwrap();
    let print_file_to_stdout = {
        #[cfg_attr(not(windows), allow(unused_mut))]
        let mut tmp = Path::new("target")
            .join("debug")
            .join("print_file_to_stdout");

        #[cfg(windows)]
        tmp.set_extension("exe");

        tmp
    };

    fs::copy(
        &print_file_to_stdout,
        pycors_home.join(format!("python{}", exec_extension)),
    )
    .with_context(|| {
        format!(
            "Failed to copy {:?} to {:?}",
            &print_file_to_stdout,
            pycors_home.join(format!("python{}", exec_extension))
        )
    })
    .unwrap();
    fs::copy(
        &print_file_to_stdout,
        pycors_home.join(format!("python2{}", exec_extension)),
    )
    .unwrap();
    fs::copy(
        &print_file_to_stdout,
        pycors_home.join(format!("python3{}", exec_extension)),
    )
    .unwrap();

    let python_versions = get_python_versions_from_path(&pycors_home, &paths_provider);

    let expected_versions: HashMap<Version, PathBuf> = [
        (Version::parse("3.7.4").unwrap(), pycors_home.clone()),
        (Version::parse("3.7.5").unwrap(), pycors_home.clone()),
        (Version::parse("2.7.17").unwrap(), pycors_home.clone()),
    ]
    .iter()
    .cloned()
    .collect();

    assert_eq!(python_versions, expected_versions);
}

#[test]
fn get_python_versions_from_path_single_word_wont_parse() {
    let pycors_home = temp_dir("get_python_versions_from_path_single_word_wont_parse");
    let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home_env_variable()
        .times(1 + 1)
        .return_const(mocked_pycors_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    let filename_to_print = pycors_home.join("python_pycors_tests_to_print_stdout.txt");
    let mut f = File::create(filename_to_print).unwrap();
    f.write_all(b"single_word_wont_parse").unwrap();
    std::mem::drop(f);

    let print_file_to_stdout = Path::new("target/debug/print_file_to_stdout");
    if !print_file_to_stdout.exists() {
        std::process::Command::new("cargo")
            .args(&["build", "--package", "print_file_to_stdout"])
            .output()
            .with_context(|| "Failed to execute 'cargo build --package print_file_to_stdout")
            .unwrap();
    }
    fs::copy(&print_file_to_stdout, pycors_home.join("python")).unwrap();

    let python_versions = get_python_versions_from_path(&pycors_home, &paths_provider);

    assert!(python_versions.is_empty());
}

#[test]
fn get_python_versions_from_path_non_version_wont_parse() {
    let pycors_home = temp_dir("get_python_versions_from_path_non_version_wont_parse");
    let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_home_env_variable()
        .times(1 + 1)
        .return_const(mocked_pycors_home);
    let paths_provider = PycorsPathsProvider::from(mock);

    let shims_dir = paths_provider.shims();
    fs::create_dir_all(&shims_dir).unwrap();

    let filename_to_print = pycors_home.join("python_pycors_tests_to_print_stdout.txt");
    let mut f = File::create(filename_to_print).unwrap();
    f.write_all(b"Python not_a_version").unwrap();
    std::mem::drop(f);

    let print_file_to_stdout = Path::new("target/debug/print_file_to_stdout");
    if !print_file_to_stdout.exists() {
        std::process::Command::new("cargo")
            .args(&["build", "--package", "print_file_to_stdout"])
            .output()
            .with_context(|| "Failed to execute 'cargo build --package print_file_to_stdout")
            .unwrap();
    }
    fs::copy(print_file_to_stdout, pycors_home.join("python")).unwrap();

    let python_versions = get_python_versions_from_path(&pycors_home, &paths_provider);

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
        crate::tests::init_logger();
        let pycors_home = temp_dir("get_python_versions_from_path_failure_to_run");
        let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home_env_variable()
            .times(1 + 1)
            .return_const(mocked_pycors_home);
        let paths_provider = PycorsPathsProvider::from(mock);

        let shims_dir = paths_provider.shims();
        fs::create_dir_all(&shims_dir).unwrap();

        let filename_to_print = pycors_home.join("python");
        let mut f = File::create(filename_to_print).unwrap();
        f.write_all(b"This is not an executable.").unwrap();
        // Make file executable
        let permissions = fs::Permissions::from_mode(0o755);
        f.set_permissions(permissions).unwrap();
        std::mem::drop(f);

        let python_versions = get_python_versions_from_path(&pycors_home, &paths_provider);

        assert!(python_versions.is_empty());
    }
}
