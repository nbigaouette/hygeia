use super::*;

fn assert_help_output(output: std::process::Output) {
    let assert_output = output.assert();

    assert_output
        .success()
        .stdout(
            predicate::str::starts_with(format!(
                "{} {}\n",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .normalize()
            .and(predicate::str::contains("USAGE:"))
            .and(predicate::str::contains("FLAGS:"))
            .and(predicate::str::contains("SUBCOMMANDS:")),
        )
        .stderr("");
}

#[test]
fn long() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd.arg("--help").unwrap();
    assert_help_output(output);
}

#[test]
fn short() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd.arg("-h").unwrap();
    assert_help_output(output);
}
