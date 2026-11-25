//! `pxcii` entry point.
//!
//! Stage 1 ships the library only; the CLI itself arrives in stage 4.

use std::process::ExitCode;

/// Prints the banner. Returns the process exit code.
fn run() -> u8 {
    println!(
        "pxcii {} — library build, the CLI arrives in stage 4",
        env!("CARGO_PKG_VERSION")
    );
    0
}

fn main() -> ExitCode {
    ExitCode::from(run())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn run_succeeds() {
        assert_eq!(run(), 0);
    }
}
