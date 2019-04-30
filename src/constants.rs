/// Name of the executable, reused across project.
pub const EXECUTABLE_NAME: &str = env!("CARGO_PKG_NAME");

/// Default hidden configuration directory.
pub const DEFAULT_DOT_DIR: String = format!(".{}", EXECUTABLE_NAME);

///
pub const HOME_ENV_VARIABLE: String = format!("{}_HOME", EXECUTABLE_NAME.to_uppercase());

pub const INFO_FILE: String = format!("installed_by_{}.txt", EXECUTABLE_NAME);

pub const EXTRA_PACKAGES_FILENAME: &str = "extra-packages-to-install.txt";

pub const EXTRA_PACKAGES_FILENAME_CONTENT: &str = include_str!("../extra-packages-to-install.txt");

pub const INSTALL_DUMMY_FILE: &str = "installed_dummy_file";
