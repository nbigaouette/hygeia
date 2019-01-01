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

    debug!("Found pycor's home: {:?}", home);

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

pub fn build_filename(version: &Version) -> Result<String> {
    let version_file = format!("{}", version).replace("-", "");

    Ok(format!("Python-{}.tgz", version_file))
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
