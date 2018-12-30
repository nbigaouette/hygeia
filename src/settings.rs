use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use log::debug;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct PythonVersion {
    location: PathBuf,
    version: Version,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    installed_python: Vec<PythonVersion>,
}

impl Settings {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Settings> {
        debug!("Reading settings from file {:?}", path.as_ref());

        let input = File::open(path)?;
        let mut buffered = BufReader::new(input);

        let mut contents = String::new();
        buffered.read_to_string(&mut contents)?;

        let settings: Settings = toml::from_str(&contents)?;

        Ok(settings)
    }
}
