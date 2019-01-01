use std::fs::File;

use flate2::read::GzDecoder;
// use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use semver::Version;
use tar::Archive;

use crate::{utils, Result};

pub fn extract_source(version: &Version) -> Result<()> {
    // let cache_dir = utils::pycors_cache()?;
    let download_dir = utils::pycors_download()?;
    let filename = utils::build_filename(&version)?;
    let file_path = download_dir.join(&filename);
    let extract_dir = utils::pycors_extract()?;
    debug!("Extracting {:?}...", file_path);

    // let mut bar = create_progress_bar(&format!("Extracting {:?}...", file_path));

    let tar_gz = File::open(file_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extract_dir)?;

    debug!("Extraction done.");

    // bar.finish();

    unimplemented!()
}

// fn create_progress_bar(msg: &str) -> ProgressBar {
//     let bar = ProgressBar::new_spinner();

//     bar.set_message(msg);
//     bar.set_style(ProgressStyle::default_spinner());

//     bar
// }
