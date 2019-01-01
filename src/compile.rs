use std::{fs::File, sync::mpsc::channel, thread, time::Duration};

use failure::format_err;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use semver::Version;
use tar::Archive;

use crate::{utils, Result};

pub fn extract_source(version: &Version) -> Result<()> {
    let download_dir = utils::pycors_download()?;
    let filename = utils::build_filename(&version)?;
    let file_path = download_dir.join(&filename);
    let extract_dir = utils::pycors_extract()?;
    let message = format!("Extracting {:?}...", file_path);
    debug!("{}", message);

    let tar_gz = File::open(file_path)?;

    let (tx, rx) = channel();
    let child = thread::spawn(move || {
        let bar = create_spinner(&message);
        let d = Duration::from_millis(100);

        loop {
            if let Ok(()) = rx.recv_timeout(d) {
                break;
            }
            bar.inc(1);
        }

        bar.finish();
    });

    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extract_dir)?;

    tx.send(())?;

    child
        .join()
        .map_err(|e| format_err!("Failed to join threads: {:?}", e))?;

    debug!("Extraction done.");

    Ok(())
}

fn create_spinner(msg: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();

    bar.set_message(msg);
    bar.set_style(ProgressStyle::default_spinner());

    bar
}
