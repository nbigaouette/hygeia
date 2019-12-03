use std::{
    env,
    fs::{create_dir_all, File},
    io::{self, BufReader},
};

use semver::Version;

use crate::{
    commands::{self, install::pip::install_extra_pip_packages},
    os::windows::build_filename_zip,
    utils, Result,
};

#[cfg_attr(not(windows), allow(dead_code))]
pub fn install_package(
    version: &Version,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    let original_current_dir = env::current_dir()?;

    let install_dir = utils::directory::install_dir(version)?;

    let cwd = utils::directory::downloaded()?;
    let archive = build_filename_zip(version)?;

    let file = BufReader::new(File::open(cwd.join(archive)).unwrap());
    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let filename = file.sanitized_name();

        if (&*file.name()).ends_with('/') {
            let outpath = install_dir.join(&filename);
            log::debug!("{:?} --> \"{}\"", filename, outpath.as_path().display());
            create_dir_all(&outpath).unwrap();
        } else {
            let outpath = install_dir.join(&filename);
            log::debug!(
                "{:?} --> \"{}\" ({} bytes)",
                filename,
                outpath.as_path().display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    // Create a file in install directory to detect if we installed it ourselves
    utils::create_info_file(&install_dir, version)?;

    if let Some(install_extra_packages) = install_extra_packages {
        install_extra_pip_packages(&install_dir, &version, install_extra_packages)?;
    }

    log::debug!(
        "Changing back current directory to {:?}",
        original_current_dir
    );
    env::set_current_dir(&original_current_dir)?;

    Ok(())
}
