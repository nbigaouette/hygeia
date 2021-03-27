use std::path::PathBuf;

use semver::Version;

use crate::{utils::directory::PycorsPathsProviderFromEnv, Result};

pub fn paths_to_prepends(version: &Version) -> Result<Vec<PathBuf>> {
    let bin_dir = PycorsPathsProviderFromEnv::new().bin_dir(version);

    #[allow(clippy::redundant_clone)]
    #[allow(unused_mut)]
    let mut paths = vec![bin_dir.clone()];

    #[cfg(windows)]
    {
        paths.push(bin_dir.join("Scripts"));
    }

    Ok(paths)
}
