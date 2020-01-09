use super::*;

fn assert_version_output(output: std::process::Output) {
    let assert_output = output.assert();

    assert_output
        .success()
        .stdout(
            predicate::str::similar(format!(
                "{} {}\n",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .normalize(),
        )
        .stderr(predicate::str::is_empty().trim());
}

#[test]
fn long() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd.arg("--version").unwrap();
    assert_version_output(output);
}

#[test]
fn short() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd.arg("-V").unwrap();
    assert_version_output(output);
}
