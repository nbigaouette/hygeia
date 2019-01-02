use std::{
    env, fs,
    path::{Path, PathBuf},
};

use dirs::home_dir;
use failure::format_err;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_pycors_home() {
        let tmp_dir = env::temp_dir();
        env::set_var("PYCORS_HOME", &tmp_dir);
        let ph = pycors_home().unwrap();
        assert_eq!(ph, Path::new(&tmp_dir));
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
}
