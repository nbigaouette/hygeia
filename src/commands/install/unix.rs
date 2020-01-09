use std::{
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use anyhow::Context;
use flate2::read::GzDecoder;
use semver::Version;
use tar::Archive;

use crate::{
    commands::{self, install::pip::install_extra_pip_packages},
    os::build_filename,
    utils::{self, directory::PycorsPathsProviderFromEnv, SpinnerMessage},
    Result,
};

#[cfg_attr(windows, allow(dead_code))]
pub fn install_package(
    release: bool,
    version_to_install: &Version,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    extract_source(&version_to_install).with_context(|| "Failed to extract source")?;
    compile_source(release, &version_to_install, install_extra_packages)
        .with_context(|| "Failed to compile source")?;
    Ok(())
}

#[cfg_attr(windows, allow(dead_code))]
pub fn extract_source(version: &Version) -> Result<()> {
    let download_dir = PycorsPathsProviderFromEnv::new().downloaded();
    let filename = build_filename(&version);
    let file_path = download_dir.join(&filename);
    let extract_dir = PycorsPathsProviderFromEnv::new().extracted();

    let line_header = "[2/15] Extract";

    let message = format!("{}ing {:?}...", line_header, file_path);

    let tar_gz =
        File::open(&file_path).with_context(|| format!("Failed to open file {:?}", file_path))?;

    let (tx, child) = utils::spinner_in_thread(message);

    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive
        .unpack(extract_dir)
        .with_context(|| format!("Failed to unpack archive {:?}", file_path))?;

    // Send signal to thread to stop
    let message = format!("{}ion of {:?} done.", line_header, file_path);
    tx.send(SpinnerMessage::Message(message))?;
    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| anyhow::anyhow!("Failed to join threads: {:?}", e))?;

    Ok(())
}

#[cfg_attr(windows, allow(dead_code))]
pub fn compile_source(
    release: bool,
    version: &Version,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    // Compilation

    let install_dir = PycorsPathsProviderFromEnv::new().install_dir(version);

    let mut configure_args = vec![
        "--prefix".to_string(),
        install_dir
            .to_str()
            .ok_or_else(|| {
                anyhow::anyhow!("Error converting install dir {:?} to `str`", install_dir)
            })?
            .to_string(),
        "--enable-shared".to_string(),
    ];
    if release {
        configure_args.push("--enable-optimizations".to_string());
    }

    #[cfg_attr(not(macos), allow(unused_mut))]
    let mut cflags: Vec<String> = Vec::new();
    #[cfg_attr(not(macos), allow(unused_mut))]
    let mut cppflags: Vec<String> = Vec::new();
    #[cfg_attr(not(macos), allow(unused_mut))]
    let mut ldflags: Vec<String> = Vec::new();

    #[cfg(target_os = "linux")]
    {
        // When compiling dynamically (--enable-shared), we need to
        // add a directory to the runtime library search path.
        // See https://stackoverflow.com/questions/37757314/problems-installing-python-3-with-enable-shared
        ldflags.push(format!("-Wl,-rpath {}", install_dir.display()));
    }

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
        let macos_sdk_path = String::from_utf8(
            std::process::Command::new("xcrun")
                .arg("--show-sdk-path")
                .output()
                .with_context(|| "Failed to execute 'xcrun --show-sdk-path'")?
                .stdout,
        )
        .with_context(|| "Failed to run command 'xrun' to find macOS SDK path")?;
        cflags.push(format!("-I{}/usr/include", macos_sdk_path.trim()));

        cppflags.push("-I/opt/X11/include".into());
    }

    env::set_var("CFLAGS", cflags.join(" "));
    env::set_var("CPPFLAGS", cppflags.join(" "));
    env::set_var("LDFLAGS", ldflags.join(" "));

    let basename = utils::build_basename(&version);
    let extract_dir = PycorsPathsProviderFromEnv::new()
        .extracted()
        .join(&basename);

    utils::run_cmd_template(
        &version,
        "[3/15] Configure",
        "./configure",
        &configure_args,
        &extract_dir,
    )
    .with_context(|| format!("Failed to run command ./configure {:?}", configure_args))?;
    utils::run_cmd_template::<&str, &PathBuf>(&version, "[4/15] Make", "make", &[], &extract_dir)
        .with_context(|| "Failed to run command 'make'")?;
    utils::run_cmd_template(
        &version,
        "[5/15] Make install",
        "make",
        &["install"],
        &extract_dir,
    )
    .with_context(|| "Failed to run command 'make install'")?;

    // Create a file in install directory to detect if we installed it ourselves
    utils::create_info_file(&install_dir, version).with_context(|| {
        format!(
            "Failed create info file for version {} in {:?}",
            version, install_dir
        )
    })?;

    if let Some(install_extra_packages) = install_extra_packages {
        install_extra_pip_packages(&version, install_extra_packages)
            .with_context(|| "Failed to install extra pip packages")?;
    }

    // Create symbolic links from binaries with `3` suffix
    let bin_dir = PycorsPathsProviderFromEnv::new().bin_dir(&version);
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
    let original_current_dir =
        env::current_dir().with_context(|| "Failed to get current working directory")?;
    env::set_current_dir(&bin_dir)
        .with_context(|| format!("Failed to set current working directory to {:?}", bin_dir))?;
    for basename_to_link in basenames_to_link {
        let basename_src = basename_to_link.replace("###", &ver_maj_min);
        // Create a hard link to the file containing the version (major.minor)
        let basename_dest = basename_to_link.replace("-###", "").replace("###", "");
        if Path::new(&basename_dest).exists() {
            fs::remove_file(&basename_dest).with_context(|| {
                format!("Failed to delete previous hard link {:?}", basename_dest)
            })?;
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
        utils::create_hard_link(&basename_src, &basename_dest).with_context(|| {
            format!(
                "Failed to create hard link {:?} pointing to {:?}",
                basename_dest, basename_src
            )
        })?;
    }

    log::debug!(
        "Changing back current directory to {:?}",
        original_current_dir
    );
    env::set_current_dir(&original_current_dir).with_context(|| {
        format!(
            "Failed to set current working directory back to original {:?}",
            original_current_dir
        )
    })?;

    Ok(())
}
