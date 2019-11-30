use std::{
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use semver::Version;
#[cfg(target_os = "macos")]
use subprocess::{Exec, Redirection};
use tar::Archive;

use crate::{
    commands::{self, install::pip::install_extra_pip_packages},
    os::build_filename,
    utils::{self, SpinnerMessage},
};

#[cfg_attr(windows, allow(dead_code))]
pub fn install_package(
    version_to_install: &Version,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    extract_source(&version_to_install)?;
    compile_source(&version_to_install, install_extra_packages)?;
    Ok(())
}

#[cfg_attr(windows, allow(dead_code))]
pub fn extract_source(version: &Version) -> Result<()> {
    let download_dir = utils::directory::downloaded()?;
    let filename = build_filename(&version)?;
    let file_path = download_dir.join(&filename);
    let extract_dir = utils::directory::extracted()?;

    let line_header = "[2/15] Extract";

    let message = format!("{}ing {:?}...", line_header, file_path);

    let tar_gz = File::open(&file_path)?;

    let (tx, child) = utils::spinner_in_thread(message);

    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extract_dir)?;

    // Send signal to thread to stop
    let message = format!("{}ion of {:?} done.", line_header, file_path);
    tx.send(SpinnerMessage::Message(message))?;
    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| anyhow!("Failed to join threads: {:?}", e))?;

    Ok(())
}

#[cfg_attr(windows, allow(dead_code))]
pub fn compile_source(
    version: &Version,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    // Compilation

    let original_current_dir = env::current_dir()?;

    let install_dir = utils::directory::install_dir(version)?;

    #[cfg_attr(not(macos), allow(unused_mut))]
    let mut configure_args = vec![
        "--prefix".to_string(),
        install_dir
            .to_str()
            .ok_or_else(|| anyhow!("Error converting install dir {:?} to `str`", install_dir))?
            .to_string(),
        "--enable-optimizations".to_string(),
    ];

    #[cfg_attr(not(macos), allow(unused_mut))]
    let mut cflags: Vec<String> = Vec::new();
    #[cfg_attr(not(macos), allow(unused_mut))]
    let mut cppflags: Vec<String> = Vec::new();
    #[cfg_attr(not(macos), allow(unused_mut))]
    let mut ldflags: Vec<String> = Vec::new();

    // See https://devguide.python.org/setup/#macos-and-os-x
    #[cfg(target_os = "macos")]
    {
        // let openssl_prefix = "brew --prefix openssl";
        let openssl_prefix = "/usr/local/opt/openssl";
        if *version >= Version::new(3, 7, 0) {
            let ssl_arg = format!("--with-openssl={}", openssl_prefix);
            configure_args.push(ssl_arg);
        } else {
            cppflags.push(format!("-I{}/include", openssl_prefix));
            ldflags.push(format!("-L{}/lib", openssl_prefix));
        };

        // Make sure compilation can find zlib
        // See https://github.com/pyenv/pyenv/wiki/common-build-problems#build-failed-error-the-python-zlib-extension-was-not-compiled-missing-the-zlib
        let macos_sdk_path = Exec::cmd("xcrun")
            .arg("--show-sdk-path")
            .stdout(Redirection::Pipe)
            .capture()?
            .stdout_str();
        cflags.push(format!("-I{}/usr/include", macos_sdk_path.trim()));

        cppflags.push("-I/opt/X11/include".into());
    }

    env::set_var("CFLAGS", cflags.join(" "));
    env::set_var("CPPFLAGS", cppflags.join(" "));
    env::set_var("LDFLAGS", ldflags.join(" "));

    let basename = utils::build_basename(&version)?;
    let extract_dir = utils::directory::extracted()?.join(&basename);

    utils::run_cmd_template(
        &version,
        "[3/15] Configure",
        "./configure",
        &configure_args,
        &extract_dir,
    )?;
    utils::run_cmd_template::<&str, &PathBuf>(&version, "[4/15] Make", "make", &[], &extract_dir)?;
    utils::run_cmd_template(
        &version,
        "[5/15] Make install",
        "make",
        &["install"],
        &extract_dir,
    )?;

    // Create a file in install directory to detect if we installed it ourselves
    utils::create_info_file(&install_dir, version)?;

    if let Some(install_extra_packages) = install_extra_packages {
        install_extra_pip_packages(&install_dir, &version, install_extra_packages)?;
    }

    // Create symbolic links from binaries with `3` suffix
    let bin_dir = install_dir.join("bin");
    let basenames_to_link = &[
        "easy_install-###",
        "idle###",
        "pip###",
        "pydoc###",
        "python###",
        "python###m",
        "python###m-config",
        "pyvenv-###",
    ];
    let ver_maj_min = format!("{}.{}", version.major, version.minor);
    let ver_maj = format!("{}", version.major);
    env::set_current_dir(&bin_dir)?;
    for basename_to_link in basenames_to_link {
        let basename_src = basename_to_link.replace("###", &ver_maj_min);
        // Create a hard link to the file containing the version (major.minor)
        let basename_dest = basename_to_link.replace("-###", "").replace("###", "");
        if Path::new(&basename_dest).exists() {
            fs::remove_file(&basename_dest)?;
        }
        log::debug!(
            "Creating hard-link from {:?} to {:?}",
            basename_src,
            basename_dest
        );
        match fs::hard_link(&basename_src, &basename_dest) {
            Ok(()) => {}
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => log::warn!(
                    "Source {:?} not found when creating hard link",
                    basename_src
                ),
                _ => return Err(e.into()),
            },
        }
        // Create a hard link to the file containing the major version only
        let basename_dest = basename_to_link
            .replace("-###", &ver_maj)
            .replace("###", &ver_maj);
        utils::create_hard_link(basename_src, basename_dest)?;
    }

    log::debug!(
        "Changing back current directory to {:?}",
        original_current_dir
    );
    env::set_current_dir(&original_current_dir)?;

    Ok(())
}
