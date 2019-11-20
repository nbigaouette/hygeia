use std::{env, fs};

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_ref().map(|it| it.as_str()) {
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
