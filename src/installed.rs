use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use failure::format_err;
use semver::{Version, VersionReq};
use subprocess::{Exec, Redirection};

use crate::{
    constants::TOOLCHAIN_FILE, selected::VersionOrPath, toolchain::get_python_versions_from_path,
    utils, Result, EXECUTABLE_NAME,
};

#[derive(Debug, Clone, failure::Fail)]
#[fail(display = "Python version {} not found!", version)]
pub struct ToolchainNotInstalled {
    version: VersionReq,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstalledToolchain {
    pub location: PathBuf,
    pub version: Version,
}

impl InstalledToolchain {
    pub fn from_path<P>(path: P) -> Option<InstalledToolchain>
    where
        P: AsRef<Path>,
    {
        let versions_found = get_python_versions_from_path(path.as_ref());
        log::debug!("versions_found: {:?}", versions_found);

        let highest_version = versions_found.into_iter().max_by(|x, y| (x.0.cmp(&y.0)))?;
        log::debug!("highest_version: {:?}", highest_version);

        Some(InstalledToolchain {
            version: highest_version.0,
            location: highest_version.1,
        })
    }

    pub fn from_select_file<P>(
        path: P,
        installed_toolchains: &[InstalledToolchain],
    ) -> Result<InstalledToolchain>
    where
        P: AsRef<Path>,
    {
        let select_file = path.as_ref().join(TOOLCHAIN_FILE);
        log::debug!("Reading configuration from file {:?}", select_file);

        let input = File::open(select_file)?;
        let buffered = BufReader::new(input);

        // Read first line only
        let line = match buffered.lines().next() {
            None => return Err(format_err!("File does not even contains a line")),
            Some(line_result) => line_result?,
        };

        let version_or_path: VersionOrPath = line.parse()?;
        match version_or_path {
            VersionOrPath::VersionReq(version_req) => {
                match utils::active_version(&version_req, installed_toolchains) {
                    Some(python_to_use) => Ok(python_to_use.clone()),
                    None => {
                        return Err(ToolchainNotInstalled {
                            version: version_req,
                        }
                        .into());
                    }
                }
            }
            VersionOrPath::Path(path) => InstalledToolchain::from_path(&path)
                .ok_or_else(|| format_err!("No Python interpreter found in {:?}!", path)),
        }
    }

    pub fn is_custom_install(&self) -> bool {
        match self.location.parent() {
            None => {
                log::error!("Cannot get parent directory of {:?}", self.location);
                false
            }
            Some(parent) => parent.join(crate::INFO_FILE).exists(),
        }
    }

    pub fn save_version(&self) -> Result<usize> {
        let version = format!("{}", self.version);
        save(&version, TOOLCHAIN_FILE)
    }

    pub fn save_path(&self) -> Result<usize> {
        let location = format!("{}", self.location.display());
        save(&location, TOOLCHAIN_FILE)
    }
}

fn save<P>(content: &str, path: P) -> Result<usize>
where
    P: AsRef<Path>,
{
    log::debug!("Writing toolchain selection to file {:?}", path.as_ref());

    let mut output = File::create(&path)?;
    let l1 = output.write(content.as_bytes())?;
    let l2 = output.write(b"\n")?;
    Ok(l1 + l2)
}
