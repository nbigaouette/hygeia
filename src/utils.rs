use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use dirs::home_dir;
use failure::format_err;
use indicatif::{ProgressBar, ProgressStyle};
use semver::{Version, VersionReq};
use subprocess::{Exec, Redirection};
use terminal_size::{terminal_size, Width};

use crate::{
    config::Cfg,
    settings::{PythonVersion, Settings},
    Result,
};

pub fn path_exists<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path).is_ok()
}

pub fn copy_file<P1: AsRef<Path>, P2: AsRef<Path>>(from: P1, to: P2) -> Result<u64> {
    if from.as_ref() == to.as_ref() {
        Err(format_err!(
            "Will not copy {:?} unto {:?} as this would probably truncate it.",
            from.as_ref(),
            to.as_ref()
        ))
    } else {
        let number_of_bytes_copied = fs::copy(from, to)?;
        Ok(number_of_bytes_copied)
    }
}

pub fn pycors_home() -> Result<PathBuf> {
    let env_var = env::var_os("PYCORS_HOME");

    let pycors_home = if env_var.is_some() {
        let cwd = env::current_dir()?;
        env_var.clone().map(|home| cwd.join(home))
    } else {
        None
    };

    let user_home = dot_dir(".pycors");

    let home = match pycors_home.or(user_home) {
        None => Err(format_err!("Cannot find pycors' home directory")),
        Some(home) => Ok(home),
    }?;

    Ok(home)
}

fn dot_dir(name: &str) -> Option<PathBuf> {
    home_dir().map(|p| p.join(name))
}

pub fn pycors_cache() -> Result<PathBuf> {
    Ok(pycors_home()?.join("cache"))
}

pub fn pycors_download() -> Result<PathBuf> {
    Ok(pycors_cache()?.join("downloads"))
}

pub fn pycors_extract() -> Result<PathBuf> {
    Ok(pycors_cache()?.join("extracted"))
}

pub fn pycors_installed() -> Result<PathBuf> {
    Ok(pycors_home()?.join("installed"))
}

pub fn pycors_shims() -> Result<PathBuf> {
    Ok(pycors_home()?.join("shims"))
}

pub fn pycors_logs() -> Result<PathBuf> {
    Ok(pycors_home()?.join("logs"))
}

pub fn install_dir(version: &Version) -> Result<PathBuf> {
    Ok(pycors_installed()?.join(format!("{}", version)))
}

pub fn default_extra_package_file() -> Result<PathBuf> {
    Ok(pycors_home()?.join("extra-packages-to-install.txt"))
}

pub fn build_basename(version: &Version) -> Result<String> {
    let version_file = format!("{}", version).replace("-", "");

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
        Ok(()) => {}
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => {
                log::warn!("Source {:?} not found when creating hard link", from)
            }
            _ => Err(e)?,
        },
    }

    Ok(())
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
    settings: &'a Settings,
) -> Option<&'a PythonVersion> {
    // Find the compatible versions from the installed list
    let mut compatible_versions: Vec<&'a PythonVersion> = settings
        .installed_python
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

pub fn get_interpreter_to_use(cfg: &Option<Cfg>, settings: &Settings) -> Result<PythonVersion> {
    if !cfg.is_some() {
        log::warn!("No '.python-version' found.");
        log::warn!("Please select a Python version to use with:");
        log::warn!("    pycors select <version>");
        log::warn!("");
        log::warn!("See available versions with:");
        log::warn!("    pycors list");
        log::warn!("");
        log::warn!("pycors will select the highest version available.");
    }

    // If `cfg` is `None`, check if there is something in `Settings`; pick the first found
    // interpreter to construct a `cfg`.
    let cfg: Cfg = cfg
        .as_ref() // &Option<Cfg> -> Option<&Cfg>
        .cloned() // Option<&Cfg> -> Option<Cfg>
        .or_else(|| {
            // Sort available versions
            let mut installed_python = settings.installed_python.clone();
            installed_python.sort_by_key(|python| python.version.clone());
            installed_python.reverse();
            match installed_python.get(0) {
                None => None,
                Some(latest_interpreter_found) => Some(Cfg {
                    version: VersionReq::exact(&latest_interpreter_found.version),
                }),
            }
        })
        .ok_or_else(|| {
            format_err!("No Python runtime configured. Use `pycors select <version> <version>`.")
        })?;

    let active_python = active_version(&cfg.version, settings).ok_or_else(|| {
        log::error!(
            "Could not find Python {} as requested from the file `.python-version`.",
            cfg.version
        );
        log::error!("Either:");
        log::error!("    1) Remove the file `.python-version` to use (one of) the interpreter(s) available in your $PATH.");
        log::error!("    2) Edit the file to use an installed interpreter.");
        log::error!("       For example, to list available interpreters:");
        log::error!("           pycors list");
        log::error!("       Then select a version to use:");
        log::error!("           pycors select <version> ~3.7");
        format_err!("No active Python runtime found.")
    })?.clone();

    Ok(active_python)
}

pub fn dir_files_set<P>(dir: P) -> Result<HashSet<PathBuf>>
where
    P: AsRef<Path>,
{
    Ok(fs::read_dir(dir.as_ref())?
        .filter_map(|entry| match entry {
            Ok(dir) => Some(dir.path()),
            Err(err) => {
                log::error!("Reading failed: {:?}", err);
                None
            }
        })
        .collect())
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
    let logs_dir = pycors_logs()?;

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

    log_line(&format!("cd {}", cwd.as_ref().display()), &mut log_file);
    log_line(&format!("{} {:?}", cmd, args), &mut log_file);

    let (tx, child) = spinner_in_thread(line_header.to_string());

    let stream = Exec::cmd(cmd)
        .args(args)
        .cwd(cwd)
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
    fn pycors_home_default() {
        env::remove_var("PYCORS_HOME");
        let default_home = pycors_home().unwrap();
        let expected = home_dir().unwrap().join(".pycors");
        assert_eq!(default_home, expected);
    }

    #[test]
    fn pycors_home_from_env_variable() {
        let tmp_dir = env::temp_dir();
        env::set_var("PYCORS_HOME", &tmp_dir);
        let tmp_home = pycors_home().unwrap();
        assert_eq!(tmp_home, Path::new(&tmp_dir));
    }

    #[test]
    fn dot_dir_success() {
        env::remove_var("PYCORS_HOME");
        let dir = dot_dir(".dummy").unwrap();
        let expected = home_dir().unwrap().join(".dummy");
        assert_eq!(dir, expected);
    }

    #[test]
    fn pycors_directories() {
        env::remove_var("PYCORS_HOME");
        let dir = pycors_cache().unwrap();
        let expected = home_dir().unwrap().join(".pycors").join("cache");
        assert_eq!(dir, expected);

        let dir = pycors_download().unwrap();
        let expected = home_dir()
            .unwrap()
            .join(".pycors")
            .join("cache")
            .join("downloads");
        assert_eq!(dir, expected);

        let dir = pycors_extract().unwrap();
        let expected = home_dir()
            .unwrap()
            .join(".pycors")
            .join("cache")
            .join("extracted");
        assert_eq!(dir, expected);

        let dir = pycors_installed().unwrap();
        let expected = home_dir().unwrap().join(".pycors").join("installed");
        assert_eq!(dir, expected);
    }

    #[test]
    fn install_dir_version() {
        env::remove_var("PYCORS_HOME");
        let version = Version::parse("3.7.2").unwrap();
        let dir = install_dir(&version).unwrap();
        let expected = home_dir()
            .unwrap()
            .join(".pycors")
            .join("installed")
            .join("3.7.2");
        assert_eq!(dir, expected);
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
