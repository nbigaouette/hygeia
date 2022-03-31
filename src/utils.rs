use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use anyhow::Context;
use indicatif::{ProgressBar, ProgressStyle};
use semver::{Version, VersionReq};
use terminal_size::{terminal_size, Width};

use crate::{
    constants::{EXECUTABLE_NAME, INFO_FILE},
    os,
    toolchain::installed::InstalledToolchain,
    Result,
};

pub mod directory;

use directory::PycorsPathsProviderFromEnv;

pub fn path_exists<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path).is_ok()
}

pub fn copy_file<P1: AsRef<Path>, P2: AsRef<Path>>(from: P1, to: P2) -> Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    if from == to {
        Err(anyhow::anyhow!(
            "Will not copy {:?} unto itself {:?} as this would probably truncate it.",
            from,
            to
        ))
    } else {
        let number_of_bytes_copied = fs::copy(from, to).with_context(|| {
            format!("Failed to copy {:?} to {:?}", from.display(), to.display())
        })?;
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

pub fn create_hard_link<P1, P2>(from: P1, to: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let from = from.as_ref();
    let to = to.as_ref();
    if Path::new(&to).exists() {
        fs::remove_file(&to).with_context(|| format!("Failed to remove file {:?}", to))?;
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
    install_dir.as_ref().join(INFO_FILE)
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
        EXECUTABLE_NAME,
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

pub fn run_cmd_template<S, P, SEnvName, SEnvValue>(
    version: &Version,
    line_header: &str,
    cmd: &str,
    args: &[S],
    envs: &[(SEnvName, SEnvValue)],
    cwd: P,
) -> Result<()>
where
    S: AsRef<std::ffi::OsStr> + std::fmt::Debug,
    SEnvName: AsRef<std::ffi::OsStr> + std::fmt::Debug,
    SEnvValue: AsRef<std::ffi::OsStr> + std::fmt::Debug,
    P: AsRef<Path>,
{
    log::debug!(
        "Running {} {}",
        cmd,
        args.iter()
            .map(|s| s.as_ref().to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    log::debug!("Using environment variables:");
    envs.iter()
        .map(|(name, value)| {
            format!(
                "{}={}",
                name.as_ref().to_string_lossy(),
                value.as_ref().to_string_lossy()
            )
        })
        .for_each(|s| log::debug!("    {}", s));

    let logs_dir = PycorsPathsProviderFromEnv::new().logs();

    // FIXME: Extract generics part to own function to reduce bloat
    let cwd = cwd.as_ref();

    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)
            .with_context(|| format!("Failed to create directory {:?}", logs_dir))?;
    }

    let log_filename = format!(
        "Python_v{}_step_{}.log",
        version,
        line_header
            .replace(':', "_")
            .replace(' ', "_")
            .replace('[', "")
            .replace('/', "_of_")
            .replace(']', "")
            .replace('-', "")
    );
    let log_filepath = logs_dir.join(&log_filename);
    let mut log_file = BufWriter::new(
        File::create(&log_filepath)
            .with_context(|| format!("Failed to create log file {:?}", log_filepath))?,
    );

    log_line(&format!("cd {}", cwd.display()), &mut log_file);
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
    let new_path = env::join_paths(new_paths.iter()).with_context(|| {
        format!(
            "Failed to create a single string from paths {:?}",
            new_paths
        )
    })?;

    let original_current_dir =
        env::current_dir().with_context(|| "Failed to get current working directory")?;

    env::set_current_dir(&cwd)
        .with_context(|| format!("Failed to set current working directory to {:?}", cwd))?;

    // Wrap in a custom `ChildProcess` that implements `Drop` to kill the child process
    let mut process = ChildProcess(
        std::process::Command::new(cmd)
            .args(args)
            .env("PATH", &new_path)
            .envs(
                envs.iter()
                    .map(|(name, value)| (name.as_ref(), value.as_ref())),
            )
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn command {} {:?}", cmd, args))?,
    );

    // Extract the stdout from `process`, replacing it with a None.
    let mut stdout: Option<std::process::ChildStdout> = None;
    std::mem::swap(&mut process.0.stdout, &mut stdout);

    let br = BufReader::new(stdout.ok_or_else(|| anyhow::anyhow!("Got none"))?);

    let message_width = if let Some((Width(width), _)) = terminal_size() {
        // There is two characters before the message: the spinner and a space
        let terminal_width = (width as usize).saturating_sub(2);
        // If terminal with is less than 5, skip sending the line to the spinner
        if terminal_width < 5 {
            None
        } else {
            Some(terminal_width)
        }
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
                return Err(anyhow::anyhow!("Error reading stdout: {:?}", e));
            }
            Ok(line) => {
                log_line(&line, &mut log_file);
                let message = format!("{}: {}", line_header, line.replace('\t', " "));
                if let Some(width) = message_width {
                    tx.send(SpinnerMessage::Message(
                        console::truncate_str(&message, width, "...").to_string(),
                    ))?;
                }
            }
        };
    }

    // We've read all process output. Wait for the process to finish and
    // get exit code.
    let exit_status = process.0.wait().with_context(|| {
        format!(
            "Failed to wait for child process with command {} {:?}",
            cmd, args
        )
    })?;

    // Send signal to thread to stop
    let message = format!("{} done.", line_header);
    tx.send(SpinnerMessage::Message(message))?;
    tx.send(SpinnerMessage::Stop)?;

    child
        .join()
        .map_err(|e| anyhow::anyhow!("Failed to join threads: {:?}", e))?;

    log::debug!(
        "Changing back current directory to {:?}",
        original_current_dir
    );
    env::set_current_dir(&original_current_dir).with_context(|| {
        format!(
            "Failed to set current working directory back to original {:?}",
            original_current_dir
        )
    })?;

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

    pb.set_message(msg.to_owned());
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
                    SpinnerMessage::Message(message) => pb.set_message(message),
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

    use hygeia_test_helpers::create_test_temp_dir;

    fn fixture_installed_toolchains() -> Vec<InstalledToolchain> {
        vec![
            InstalledToolchain {
                location: PathBuf::from("unimportant"),
                version: Version::new(3, 7, 4),
            },
            InstalledToolchain {
                location: PathBuf::from("unimportant"),
                version: Version::new(3, 7, 5),
            },
            InstalledToolchain {
                location: PathBuf::from("unimportant"),
                version: Version::new(3, 7, 2),
            },
            InstalledToolchain {
                location: PathBuf::from("unimportant"),
                version: Version::new(3, 6, 1),
            },
            InstalledToolchain {
                location: PathBuf::from("unimportant"),
                version: Version::new(3, 8, 0),
            },
        ]
    }

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
        let tmp_dir = create_test_temp_dir!();

        let copied_file_location = tmp_dir.join("dummy_copied_file");
        let _ = fs::remove_file(&copied_file_location);
        assert!(!copied_file_location.exists());
        let nb_bytes_copied = copy_file("LICENSE-APACHE", &copied_file_location).unwrap();
        // On Azure Pipelines, the Windows build reports `11039` bytes copied, not 10838.
        // See https://dev.azure.com/nbigaouette/hygeia/_build/results?buildId=13
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

    #[test]
    fn active_version_empty_list() {
        let version_req = VersionReq::parse("=3.7.5").unwrap();
        let installed_toolchains = vec![];
        let compatible_version = active_version(&version_req, &installed_toolchains);
        assert!(compatible_version.is_none());
    }

    #[test]
    fn active_version_tilde() {
        let version_req = VersionReq::parse("~3.7").unwrap();
        let installed_toolchains = fixture_installed_toolchains();
        let compatible_version = active_version(&version_req, &installed_toolchains).unwrap();
        assert_eq!(compatible_version.version, Version::new(3, 7, 5));
    }

    #[test]
    fn active_version_exact_not_in_list() {
        let version_req = VersionReq::parse("=3.7.3").unwrap();
        let installed_toolchains = fixture_installed_toolchains();
        assert!(active_version(&version_req, &installed_toolchains).is_none());
    }

    #[test]
    fn active_version_found() {
        let version_req = VersionReq::parse("=3.7.2").unwrap();
        let installed_toolchains = fixture_installed_toolchains();
        let compatible_version = active_version(&version_req, &installed_toolchains).unwrap();
        assert_eq!(compatible_version.version, Version::new(3, 7, 2));
    }

    #[test]
    fn get_info_file_success() {
        let dir = Path::new("unimportant");
        let file = get_info_file(&dir);
        let expected = dir.join(INFO_FILE);
        assert_eq!(file, expected);
    }

    #[test]
    fn create_info_file_success() {
        let tmp_dir = create_test_temp_dir!();
        let version = Version::new(3, 7, 5);
        let expected_file_path = tmp_dir.join(INFO_FILE);
        let expected_file_begin = "Python 3.7.5 installed using ";
        assert!(!expected_file_path.exists());
        create_info_file(&tmp_dir, &version).unwrap();
        assert!(expected_file_path.exists());
        let file_content = fs::read_to_string(&expected_file_path).unwrap();
        assert!(file_content.starts_with(&expected_file_begin));
    }

    #[test]
    fn run_cmd_template_success() {
        let version = Version::new(0, 0, 0);
        let line_header = "0 utils::tests::run_cmd_template_success";
        let cmd = "cargo";
        let args = &["-V"];
        let envs: &[(&str, String)] = &[];
        let cwd = ".";

        let expected_file_path = PycorsPathsProviderFromEnv::new()
            .logs()
            .join("Python_v0.0.0_step_0_utils__tests__run_cmd_template_success.log");
        if expected_file_path.exists() {
            fs::remove_file(&expected_file_path).unwrap();
        }

        println!("expected_file_path: {:?}", expected_file_path);
        run_cmd_template(&version, line_header, cmd, args, envs, cwd).unwrap();

        let re = regex::Regex::new(r#"(?P<date>20[0-9]{2}-[0-1][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-5][0-9].[0-9]+[-+][0-2][0-9]:[0-5][0-9]) - (?P<cmd>.*)"#).unwrap();
        let file_content = fs::read_to_string(&expected_file_path).unwrap();
        println!("file_content:\n{}", file_content);

        let lines: Vec<&str> = file_content.lines().collect();

        let caps = re.captures(lines[0]).unwrap();
        let _date = &caps["date"];
        assert_eq!(&caps["cmd"], "cd .");

        let caps = re.captures(lines[1]).unwrap();
        let _date = &caps["date"];
        assert_eq!(&caps["cmd"], r#"cargo ["-V"]"#);

        let caps = re.captures(lines[2]).unwrap();
        let _date = &caps["date"];
        assert!(&caps["cmd"].starts_with("cargo 1."));
    }

    #[test]
    fn run_cmd_template_fail_stdout() {
        let version = Version::new(0, 0, 0);
        let line_header = "0 utils::tests::run_cmd_template_fail_stdout";
        let cmd = "non-existent-command";
        let args = &["-V"];
        let envs: &[(&str, String)] = &[];
        let cwd = ".";

        let expected_file_path = PycorsPathsProviderFromEnv::new()
            .logs()
            .join("Python_v0.0.0_step_0_utils__tests__run_cmd_template_fail_stdout.log");
        if expected_file_path.exists() {
            fs::remove_file(&expected_file_path).unwrap();
        }

        println!("expected_file_path: {:?}", expected_file_path);
        run_cmd_template(&version, line_header, cmd, args, envs, cwd).unwrap_err();

        let re = regex::Regex::new(r#"(?P<date>20[0-9]{2}-[0-1][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-5][0-9].[0-9]+[-+][0-2][0-9]:[0-5][0-9]) - (?P<cmd>.*)"#).unwrap();
        let file_content = fs::read_to_string(&expected_file_path).unwrap();
        println!("file_content:\n{}", file_content);

        let lines: Vec<&str> = file_content.lines().collect();

        let caps = re.captures(lines[0]).unwrap();
        let _date = &caps["date"];
        assert_eq!(&caps["cmd"], "cd .");

        let caps = re.captures(lines[1]).unwrap();
        let _date = &caps["date"];
        assert_eq!(&caps["cmd"], r#"non-existent-command ["-V"]"#);
    }

    #[test]
    #[ignore] // stderr is not saved for now in run_cmd_template()
    fn run_cmd_template_fail_stderr() {
        let version = Version::new(0, 0, 0);
        let line_header = "0 utils::tests::run_cmd_template_fail_stderr";
        let cmd = "cargo";
        let args = &["non-existent-subcommand"];
        let envs: &[(&str, String)] = &[];
        let cwd = ".";

        let expected_file_path = PycorsPathsProviderFromEnv::new()
            .logs()
            .join("Python_v0.0.0_step_0_utils__tests__run_cmd_template_fail_stderr.log");
        if expected_file_path.exists() {
            fs::remove_file(&expected_file_path).unwrap();
        }

        run_cmd_template(&version, line_header, cmd, args, envs, cwd).unwrap_err();

        let re = regex::Regex::new(r#"(?P<date>20[0-9]{2}-[0-3][0-9]-[0-1][0-9]T[0-2][0-9]:[0-5][0-9]:[0-5][0-9].[0-9]+[-+][0-2][0-9]:[0-5][0-9]) - (?P<cmd>.*)"#).unwrap();
        let file_content = fs::read_to_string(&expected_file_path).unwrap();
        println!("file_content:\n{}", file_content);

        let lines: Vec<&str> = file_content.lines().collect();

        let caps = re.captures(lines[0]).unwrap();
        let _date = &caps["date"];
        assert_eq!(&caps["cmd"], "cd .");

        let caps = re.captures(lines[1]).unwrap();
        let _date = &caps["date"];
        assert_eq!(&caps["cmd"], r#"cargo ["non-existent-subcommand"]"#);

        let caps = re.captures(lines[2]).unwrap();
        let _date = &caps["date"];
        assert_eq!(
            &caps["cmd"],
            r#"error: no such subcommand: `non-existent-command`"#
        );
    }
}
