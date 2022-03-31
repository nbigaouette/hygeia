use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use question::{Answer, Question};
use semver::Version;

use crate::{
    commands,
    constants::EXECUTABLE_NAME,
    dir_monitor::DirectoryMonitor,
    utils::{self, directory::PycorsPathsProviderFromEnv},
    Result,
};

pub fn install_extra_pip_packages(
    version: &Version,
    install_extra_packages: &commands::InstallExtraPackagesOptions,
) -> Result<()> {
    let install_extra_flag_present = install_extra_packages.install_extra_packages
        || install_extra_packages.install_extra_packages_from.is_some();

    if install_extra_flag_present
        && Answer::YES
            == Question::new("Install extra Python packages using `pip install --upgrade`?")
                .default(Answer::YES)
                .show_defaults()
                .confirm()
    {
        let mut to_pip_installs: Vec<String> = Vec::new();

        let bin_dir = PycorsPathsProviderFromEnv::new().bin_dir(version);
        let mut bin_dir_monitor = DirectoryMonitor::new(&bin_dir)?;

        if install_extra_packages.install_extra_packages {
            to_pip_installs.extend(
                load_extra_packages_to_install_from_file(
                    PycorsPathsProviderFromEnv::new().default_extra_package_file(),
                )?
                .into_iter(),
            );
        }

        if let Some(install_extra_packages_from) =
            &install_extra_packages.install_extra_packages_from
        {
            to_pip_installs.extend(
                load_extra_packages_to_install_from_file(&install_extra_packages_from)?.into_iter(),
            );
        }

        let to_pip_installs: Vec<_> = to_pip_installs
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                if Answer::YES
                    == Question::new(&format!(
                        "    [{:2}/{}] {}",
                        i + 1,
                        to_pip_installs.len(),
                        name
                    ))
                    .default(Answer::YES)
                    .show_defaults()
                    .confirm()
                {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .collect();

        if Answer::YES
            == Question::new(&format!(
                "Selected packages: {}.\nContinue?",
                to_pip_installs.as_slice().join(", ")
            ))
            .default(Answer::YES)
            .show_defaults()
            .confirm()
        {
            let install_dir = PycorsPathsProviderFromEnv::new().bin_dir(version);
            let python_major_bin = install_dir.join(format!(
                "python{}{}{}",
                version.major,
                utils::extension_sep(),
                utils::bin_extension()
            ));
            log::debug!("python_major_bin: {:?}", python_major_bin);
            if let Some(python_major_bin) = python_major_bin.to_str() {
                let env_variables: [(&str, &str); 0] = [];
                for (i, to_pip_install) in to_pip_installs.iter().enumerate() {
                    if let Err(e) = utils::run_cmd_template(
                        version,
                        &format!("[{}/15] pip install --upgrade {}", i + 6, to_pip_install),
                        python_major_bin,
                        &[
                            "-m",
                            "pip",
                            "install",
                            "--verbose",
                            "--upgrade",
                            to_pip_install,
                        ],
                        &env_variables,
                        &install_dir,
                    ) {
                        log::error!("Failed to pip install {}: {:?}", to_pip_install, e);
                    }
                }
            } else {
                log::error!(
                    "Could not get string slice from python path: {:?}",
                    python_major_bin
                );
            }
        }

        let new_bin_files: Vec<_> = bin_dir_monitor.check()?.collect();

        // Create a hard-link for the new bins
        let shim_dir = PycorsPathsProviderFromEnv::new().shims();
        let executable_path = shim_dir.join(EXECUTABLE_NAME);
        for new_bin_file_path in new_bin_files {
            match new_bin_file_path.file_name() {
                Some(new_bin_filename) => {
                    let new_bin_path = shim_dir.join(new_bin_filename);
                    utils::create_hard_link(&executable_path, new_bin_path)?;
                }
                None => {
                    log::error!("Cannot get path's filename part: {:?}", new_bin_file_path);
                }
            }
        }
    }

    Ok(())
}

fn load_extra_packages_to_install_from_file<P>(file: P) -> Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let input = File::open(file.as_ref())?;
    let buffered = BufReader::new(input);

    Ok(buffered
        .lines()
        .filter_map(|line_result| match line_result {
            Ok(line) => Some(line),
            Err(err) => {
                log::error!(
                    "Error reading line from {:?}, ignoring it: {:?}",
                    file.as_ref(),
                    err
                );
                None
            }
        })
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                Some(line.to_string())
            }
        })
        .filter(|line| !line.starts_with('#')) // Ignore comments
        .collect())
}
