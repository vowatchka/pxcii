//! Runs the compiled binary: the stub has no CLI yet, so this covers the entry point only.

use std::process::Command;

#[test]
fn binary_prints_the_version_banner() {
    let output = Command::new(env!("CARGO_BIN_EXE_pxcii"))
        .output()
        .expect("the test binary can be spawned");
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("pxcii "), "{stdout}");
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "{stdout}");
}
