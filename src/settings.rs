use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use log::debug;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{utils, Result};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PythonVersion {
    pub location: PathBuf,
    pub version: Version,
}

pub fn load_settings_file() -> Result<(Settings, PathBuf)> {
    let pycors_home = utils::pycors_home()?;
    let settings_file = pycors_home.join("settings.toml");

    if !utils::path_exists(&pycors_home) {
        debug!("Directory {:?} does not exists. Creating.", pycors_home);
        fs::create_dir_all(&pycors_home)?;
    }

    if utils::path_exists(&settings_file) {
        Ok((Settings::from_file(&settings_file)?, settings_file))
    } else {
        debug!(
            "File {:?} does not exists. Creatin a default one.",
            settings_file
        );
        let settings = Settings::default();
        settings.save_to(&settings_file)?;
        Ok((settings, settings_file))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub installed_python: Vec<PythonVersion>,
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

    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<usize> {
        let settings_toml = toml::to_string_pretty(self)?;
        let mut output = File::create(&path)?;
        Ok(output.write(settings_toml.as_bytes())?)
    }
}
