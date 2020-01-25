// TODO: Add `git describe --dirty=-modified --tags --always --long` to archive name

use std::{env, fs, io, path::Path, process::Command, str::FromStr};

use structopt::StructOpt;
use zip::write::FileOptions;

type DynError = Box<dyn std::error::Error>;

const BIN_NAME: &str = "hygeia";

#[cfg(windows)]
const BIN_EXTENSION: &str = ".exe";
#[cfg(not(windows))]
const BIN_EXTENSION: &str = "";

const ARCHIVE_EXTENSION: &str = "zip";

// /// Tasks meant for CI
#[derive(StructOpt, Debug)]
enum Opt {
    /// Print to stdout the content of the `release_url/release_url.txt` file
    ReleaseUrl,
    /// Package the binary into a zip file meant for release
    PackageArtifacts {
        /// Build target (debug or release)
        #[structopt(long)]
        target: Target,
        /// Target triple (f.e. x86_64-apple-darwin)
        #[structopt(long)]
        target_triple: String,
    },
    /// Run tests
    Test(Tests),
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

impl FromStr for Target {
    type Err = DynError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Target::Debug),
            "release" => Ok(Target::Release),
            _ => Err(Self::Err::from(format!("Unknown target: {}", s))),
        }
    }
}

#[derive(StructOpt, Debug)]
enum Tests {
    /// Run unit tests
    Unit,
    /// Run integration tests
    Integration(IntegrationTests),
}

#[derive(StructOpt, Debug)]
enum IntegrationTests {
    /// Run integration tests covering all commands
    Commands,
    /// Run integration tests that compile all Python 3 versions available
    AllVersions,
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
    println!("cargo xtask success!")
}

fn try_main() -> Result<(), DynError> {
    let opt = Opt::from_args();

    match opt {
        Opt::ReleaseUrl => release_url(),
        Opt::PackageArtifacts {
            target,
            target_triple,
        } => package_artifacts(target, target_triple),
        Opt::Test(tests_type) => run_tests(tests_type),
    }
}

fn run_tests(tests_type: Tests) -> Result<(), DynError> {
    let result = match tests_type {
        Tests::Unit => {
            cargo(&[
                "test",
                "--color=always",
                "--no-fail-fast",
                "tests::",
                "--",
                "--color=always",
                "--nocapture",
            ])?;
        }
        Tests::Integration(integration_tests) => match integration_tests {
            IntegrationTests::Commands => {
                cargo(&[
                    "test",
                    "--color=always",
                    "--no-fail-fast",
                    "integration::",
                    "--",
                    "--color=always",
                    "--nocapture",
                ])?;
            }
            IntegrationTests::AllVersions => {
                cargo(&[
                    "test",
                    "--color=always",
                    "--no-fail-fast",
                    "integration::install::all::",
                    "--",
                    "--color=always",
                    "--nocapture",
                    "--ignored",
                ])?;
            }
        },
    };

    Ok(result)
}

fn release_url() -> Result<(), DynError> {
    let release_url = fs::read_to_string("release_url/release_url.txt")?;
    println!("::set-output name=upload_url::{}", release_url);

    Ok(())
}

fn archive_basename(target_triple: &str) -> Result<String, DynError> {
    Ok(format!(
        "{}-{}-{}",
        BIN_NAME,
        git_describe()?,
        target_triple
    ))
}

fn bin_name() -> String {
    format!("{}{}", BIN_NAME, BIN_EXTENSION)
}

fn archive_name(target_triple: &str) -> Result<String, DynError> {
    Ok(format!(
        "{}.{}",
        archive_basename(target_triple)?,
        ARCHIVE_EXTENSION
    ))
}

fn package_artifacts(target: Target, target_triple: String) -> Result<(), DynError> {
    let bin_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target")
        .join(&target_triple)
        .join(target.as_str())
        .join(bin_name());
    let archive_path = archive_name(&target_triple)?;

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

pub fn cargo(arguments: &[&str]) -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    println!("Running:\n    {} {}", cargo, arguments.join(" "));

    let mut child = Command::new(&cargo).args(arguments).spawn()?;

    let exit_status = child.wait()?;
    if exit_status.success() {
        Ok(())
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                r#"Failed to run "{} {}". Error code: {:?}"#,
                cargo,
                arguments.join(" "),
                exit_status.code()
            ),
        )))
    }
}

pub fn git_describe() -> Result<String, DynError> {
    Ok(Command::new("git")
        .arg("describe")
        .arg("--always")
        .arg("--tags")
        .arg("--long")
        .arg("--dirty=-modified")
        .output()
        .map_err(|e: io::Error| -> DynError { Box::new(e) })
        .and_then(|output| {
            if !output.status.success() {
                Err(DynError::from(format!(
                    "Failed to call 'git describe --always --tags --long --dirty=-modified'"
                )))
            } else {
                Ok(output)
            }
        })
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .map_err(|e: std::string::FromUtf8Error| -> DynError { Box::new(e) })
        })?
        .trim()
        .to_string())
}
