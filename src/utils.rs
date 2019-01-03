use std::{
    env, fs,
    path::{Path, PathBuf},
};

use dirs::home_dir;
use failure::format_err;
use log::debug;
use semver::Version;

use crate::Result;

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

pub fn install_dir(version: &Version) -> Result<PathBuf> {
    Ok(pycors_installed()?.join(format!("{}", version)))
}

pub fn build_basename(version: &Version) -> Result<String> {
    let version_file = format!("{}", version).replace("-", "");

    Ok(format!("Python-{}", version_file))
}

pub fn build_filename(version: &Version) -> Result<String> {
    Ok(format!("{}.tgz", build_basename(version)?))
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
        debug!(
            "Creating hard link from {:?} to {:?}...",
            copy_from.as_ref(),
            new_path
        );
        fs::hard_link(copy_from.as_ref(), &new_path)?;
    }

    Ok(())
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
        assert_eq!(nb_bytes_copied, 10838);
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
    fn dot_dir_sucess() {
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
    fn build_filename_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();

        let filename = build_filename(&version).unwrap();

        assert_eq!(&filename, "Python-3.7.2.tgz");
    }

    #[test]
    fn build_filename_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let filename = build_filename(&version).unwrap();
        assert_eq!(&filename, "Python-3.7.2rc1.tgz");
    }

    #[test]
    fn create_hard_links_success() {
        let in_dir = env::temp_dir();
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
