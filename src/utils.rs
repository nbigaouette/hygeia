use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use semver::{Version, VersionReq};
use terminal_size::{terminal_size, Width};

use crate::{os, toolchain::installed::InstalledToolchain};

pub mod directory;

use directory::{PycorsPaths, PycorsPathsFromEnv};

pub fn path_exists<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path).is_ok()
}

pub fn copy_file<P1: AsRef<Path>, P2: AsRef<Path>>(from: P1, to: P2) -> Result<u64> {
    if from.as_ref() == to.as_ref() {
        Err(anyhow!(
            "Will not copy {:?} unto {:?} as this would probably truncate it.",
            from.as_ref(),
            to.as_ref()
        ))
    } else {
        let number_of_bytes_copied = fs::copy(from, to)?;
        Ok(number_of_bytes_copied)
    }
}

#[cfg(windows)]
pub fn bin_extension() -> &'static str {
    "exe"
}

#[cfg(not(windows))]
pub fn bin_extension() -> &'static str {
    ""
}

#[cfg(windows)]
pub fn extension_sep() -> &'static str {
    "."
}

#[cfg(not(windows))]
pub fn extension_sep() -> &'static str {
    ""
}

pub fn build_basename(version: &Version) -> Result<String> {
    // Starting with 3.3, the filename contains the full MAJOR.MINOR.PATCH-RC (f.e. "3.3.0" or "3.7.2-rc1").
    // Before that, the filename only contained MAJOR.MINOR (without the patch, for example "3.2")
    // See for example the difference between those versions:
    //      https://www.python.org/ftp/python/3.2
    //      https://www.python.org/ftp/python/3.3.0
    // let version_string =
    let version_file = if *version >= Version::new(3, 3, 0) {
        format!("{}", version)
    } else {
        format!("{}.{}", version.major, version.minor)
    }
    .replace("-", "");

    Ok(format!("Python-{}", version_file))
}

pub fn create_hard_link<P1, P2>(from: P1, to: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let from = from.as_ref();
    let to = to.as_ref();
    if Path::new(&to).exists() {
        fs::remove_file(&to)?;
    }
    log::debug!("Creating hard-link from {:?} to {:?}", from, to);
    match fs::hard_link(&from, &to) {
        Ok(()) => Ok(()),
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => {
                log::warn!("Source {:?} not found when creating hard link", from);
                Ok(())
            }
            _ => Err(e.into()),
        },
    }
}

pub fn create_hard_links<S, P1, P2>(
    copy_from: P1,
    new_files: &[S],
    in_dir: P2,
    replace_sharps_with: &str,
) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
    P1: AsRef<Path>,
    P2: Into<PathBuf>,
{
    let in_dir = in_dir.into();
    for new_file in new_files {
        let filename_str: &str = new_file.as_ref();
        let filename_string = filename_str.to_string().replace("###", replace_sharps_with);
        let new_file = Path::new(&filename_string);
        let new_path = in_dir.join(new_file);
        if new_path.exists() {
            fs::remove_file(&new_path)?;
        }
        log::debug!(
            "Creating hard link from {:?} to {:?}...",
            copy_from.as_ref(),
            new_path
        );
        fs::hard_link(copy_from.as_ref(), &new_path)?;
    }

    Ok(())
}

pub fn active_version<'a>(
    version: &VersionReq,
    installed_toolchains: &'a [InstalledToolchain],
) -> Option<&'a InstalledToolchain> {
    // Find the compatible versions from the installed list
    let mut compatible_versions: Vec<&'a InstalledToolchain> = installed_toolchains
        .iter()
        .filter(|installed_python| version.matches(&installed_python.version))
        .collect();
    // Sort to get latest version. If two versions are identical, pick the
    // one that is custom installed (not a system one).
    compatible_versions.sort_unstable_by(|a, b| {
        let version_comparison = a.version.cmp(&b.version);
        if version_comparison == std::cmp::Ordering::Equal {
            if a.is_custom_install() {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        } else {
            version_comparison
        }
    });
    log::debug!("Compatible versions found: {:?}", compatible_versions);

    compatible_versions.last().cloned()
}

pub fn get_info_file<P>(install_dir: P) -> PathBuf
where
    P: AsRef<Path>,
{
    install_dir.as_ref().join(crate::INFO_FILE)
}

pub fn create_info_file<P>(install_dir: P, version: &Version) -> Result<()>
where
    P: AsRef<Path>,
{
    let filename = get_info_file(install_dir);
    let mut file = fs::File::create(&filename)?;
    writeln!(
        file,
        "Python {} installed using {} version {} on {}.\n",
        version,
        crate::EXECUTABLE_NAME,
        crate::git_version(),
        chrono::Local::now().to_rfc3339()
    )?;

    Ok(())
}

/// Wrapper around `std::process::Child`
///
/// NOTE: According to [](https://doc.rust-lang.org/std/process/struct.Child.html),
/// `std::process::Child` does not implements `Drop`. Since we use `?` for earlier
/// return, let's make sure the child process is killed by implementing `Drop`.
struct ChildProcess(std::process::Child);

