use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, Write},
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

const MAX_LINE_LENGTH: usize = 110;

pub fn extract_source(version: &Version) -> Result<()> {
    let download_dir = utils::pycors_download()?;
    let filename = utils::build_filename(&version)?;
    let file_path = download_dir.join(&filename);
    let extract_dir = utils::pycors_extract()?;

    let line_header = "[2/5] Extract";

    let message = format!("{}ing {:?}...", line_header, file_path);

    let tar_gz = File::open(&file_path)?;

    let (tx, child) = spinner_in_thread(message);

    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extract_dir)?;

    // Send signal to thread to stop
    let message = format!("{}ion of {:?} done.", line_header, file_path);
    tx.send(SpinnerMessage::Message(message))?;
    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| format_err!("Failed to join threads: {:?}", e))?;

    Ok(())
}

pub fn compile_source(version: &Version) -> Result<()> {
    // Compilation

    let install_dir = utils::install_dir(version)?;

    let mut configure_args = vec![
        "--prefix".to_string(),
        install_dir
            .to_str()
            .ok_or_else(|| format_err!("Error converting install dir {:?} to `str`", install_dir))?
            .to_string(),
        "--with-pydebug".to_string(),
    ];

    // See https://devguide.python.org/setup/#macos-and-os-x
    #[cfg(target_os = "macos")]
    {
        // let openssl_prefix = "brew --prefix openssl";
        let openssl_prefix = "/usr/local/opt/openssl";
        if version >= &Version::new(3, 7, 0) {
            let ssl_arg = format!("--with-openssl={}", openssl_prefix);
            configure_args.push(ssl_arg);
        } else {
            env::set_var("CPPFLAGS", format!("-I{}/include", openssl_prefix));
            env::set_var("LDFLAGS", format!("-L{}/lib", openssl_prefix));
        };
    }

    run_cmd_template(&version, "[3/5] Configure", "./configure", &configure_args)?;
    run_cmd_template::<&str>(&version, "[4/5] Make", "make", &[])?;
    run_cmd_template(&version, "[5/5] Make install", "make", &["install"])?;

    // Create a file containing the version so the folder can be reloaded in a `Settings`
    let version_file_path = install_dir.join("version");
    let version_str = format!("{}", version);
    debug!(
        "Saving version {} to {}",
        version,
        version_file_path.display()
    );
    let mut output = File::create(&version_file_path)?;
    output.write(version_str.as_bytes())?;

    Ok(())
}

fn run_cmd_template<S: AsRef<std::ffi::OsStr>>(
    version: &Version,
    line_header: &str,
    cmd: &str,
    args: &[S],
) -> Result<()> {
    let basename = utils::build_basename(&version)?;
    let extract_dir = utils::pycors_extract()?.join(&basename);

    let (tx, child) = spinner_in_thread(line_header.to_string());

    let stream = Exec::cmd(cmd)
        .args(args)
        .cwd(&extract_dir)
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
            Ok(mut line) => {
                // FIXME: Save to log file
                line.truncate(MAX_LINE_LENGTH);
                let message = format!("{}: {}", line_header, line);
                tx.send(SpinnerMessage::Message(message))?
            }
        };
    }

    // Send signal to thread to stop
    let message = format!("{} done.", line_header);
    tx.send(SpinnerMessage::Message(message))?;
    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| format_err!("Failed to join threads: {:?}", e))?;

    Ok(())
}

fn create_spinner(msg: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();

    bar.set_message(msg);
    bar.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}"));

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
