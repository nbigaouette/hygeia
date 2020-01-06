// https://stackoverflow.com/a/40234666
#[cfg(test)]
macro_rules! function_path {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

#[cfg(test)]
macro_rules! create_test_temp_dir {
    () => {{
        let dir = std::env::temp_dir()
            .join(crate::constants::EXECUTABLE_NAME)
            .join("integration_tests");

        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }
        let mut dir = dir.canonicalize().unwrap();
        for component in function_path!().split("::").skip(1) {
            dir.push(component);
        }

        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }

        std::fs::create_dir_all(&dir).unwrap();

        dir
    }};
}

mod cache;
pub mod commands;
pub mod constants;
mod dir_monitor;
mod download;
mod os;
pub mod shim;
mod toolchain;
mod utils;

pub use anyhow::Result;
pub use structopt::StructOpt;
pub use thiserror::Error;

use git_testament::{git_testament, render_testament};
use lazy_static::lazy_static;

git_testament!(GIT_TESTAMENT);

fn git_version() -> &'static str {
    lazy_static! {
        static ref RENDERED: String = render_testament!(GIT_TESTAMENT);
    }
    &RENDERED
}

/// Control which Python toolchain to use on a directory basis.
#[derive(StructOpt, Debug)]
#[structopt(version = git_version())]
pub struct Opt {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,

    #[structopt(subcommand)]
    pub subcommand: Option<commands::Command>,
}

#[cfg(test)]
pub mod tests {
    use std::env;

    pub fn init_logger() {
        env::var("RUST_LOG")
            .or_else(|_| -> Result<String, ()> {
                let rust_log = "debug".to_string();
                println!("Environment variable 'RUST_LOG' not set.");
                println!("Setting to: {}", rust_log);
                env::set_var("RUST_LOG", &rust_log);
                Ok(rust_log)
            })
            .unwrap();
        let _ = env_logger::try_init();
    }

    // Version is reported as "unknown" in GitHub Actions.
    // See https://github.com/nbigaouette/pycors/pull/90/checks?check_run_id=311900597
    #[test]
    #[ignore]
    fn version() {
        let crate_version = structopt::clap::crate_version!();

        // GIT_VERSION is of the shape `v0.1.7-1-g095d7f5-modified`

        // Strip out the `v` prefix
        let (v, git_version_without_v) = crate::git_version().split_at(1);

        println!("crate_version: {:?}", crate_version);
        println!("v: {}", v);
        println!("git_version_without_v: {}", git_version_without_v);

        assert_eq!(v, "v");
        assert!(git_version_without_v.starts_with(crate_version));
    }
}
