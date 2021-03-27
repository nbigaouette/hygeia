use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Write},
};

use anyhow::Context;

use crate::{
    cache::AvailableToolchain,
    commands::{self, install::pip::install_extra_pip_packages},
    download::{download_to_path, HyperDownloader},
    utils::{self, directory::PycorsPathsProviderFromEnv},
    Result,
};

const GET_PIP_URL: &str = "https://bootstrap.pypa.io/get-pip.py";

#[cfg_attr(not(windows), allow(dead_code))]
pub fn install_package(
    available_toolchain: &AvailableToolchain,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    let version = &available_toolchain.version;
    let install_dir = PycorsPathsProviderFromEnv::new().install_dir(version);

    let cwd = PycorsPathsProviderFromEnv::new().downloaded();
    let archive = available_toolchain.win_pre_built.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Installing a Windows pre-built requires a prebuilt archive being available"
        )
    })?;
    let archive_path = cwd.join(archive);

    let file = BufReader::new(
        File::open(&archive_path)
            .with_context(|| format!("Failed to open archive {:?}", archive_path))?,
    );
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to open archive as zip file: {:?}", archive_path))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).with_context(|| {
            format!(
                "Failed to access file at index {} in zip archive {:?}",
                i, archive_path
            )
        })?;

        // sanitized_name() is deprecated in zip since 0.5.7 but its replacement is
        // not ready yet.
        // https://github.com/zip-rs/zip/commit/d92a06adec90ec3f1d92dfc071280d84008dce78
        #[allow(deprecated)]
        let filename = file.sanitized_name();

        if (&*file.name()).ends_with('/') {
            let outpath = install_dir.join(&filename);
            log::debug!("{:?} --> \"{}\"", filename, outpath.as_path().display());
            fs::create_dir_all(&outpath)
                .with_context(|| format!("Failed to create directory {:?}", outpath))?;
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
                    fs::create_dir_all(&p)
                        .with_context(|| format!("Failed to create directory {:?}", p))?;
                }
            }
            let mut outfile_file = File::create(&outpath)
                .with_context(|| format!("Failed to create file {:?}", outpath))?;
            io::copy(&mut file, &mut outfile_file).with_context(|| {
                format!(
                    "Failed to extract file {:?} from zip archive into {:?}",
                    filename, outpath
                )
            })?;
        }
    }

    // Create a file in install directory to detect if we installed it ourselves
    utils::create_info_file(&install_dir, version)?;

    let python_major_exe = install_dir.join(format!("python{}.exe", version.major));
    let python_exe = install_dir.join("python.exe");

    // Install pip
    let cache_dir = PycorsPathsProviderFromEnv::new().cache();
    let get_pip_py = cache_dir.join("get-pip.py");
    // File is too small to bother for a progress bar
    let with_progress_bar = false;
    let mut downloader = HyperDownloader::new(GET_PIP_URL)?;
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(download_to_path(
        &mut downloader,
        &cache_dir,
        with_progress_bar,
    ))?;

    let env_variables: [(&str, &str); 0] = [];
    utils::run_cmd_template(
        &version,
        "Install pip",
        &python_exe.to_string_lossy().into_owned(),
        &[
            get_pip_py.to_string_lossy().into_owned(),
            "--no-warn-script-location".to_string(),
        ],
        &env_variables,
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
