// TODO: Add `git describe --dirty=-modified --tags --always --long` to archive name

use std::{env, fs, io, path::Path};

use structopt::StructOpt;
use zip::write::FileOptions;

type DynError = Box<dyn std::error::Error>;

const TARGET: &str = env!("TARGET");

const BIN_NAME: &str = "pycors";

#[cfg(windows)]
const BIN_EXTENSION: &str = ".exe";
#[cfg(not(windows))]
const BIN_EXTENSION: &str = "";

const ARCHIVE_EXTENSION: &str = "zip";

/// Tasks meant for CI
#[derive(StructOpt, Debug)]
enum Opt {
    /// Print to stdout the content of the `release_url/release_url.txt` file
    ReleaseUrl,
    /// Package the binary into a zip file meant for release
    PackageArtifacts(Target),
}

#[derive(StructOpt, Debug)]
enum Target {
    /// Debug
    Debug,
    /// Release
    Release,
}

impl Target {
    fn as_str(&self) -> &'static str {
        match self {
            Target::Debug => "debug",
            Target::Release => "release",
        }
    }
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let opt = Opt::from_args();

    match opt {
        Opt::ReleaseUrl => release_url(),
        Opt::PackageArtifacts(target) => package_artifacts(target),
    }
}

fn release_url() -> Result<(), DynError> {
    let release_url = fs::read_to_string("release_url/release_url.txt")?;
    println!("::set-output name=upload_url::{}", release_url);

    Ok(())
}

fn archive_basename() -> String {
    format!("{}-{}", BIN_NAME, TARGET)
}

fn bin_name() -> String {
    format!("{}{}", BIN_NAME, BIN_EXTENSION)
}

fn archive_name() -> String {
    format!("{}.{}", archive_basename(), ARCHIVE_EXTENSION)
}

fn package_artifacts(target: Target) -> Result<(), DynError> {
    let bin_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target")
        .join(target.as_str())
        .join(bin_name());
    let archive_path = archive_name();

    println!("Compressing {:?} into {:?}...", bin_path, archive_path);

    let mut b_in = io::BufReader::new(fs::File::open(&bin_path)?);
    let b_out = io::BufWriter::new(fs::File::create(archive_path)?);

    let mut zip = zip::ZipWriter::new(b_out);
    let options = FileOptions::default().unix_permissions(0o755);
    zip.start_file(bin_name(), options)?;
    io::copy(&mut b_in, &mut zip)?;
    zip.finish()?;

    Ok(())
}
