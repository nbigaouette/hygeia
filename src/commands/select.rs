use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use failure::format_err;
use semver::VersionReq;

use crate::{commands, installed::InstalledToolchain, selected::SelectedVersion, utils, Result};

enum VersionOrPath {
    VersionReq(semver::VersionReq),
    Path(PathBuf),
}

impl FromStr for VersionOrPath {
    type Err = std::io::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // One can use 'latest' to mean '*'
        if s == "latest" {
            "*"
        } else {
            s
        };

        match semver::VersionReq::parse(s) {
            Ok(version_req) => {
                log::info!("Parsed {:?} as semantic version : {}", s, version_req);
                Ok(VersionOrPath::VersionReq(version_req))
            }
            Err(e) => {
                log::debug!("e: {:?}", e);
                let path = Path::new(s);
                log::info!("Parsed {:?} as Path: {:?}", s, path);
                if path.exists() {
                    Ok(VersionOrPath::Path(path.to_path_buf()))
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Path {:?} not found", s),
                    ))
                }
            }
        }
    }
}

pub fn run(
    requested_version: commands::VersionOrPath,
    installed_toolchains: &[InstalledToolchain],
) -> Result<()> {
    log::debug!("Requested version: {:?}", requested_version);

    let version_or_path: VersionOrPath = requested_version.version_or_path.parse()?;

    let python_to_use = match version_or_path {
        VersionOrPath::VersionReq(version_req) => {
            match utils::active_version(&version_req, installed_toolchains) {
                Some(python_to_use) => python_to_use.clone(),
                None => {
                    return Err(format_err!(
                        "Python version {} not found!",
                        requested_version.version_or_path
                    ));
                }
            }
        }
        VersionOrPath::Path(path) => match InstalledToolchain::from_path(&path) {
            Some(python_to_use) => python_to_use,
            None => {
                return Err(format_err!(
                    "Could not find a Python interpreter under {:?}",
                    path
                ));
            }
        },
    };

    log::debug!(
        "Using {} from {}",
        python_to_use.version,
        python_to_use.location.display()
    );

    // Write to `.python-version`
    SelectedVersion {
        version: VersionReq::exact(&python_to_use.version),
    }
    .save()?;

    Ok(())
}
