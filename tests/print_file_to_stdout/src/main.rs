use std::{fs, path::Path};

const FILE_NAME_TO_PRINT_STDOUT_VALUE: &str = "pycors_tests_to_print_stdout.txt";
const FILE_NAME_TO_PRINT_STDERR_VALUE: &str = "pycors_tests_to_print_stderr.txt";

fn main() {
    let stdout_filename = Path::new(FILE_NAME_TO_PRINT_STDOUT_VALUE);
    if stdout_filename.exists() {
        let to_print_stdout = fs::read_to_string(&stdout_filename).unwrap();
        println!("{}", to_print_stdout);
    }

    let stderr_filename = Path::new(FILE_NAME_TO_PRINT_STDERR_VALUE);
    if stderr_filename.exists() {
        let to_print_stdout = fs::read_to_string(&stderr_filename).unwrap();
        eprintln!("{}", to_print_stdout);
    }
}
