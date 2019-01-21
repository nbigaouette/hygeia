use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use failure::format_err;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use semver::Version;
use subprocess::{Exec, Redirection};
use tar::Archive;
use terminal_size::{terminal_size, Width};

use crate::{utils, Result};

pub fn extract_source(version: &Version) -> Result<()> {
    let download_dir = utils::pycors_download()?;
    let filename = utils::build_filename(&version)?;
    let file_path = download_dir.join(&filename);
    let extract_dir = utils::pycors_extract()?;

    let line_header = "[2/15] Extract";

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

    let original_current_dir = env::current_dir()?;

    let install_dir = utils::install_dir(version)?;

    #[allow(unused_mut)]
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
        if *version >= Version::new(3, 7, 0) {
            let ssl_arg = format!("--with-openssl={}", openssl_prefix);
            configure_args.push(ssl_arg);
        } else {
            env::set_var("CPPFLAGS", format!("-I{}/include", openssl_prefix));
            env::set_var("LDFLAGS", format!("-L{}/lib", openssl_prefix));
        };

        // Make sure compilation can find zlib
        // See https://github.com/pyenv/pyenv/wiki/common-build-problems#build-failed-error-the-python-zlib-extension-was-not-compiled-missing-the-zlib
        let macos_sdk_path = Exec::cmd("xcrun")
            .arg("--show-sdk-path")
            .stdout(Redirection::Pipe)
            .capture()?
            .stdout_str();
        env::set_var("CFLAGS", format!("-I{}/usr/include", macos_sdk_path.trim()));
    }

    run_cmd_template(&version, "[3/15] Configure", "./configure", &configure_args)?;
    run_cmd_template::<&str>(&version, "[4/15] Make", "make", &[])?;
    run_cmd_template(&version, "[5/15] Make install", "make", &["install"])?;

    // Install some pip packages
    let to_pip_installs = &[
        "pip",
        "wheel",
        "pipenv",
        "poetry",
        "virtualenv",
        "neovim",
        "autopep8",
        "pylint",
        "black",
        "yapf",
    ];
    let pip = install_dir.join("bin").join("pip");
    if let Some(pip) = pip.to_str() {
        for (i, to_pip_install) in to_pip_installs.iter().enumerate() {
            if let Err(e) = run_cmd_template(
                &version,
                &format!("[{}/15] pip install --upgrade {}", i + 6, to_pip_install),
                pip,
                &[
                    "install",
                    "--verbose",
                    "--upgrade",
                    &format!("--install-option='--prefix={:?}'", install_dir),
                    to_pip_install,
                ],
            ) {
                log::error!("Failed to pip install {}: {:?}", to_pip_install, e);
            }
        }
    } else {
        log::error!("Could not get string slice from pip path: {:?}", pip);
    }

    // Create symbolic links from binaries with `3` suffix
    let bin_dir = install_dir.join("bin");
    let basenames_to_link = &[
        "easy_install-###",
        "idle###",
        "pip###",
        "pydoc###",
        "python###",
        "python###dm",
        "python###dm-config",
        "pyvenv-###",
    ];
    let ver_maj_min = format!("{}.{}", version.major, version.minor);
    let ver_maj = format!("{}", version.major);
    env::set_current_dir(&bin_dir)?;
    for basename_to_link in basenames_to_link {
        let basename_src = basename_to_link.replace("###", &ver_maj_min);
        // Create a hard link to the file containing the version (major.minor)
        let basename_dest = basename_to_link.replace("-###", "").replace("###", "");
        if Path::new(&basename_dest).exists() {
            fs::remove_file(&basename_dest)?;
        }
        log::debug!(
            "Creating hard-link from {:?} to {:?}",
            basename_src,
            basename_dest
        );
        match fs::hard_link(&basename_src, &basename_dest) {
            Ok(()) => {}
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => log::warn!(
                    "Source {:?} not found when creating hard link",
                    basename_src
                ),
                _ => Err(e)?,
            },
        }
        // Create a hard link to the file containing the major version only
        let basename_dest = basename_to_link
            .replace("-###", &ver_maj)
            .replace("###", &ver_maj);
        if Path::new(&basename_dest).exists() {
            fs::remove_file(&basename_dest)?;
        }
        log::debug!(
            "Creating hard-link from {:?} to {:?}",
            basename_src,
            basename_dest
        );
        match fs::hard_link(&basename_src, &basename_dest) {
            Ok(()) => {}
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => log::warn!(
                    "Source {:?} not found when creating hard link",
                    basename_src
                ),
                _ => Err(e)?,
            },
        }
    }

    log::debug!(
        "Changing back current directory to {:?}",
        original_current_dir
    );
    env::set_current_dir(&original_current_dir)?;

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
    let logs_dir = utils::pycors_logs()?;

    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }

    let log_filename = format!(
        "Python_v{}_step_{}.log",
        version,
        line_header
            .replace(" ", "_")
            .replace("[", "")
            .replace("/", "_of_")
            .replace("]", "")
            .replace("-", "")
    );
    let log_filepath = logs_dir.join(&log_filename);
    let mut log_file = BufWriter::new(File::create(log_filepath)?);

    let (tx, child) = spinner_in_thread(line_header.to_string());

    let stream = Exec::cmd(cmd)
        .args(args)
        .cwd(&extract_dir)
        .stderr(Redirection::Merge)
        .stream_stdout()?;

    let br = BufReader::new(stream);

    let message_width = if let Some((Width(width), _)) = terminal_size() {
        // There is two characters before the message: the spinner and a space
        let message_width = (width as usize) - 2;
        Some(message_width)
    } else {
        log::warn!("Unable to get terminal size");
        None
    };

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
                log_line(&line, &mut log_file);
                let message = format!("{}: {}", line_header, line.replace("\t", " "));
                let message = match message_width {
                    None => message,
                    Some(width) => console::truncate_str(&message, width, "...").to_string(),
                };
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

fn log_line<F>(line: &str, log_file: &mut F)
where
    F: Write,
{
    log_file
        .write_all(chrono::Local::now().to_rfc3339().as_bytes())
        .unwrap_or_else(|e| log::error!("Writing to log file failed: {:?}", e));
    log_file
        .write_all(b" - ")
        .unwrap_or_else(|e| log::error!("Writing to log file failed: {:?}", e));
    log_file
        .write_all(line.as_bytes())
        .unwrap_or_else(|e| log::error!("Writing to log file failed: {:?}", e));
    log_file
        .write_all(b"\n")
        .unwrap_or_else(|e| log::error!("Writing to log file failed: {:?}", e));
}

fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();

    pb.set_message(msg);
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}"));

    pb
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
        let pb = create_spinner(&message);
        let d = Duration::from_millis(100);

        loop {
            if let Ok(msg) = rx.recv_timeout(d) {
                match msg {
                    SpinnerMessage::Stop => break,
                    SpinnerMessage::Message(message) => pb.set_message(&message),
                }
            }
            pb.inc(1);
        }

        pb.finish();
    });

    (tx, child)
}

enum SpinnerMessage {
    Stop,
    Message(String),
}