impl Drop for ChildProcess {
    fn drop(&mut self) {
        match self.0.try_wait() {
            Ok(Some(_status)) => {}
            Ok(None) => {
                log::error!("Killing child process (pid: {})", self.0.id());
                match self.0.kill() {
                    Ok(()) => {}
                    Err(e) => {
                        log::error!("An error occurred while killing child process: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("An error occurred while waiting for child process: {:?}", e);
            }
        }
    }
}

pub fn run_cmd_template<S, P>(
    version: &Version,
    line_header: &str,
    cmd: &str,
    args: &[S],
    cwd: P,
) -> Result<()>
where
    S: AsRef<std::ffi::OsStr> + std::fmt::Debug,
    P: AsRef<Path>,
{
    let logs_dir = PycorsPathsFromEnv::new().logs();

    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }

    let log_filename = format!(
        "Python_v{}_step_{}.log",
        version,
        line_header
            .replace(":", "_")
            .replace(" ", "_")
            .replace("[", "")
            .replace("/", "_of_")
            .replace("]", "")
            .replace("-", "")
    );
    let log_filepath = logs_dir.join(&log_filename);
    let mut log_file = BufWriter::new(File::create(&log_filepath)?);

    log_line(&format!("cd {}", cwd.as_ref().display()), &mut log_file);
    log_line(&format!("{} {:?}", cmd, args), &mut log_file);

    let (tx, child) = spinner_in_thread(line_header.to_string());

    let current_paths: Vec<PathBuf> = match env::var("PATH") {
        Ok(path) => env::split_paths(&path).collect(),
        Err(err) => {
            log::error!("Failed to get environment variable PATH: {:?}", err);
            vec![PathBuf::new()]
        }
    };
    let new_paths: Vec<PathBuf> = {
        let mut tmp = os::paths_to_prepends(version)?;
        tmp.extend_from_slice(&current_paths);
        tmp
    };
    let new_path = env::join_paths(new_paths.iter())?;

    // FIXME: Change cwd
    // Wrap in a custom `ChildProcess` that implements `Drop` to kill the child process
    let mut process = ChildProcess(
        std::process::Command::new(cmd)
            .args(args)
            .env("PATH", &new_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?,
    );

    // Extract the stdout from `process`, replacing it with a None.
    let mut stdout: Option<std::process::ChildStdout> = None;
    std::mem::swap(&mut process.0.stdout, &mut stdout);

    let br = BufReader::new(stdout.ok_or_else(|| anyhow!("Got none"))?);

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
                return Err(anyhow!("Error reading stdout: {:?}", e));
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

    // We've read all process output. Wait for the process to finish and
    // get exit code.
    let exit_status = process.0.wait()?;

    // Send signal to thread to stop
    let message = format!("{} done.", line_header);
    tx.send(SpinnerMessage::Message(message))?;
    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| anyhow!("Failed to join threads: {:?}", e))?;

    if exit_status.success() {
        log::debug!("Success!");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to execute command (exit code: {:?}): {} {}\nPATH: \"{}\"",
            exit_status.code(),
            cmd,
            args.iter()
                .map(|s| s.as_ref().to_string_lossy().to_string())
                .collect::<Vec<String>>()
                .join(" "),
            new_path.to_string_lossy()
        ))
    }
}

pub fn log_line<F>(line: &str, log_file: &mut F)
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

pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();

    pb.set_message(msg);
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}"));

    pb
}

pub fn spinner_in_thread<S: Into<String>>(
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

pub enum SpinnerMessage {
    Stop,
    Message(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_exists_success() {
        assert!(path_exists("target"));
    }

    #[test]
    fn path_exists_fail() {
        assert!(!path_exists("non-existing-directory"));
    }

    #[test]
    fn copy_file_success() {
        let copied_file_location = env::temp_dir().join("dummy_copied_file");
        let _ = fs::remove_file(&copied_file_location);
        assert!(!copied_file_location.exists());
        let nb_bytes_copied = copy_file("LICENSE-APACHE", &copied_file_location).unwrap();
        // On Azure Pipelines, the Windows build reports `11039` bytes copied, not 10838.
        // See https://dev.azure.com/nbigaouette/pycors/_build/results?buildId=13
        #[cfg(target_os = "windows")]
        {
            if nb_bytes_copied != 10838 {
                eprintln!(
                    "WARNING: Number of bytes copied: {}, expecting 10838",
                    nb_bytes_copied
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(nb_bytes_copied, 10838);
        }
        assert!(copied_file_location.exists());
        let _ = fs::remove_file(&copied_file_location);
    }

    #[test]
    fn copy_file_overwrite() {
        copy_file("LICENSE-APACHE", "LICENSE-APACHE").unwrap_err();
    }

    #[test]
    fn build_basename_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();

        let filename = build_basename(&version).unwrap();

        assert_eq!(&filename, "Python-3.7.2");
    }

    #[test]
    fn build_basename_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let filename = build_basename(&version).unwrap();
        assert_eq!(&filename, "Python-3.7.2rc1");
    }

    #[test]
    #[ignore]
    fn create_hard_links_success() {
        let in_dir = env::current_dir().unwrap().join("target");
        let hardlinks_location = &[
            in_dir
                .join("dummy_hardlink_1-###")
                .to_str()
                .unwrap()
                .to_string(),
            in_dir
                .join("dummy_hardlink_2-###")
                .to_str()
                .unwrap()
                .to_string(),
        ];
        for hardlink_location in hardlinks_location {
            let _ = fs::remove_file(hardlink_location);
        }
        for hardlink_location in hardlinks_location {
            assert!(!Path::new(hardlink_location).exists());
        }
        create_hard_links("LICENSE-APACHE", hardlinks_location, &in_dir, "replaced").unwrap();
        for hardlink_location in hardlinks_location {
            assert!(Path::new(&hardlink_location.replace("###", "replaced")).exists());
        }
        for hardlink_location in hardlinks_location {
            assert!(Path::new(&hardlink_location.replace("###", "replaced")).exists());
            let _ = fs::remove_file(&hardlink_location.replace("###", "replaced"));
        }
    }
}
