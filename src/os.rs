use std::path::PathBuf;

use semver::Version;

use crate::{utils::directory::PycorsPathsProviderFromEnv, Result};

pub fn paths_to_prepends(version: &Version) -> Result<Vec<PathBuf>> {
    let bin_dir = PycorsPathsProviderFromEnv::new().bin_dir(version);

    let mut paths = Vec::new();

    #[allow(clippy::redundant_clone)]
    paths.push(bin_dir.clone());

    #[cfg(windows)]
    {
        paths.push(bin_dir.join("Scripts"));
    }

    Ok(paths)
}
