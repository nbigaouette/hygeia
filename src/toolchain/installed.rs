use std::{
    fmt,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;
use semver::{Version, VersionReq};
use thiserror::Error;

use crate::{
    constants::TOOLCHAIN_FILE,
    toolchain::{self, get_python_versions_from_path},
    utils::directory::PycorsPathsProviderFromEnv,
};

#[derive(Debug, Clone, Error)]
#[error("Python version {version} not found!")]
pub struct ToolchainNotInstalled {
    version: VersionReq,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstalledToolchain {
    pub location: PathBuf,
    pub version: Version,
}

impl fmt::Display for InstalledToolchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Python {} ({})", self.version, self.location.display())
    }
}

#[derive(Debug, PartialEq)]
pub struct NotInstalledToolchain {
    pub version: Option<VersionReq>,
    pub location: Option<PathBuf>,
}

impl InstalledToolchain {
    pub fn from_path<P>(path: P) -> Option<InstalledToolchain>
    where
        P: AsRef<Path>,
    {
        let paths_provider = PycorsPathsProviderFromEnv::new();

        let versions_found = get_python_versions_from_path(path.as_ref(), &paths_provider);
        log::debug!("versions_found: {:?}", versions_found);

        let highest_version = versions_found.into_iter().max_by(|x, y| (x.0.cmp(&y.0)))?;
        log::debug!("highest_version: {:?}", highest_version);

        Some(InstalledToolchain {
            version: highest_version.0,
            location: highest_version.1,
        })
    }

    pub fn is_custom_install(&self) -> bool {
        toolchain::is_a_custom_install(&self.location)
    }

    pub fn save_version(&self) -> Result<usize> {
        let version = format!("={}", self.version);
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
