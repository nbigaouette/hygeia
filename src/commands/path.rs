use crate::{
    toolchain::{find_installed_toolchains, get_compatible_version_or_latest, InstalledToolchain},
    Result,
};

pub fn run() -> Result<()> {
    let installed_toolchains: Vec<InstalledToolchain> = find_installed_toolchains()?;
    let compatible_toolchain = get_compatible_version_or_latest(&installed_toolchains)?;

    match compatible_toolchain {
        Some(compatible_toolchain) => println!("{}", compatible_toolchain.location.display()),
        None => {
            log::error!("No Python interpreter found at all. Please install at least one!");
            println!("")
        }
    }

    Ok(())
}
