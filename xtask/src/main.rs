// TODO: Add `git describe --dirty=-modified --tags --always --long` to archive name

use std::{env, fs, io, path::Path};

use zip::write::FileOptions;

type DynError = Box<dyn std::error::Error>;

const TARGET: &str = env!("TARGET");

const BIN_NAME: &str = "pycors";

#[cfg(windows)]
const BIN_EXTENSION: &str = ".exe";
#[cfg(not(windows))]
const BIN_EXTENSION: &str = "";

const ARCHIVE_EXTENSION: &str = "zip";

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_ref().map(|it| it.as_str()) {
        Some("package_artifacts") => package_artifacts()?,
        Some("release_url") => release_url()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:
release_url            Extract release url from 'release_url/release_url.txt' file
"
    )
}

fn release_url() -> Result<(), DynError> {
    let release_url = fs::read_to_string("release_url/release_url.txt")?;
    println!("::set-output name=upload_url::{}", release_url);

    Ok(())
}

fn archive_basename() -> String {
    format!("pycors-{}", TARGET)
}

fn bin_name() -> String {
    format!("{}{}", BIN_NAME, BIN_EXTENSION)
}

fn archive_name() -> String {
    format!("{}.{}", archive_basename(), ARCHIVE_EXTENSION)
}

fn package_artifacts() -> Result<(), DynError> {
    let bin_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target")
        // .join("release")
        .join("debug")
        .join(bin_name());
    println!("bin_path: {:?}", bin_path);

    let mut b_in = io::BufReader::new(fs::File::open(&bin_path)?);
    let b_out = io::BufWriter::new(fs::File::create(archive_name())?);

    let mut zip = zip::ZipWriter::new(b_out);
    let options = FileOptions::default().unix_permissions(0o755);
    zip.start_file(bin_name(), options)?;
    io::copy(&mut b_in, &mut zip)?;
    zip.finish()?;

    Ok(())
}
