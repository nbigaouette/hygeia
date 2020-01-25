use std::{env, fs};

fn main() {
    let current_exe = env::current_exe().unwrap();
    let bin_dir = current_exe.parent().unwrap();
    let bin_name = current_exe.file_name().unwrap();

    let stdout_filename = bin_dir.join(format!(
        "{}_hygeia_tests_to_print_stdout.txt",
        bin_name.to_str().unwrap()
    ));
    if stdout_filename.exists() {
        let to_print_stdout = fs::read_to_string(&stdout_filename).unwrap();
        println!("{}", to_print_stdout);
    }

    let stderr_filename = bin_dir.join(format!(
        "{}_hygeia_tests_to_print_stderr.txt",
        bin_name.to_str().unwrap()
    ));
    if stderr_filename.exists() {
        let to_print_stdout = fs::read_to_string(&stderr_filename).unwrap();
        eprintln!("{}", to_print_stdout);
    }
}
