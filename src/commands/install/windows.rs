use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Write},
};

use semver::Version;

use crate::{
    commands::{self, install::pip::install_extra_pip_packages},
    download::download_to_path,
    os::windows::build_filename_zip,
    utils::{self, directory::PycorsPathsProviderFromEnv},
    Result,
};

const GET_PIP_URL: &str = "https://bootstrap.pypa.io/get-pip.py";

#[cfg_attr(not(windows), allow(dead_code))]
pub fn install_package(
    version: &Version,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    let install_dir = PycorsPathsProviderFromEnv::new().install_dir(version);

    let cwd = PycorsPathsProviderFromEnv::new().downloaded();
    let archive = build_filename_zip(version);

    let file = BufReader::new(File::open(cwd.join(archive)).unwrap());
    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let filename = file.sanitized_name();

        if (&*file.name()).ends_with('/') {
            let outpath = install_dir.join(&filename);
            log::debug!("{:?} --> \"{}\"", filename, outpath.as_path().display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            let outpath = install_dir.join(&filename);
            log::debug!(
                "Extracting {:?} --> \"{}\" ({} bytes)",
                filename,
                outpath.as_path().display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    // Create a file in install directory to detect if we installed it ourselves
    utils::create_info_file(&install_dir, version)?;

    let python_major_exe = install_dir.join(format!("python{}.exe", version.major));
    let python_exe = install_dir.join("python.exe");

    // Install pip
    let cache_dir = PycorsPathsProviderFromEnv::new().cache();
    let get_pip_py = cache_dir.join("get-pip.py");
    let mut rt = tokio::runtime::Runtime::new()?;
    rt.block_on(download_to_path(GET_PIP_URL, &cache_dir))?;
    utils::run_cmd_template(
        &version,
        "Install pip",
        &python_exe.to_string_lossy().into_owned(),
        &[
            get_pip_py.to_string_lossy().into_owned(),
            "--no-warn-script-location".to_string(),
        ],
        &install_dir,
    )?;

    // Make sure we have a binary 'python<MAJOR>.exe', which the zip file doesn't include
    if !python_major_exe.exists() {
        log::debug!("Copying {:?} to {:?}...", python_exe, python_major_exe);
        fs::copy(python_exe, python_major_exe)?;
    }

    // Make sure we can import pip
    // https://michlstechblog.info/blog/python-install-python-with-pip-on-windows-by-the-embeddable-zip-file/
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(install_dir.join(format!("python{}{}._pth", version.major, version.minor)))?;
    writeln!(
        file,
        "{}",
        install_dir.join("Lib").join("site-packages").display()
    )?;

    if let Some(install_extra_packages) = install_extra_packages {
        install_extra_pip_packages(&version, install_extra_packages)?;
    }

    Ok(())
}
