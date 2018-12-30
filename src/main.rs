use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use dirs::home_dir;
use failure::format_err;
use log::{debug, error};

mod config;
mod settings;

use crate::config::Cfg;
use crate::settings::Settings;

pub type Result<T> = std::result::Result<T, failure::Error>;

static TOOLCHAIN_FILE: &str = ".python-version";

fn main() -> Result<()> {
    env_logger::init();

    match env::args().next() {
        None => {
            error!("Cannot get first argument.");
            Err(format_err!("Cannot get first argument"))?
        }
        Some(arg) => {
            if arg.ends_with("pycors") {
                debug!("Running pycors");
                pycors()
            } else {
                debug!("Running a Python shim");
                python_shim()
            }
        }
    }
}

fn python_shim() -> Result<()> {
    unimplemented!()
}

fn pycors() -> Result<()> {
    let settings = load_settings_file()?;
    let cfg = load_config_file()?;

    debug!("settings: {:?}", settings);
    debug!("cfg: {:?}", cfg);

    Ok(())
}

fn load_settings_file() -> Result<Settings> {
    let pycors_home = pycors_home()?;
    let settings_file = pycors_home.join("settings.toml");

    if !path_exists(&pycors_home) {
        debug!("Directory {:?} does not exists. Creating.", pycors_home);
        fs::create_dir_all(&pycors_home)?;
    }

    if !path_exists(&settings_file) {
        debug!(
            "File {:?} does not exists. Creatin a default one.",
            settings_file
        );
        let settings = Settings::default();
        let settings_toml = toml::to_string_pretty(&settings)?;
        let mut output = File::create(&settings_file)?;
        output.write(settings_toml.as_bytes())?;
    }

    Settings::from_file(&settings_file)
}

fn load_config_file() -> Result<Cfg> {
    if path_exists(TOOLCHAIN_FILE) {
        Cfg::from_file(TOOLCHAIN_FILE)
    } else {
        Cfg::from_user_input()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_pycors_home() {
        env::set_var("PYCORS_HOME", "/tmp");
        let ph = pycors_home().unwrap();
        assert_eq!(ph, Path::new("/tmp"));
    }
}
