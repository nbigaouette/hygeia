use std::env;

const ENV_NAME_TO_PRINT_STDOUT_VALUE: &str = "PYCORS_TESTS_TO_PRINT_STDOUT";
const ENV_NAME_TO_PRINT_STDERR_VALUE: &str = "PYCORS_TESTS_TO_PRINT_STDERR";

fn main() {
    match env::var(ENV_NAME_TO_PRINT_STDOUT_VALUE) {
        Ok(to_print_stdout) => println!("{}", to_print_stdout),
        Err(_) => {}
    }
    match env::var(ENV_NAME_TO_PRINT_STDERR_VALUE) {
        Ok(to_print_stderr) => eprintln!("{}", to_print_stderr),
        Err(_) => {}
    }
}
