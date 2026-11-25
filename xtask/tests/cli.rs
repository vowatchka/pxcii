//! Runs the compiled `xtask` binary to cover its entry point and exit codes.

use std::process::Command;

fn xtask(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(arguments)
        .output()
        .expect("the xtask binary can be spawned")
}

#[test]
fn purity_check_succeeds_on_the_real_sources() {
    let output = xtask(&["check-core-purity"]);
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pxcii-core is pure"), "{stdout}");
}

#[test]
fn fixture_generation_writes_into_the_requested_directory() {
    // Tests never write the committed `fixtures/`: the scratch directory lives under `target/`.
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("fixtures-cli");
    let _ = std::fs::remove_dir_all(&directory);
    let output = xtask(&["gen-fixtures", "--out", &directory.to_string_lossy()]);
    assert!(output.status.success(), "status: {:?}", output.status);
    assert!(directory.join("gradient.png").is_file());
}

#[test]
fn unknown_subcommand_exits_with_failure() {
    let output = xtask(&["definitely-not-a-task"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown subcommand"), "{stderr}");
}
