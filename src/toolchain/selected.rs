use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use failure::format_err;
use semver::VersionReq;

use crate::{constants::TOOLCHAIN_FILE, toolchain::installed::InstalledToolchain, utils, Result};

#[derive(Debug, PartialEq)]
pub enum VersionOrPath {
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
                log::info!("Parsed {:?} as semantic version: {}", s, version_req);
                Ok(VersionOrPath::VersionReq(version_req))
            }
            Err(e) => {
                log::debug!("e: {:?}", e);
                let path = Path::new(s);
                log::info!("Parsed {:?} as Path: {:?}", s, path);
                if path.exists() {
                    Ok(VersionOrPath::Path(
                        path.canonicalize().expect("path is expected to exists"),
                    ))
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
