use std::env;

use semver::Version;

use crate::{
    commands::{self, install::pip::install_extra_pip_packages},
    os::windows::build_filename_exe,
    utils, Result,
};

pub fn install_package(
    version: &Version,
    install_extra_packages: &commands::InstallExtraPackagesOptions,
) -> Result<()> {
    // See https://docs.python.org/3.7/using/windows.html#installing-without-ui

    let original_current_dir = env::current_dir()?;

    let install_dir = utils::install_dir(version)?;
    let target_dir_opt = format!("TargetDir={}", install_dir.display());

    let unattended_arguments = vec![
        "/quiet",
        "InstallAllUsers=0",
        &target_dir_opt,
        "Shortcuts=0",
        "Include_launcher=0",
        "InstallLauncherAllUsers=0",
        "Include_pip=1",
    ];

    let cwd = utils::pycors_download()?;
    let exe = format!("./{}", build_filename_exe(version)?);

    utils::run_cmd_template(
        &version,
        "[3/15] Unattended install",
        &exe,
        &unattended_arguments,
        cwd,
    )?;

    // Create a file in install directory to detect if we installed it ourselves
    utils::create_info_file(&install_dir, version)?;

    install_extra_pip_packages(&install_dir, &version, install_extra_packages)?;

    log::debug!(
        "Changing back current directory to {:?}",
        original_current_dir
    );
    env::set_current_dir(&original_current_dir)?;

    Ok(())
}
