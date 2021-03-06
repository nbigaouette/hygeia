use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};

pub use anyhow::{Context, Result};

#[cfg(windows)]
pub const EXECUTABLE_EXTENSION: &str = ".exe";
#[cfg(not(windows))]
pub const EXECUTABLE_EXTENSION: &str = "";

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

pub struct MockedOutput<'a> {
    pub out: Option<&'a str>,
    pub err: Option<&'a str>,
}

pub fn mock_executable<P, S>(
    executable_location: P,
    executable_name: S,
    output: MockedOutput,
) -> crate::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    _mock_executable(
        executable_location.as_ref(),
        executable_name.as_ref(),
        output,
    )
}

fn _mock_executable(
    executable_location: &Path,
    executable_name: &str,
    output: MockedOutput,
) -> crate::Result<()> {
    let _cargo_output = std::process::Command::new("cargo")
        .args(&[
            "build",
            "--package",
            "print_file_to_stdout",
            "--target",
            env!("TARGET"), // Set by build.rs
        ])
        .output()
        .with_context(|| "Failed to execute 'cargo build --package print_file_to_stdout")?;

    if !executable_location.exists() {
        fs::create_dir_all(&executable_location)?
    }

    let stdout_filepath = executable_location.join(format!(
        "{}{}_hygeia_tests_to_print_stdout.txt",
        executable_name, EXECUTABLE_EXTENSION
    ));
    let stderr_filepath = executable_location.join(format!(
        "{}{}_hygeia_tests_to_print_stderr.txt",
        executable_name, EXECUTABLE_EXTENSION
    ));

    if stdout_filepath.exists() {
        fs::remove_file(&stdout_filepath)
            .with_context(|| format!("Failed to remove file {:?}", stdout_filepath))?;
    }
    if stderr_filepath.exists() {
        fs::remove_file(&stderr_filepath)
            .with_context(|| format!("Failed to remove file {:?}", stderr_filepath))?;
    }

    if let Some(stdout) = output.out {
        let mut f = File::create(&stdout_filepath)
            .with_context(|| format!("Failed to create file {:?}", stdout_filepath))?;
        f.write_all(stdout.as_bytes())
            .with_context(|| format!("Failed to write to file {:?}", stdout_filepath))?;
    }
    if let Some(stderr) = output.err {
        let mut f = File::create(&stderr_filepath)
            .with_context(|| format!("Failed to create file {:?}", stderr_filepath))?;
        f.write_all(stderr.as_bytes())
            .with_context(|| format!("Failed to write to file {:?}", stderr_filepath))?;
    }

    let print_file_to_stdout =
        Path::new(&env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| String::from("target")))
            .join(env!("TARGET"))
            .join("debug")
            .join(format!(
                "print_file_to_stdout{}",
                std::env::consts::EXE_SUFFIX
            ));

    fs::copy(
        &print_file_to_stdout,
        executable_location.join(format!("{}{}", executable_name, EXECUTABLE_EXTENSION)),
    )
    .with_context(|| {
        format!(
            "Failed to copy {:?} to {:?}",
            print_file_to_stdout,
            executable_location.join(format!("{}{}", executable_name, EXECUTABLE_EXTENSION))
        )
    })?;

    Ok(())
}

#[macro_export]
macro_rules! function_path {
    () => {{
        // https://stackoverflow.com/a/40234666
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        let function_path = &name[..name.len() - 3];
        function_path
    }};
}

#[macro_export]
macro_rules! _create_test_temp_dir_impl {
    ($directory:expr) => {{
        let dir = std::env::temp_dir()
            .join("hygeia")
            .join("integration_tests");

        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }
        let mut dir = dir.canonicalize().unwrap();

        let function_path = hygeia_test_helpers::function_path!();

        for component in function_path.split("::").skip(1) {
            dir.push(component);
        }

        dir.push($directory);

        let dir: std::path::PathBuf = dir
            .components()
            // Strip current directory from the path, mainly introduced by
            // the macro create_test_temp_dir!() when called without argument.
            .filter(|c| *c != std::path::Component::CurDir)
            .collect();

        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }

        std::fs::create_dir_all(&dir).unwrap();

        dir
    }};
}

#[macro_export]
macro_rules! create_test_temp_dir {
    ($subdirectory:ident) => {{
        hygeia_test_helpers::_create_test_temp_dir_impl!($subdirectory)
    }};
    () => {{
        hygeia_test_helpers::_create_test_temp_dir_impl!(".")
    }};
}
