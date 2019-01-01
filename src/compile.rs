use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use failure::format_err;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use semver::Version;
use subprocess::{Exec, Redirection};
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

    let (tx, child) = spinner_in_thread(message);

    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extract_dir)?;

    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| format_err!("Failed to join threads: {:?}", e))?;

    debug!("Extraction done.");

    Ok(())
}

pub fn compile_source(version: &Version) -> Result<()> {
    let message = format!("Compiling {}...", version);

    // Compilation
    configure(&version)?;
    make(&version)?;
    make_install(&version)?;

    unimplemented!()
}

fn configure(version: &Version) -> Result<()> {
    let basename = utils::build_basename(&version)?;
    let extract_dir = utils::pycors_extract()?.join(&basename);
    let install_dir = utils::install_dir(version)?;

    env::set_current_dir(&extract_dir)?;

    let line_header = "[3/5] Configure:";

    let (tx, child) = spinner_in_thread("./configure");

    let stream = Exec::cmd("./configure")
        .arg("--prefix")
        .arg(install_dir)
        .stderr(Redirection::Merge)
        .stream_stdout()?;

    let br = BufReader::new(stream);

    for line in br.lines() {
        match line {
            Err(e) => {
                tx.send(SpinnerMessage::Message(format!(
                    "Error reading stdout: {:?}",
                    e
                )))?;
                tx.send(SpinnerMessage::Stop)?;
                return Err(format_err!("Error reading stdout: {:?}", e));
            }
            Ok(line) => {
                // FIXME: Save to log file
                let message = format!("{} {}", line_header, line);
                tx.send(SpinnerMessage::Message(message))?
            }
        };
    }

    // Send signal to thread to stop
    let message = format!("{} Done", line_header);
    tx.send(SpinnerMessage::Message(message))?;
    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| format_err!("Failed to join threads: {:?}", e))?;

    Ok(())
}

fn make(version: &Version) -> Result<()> {
    let extract_dir = utils::pycors_extract()?;
    env::set_current_dir(&extract_dir)?;

    unimplemented!()
}

fn make_install(version: &Version) -> Result<()> {
    let extract_dir = utils::pycors_extract()?;
    env::set_current_dir(&extract_dir)?;

    unimplemented!()
}

fn create_spinner(msg: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();

    bar.set_message(msg);
    bar.set_style(ProgressStyle::default_spinner());

    bar
}

fn spinner_in_thread<S: Into<String>>(
    message: S,
) -> (
    std::sync::mpsc::Sender<SpinnerMessage>,
    std::thread::JoinHandle<()>,
) {
    let message = message.into();
    let (tx, rx) = channel();
    let child = thread::spawn(move || {
        let bar = create_spinner(&message);
        let d = Duration::from_millis(100);

        loop {
            match rx.recv_timeout(d) {
                Ok(msg) => match msg {
                    SpinnerMessage::Stop => break,
                    SpinnerMessage::Message(message) => bar.set_message(&message),
                },
                Err(_) => {}
            }
            bar.inc(1);
        }

        bar.finish();
    });

    (tx, child)
}

enum SpinnerMessage {
    Stop,
    Message(String),
}
