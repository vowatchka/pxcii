//! Repository maintenance tasks: fixture generation, source-purity checks and the conversion demo.

mod convert_cmd;
mod core_purity;
mod gen_fixtures;
mod workspace_paths;

use std::process::ExitCode;

/// Runs a subcommand by name with the remaining arguments.
///
/// # Errors
///
/// Returns a human-readable message when the subcommand is unknown or fails.
fn run(args: &[String]) -> Result<(), String> {
    match args.split_first() {
        Some((cmd, rest)) if cmd == "gen-fixtures" => gen_fixtures::run(rest),
        Some((cmd, rest)) if cmd == "check-core-purity" => core_purity::run(rest),
        Some((cmd, rest)) if cmd == "convert" => convert_cmd::run(rest),
        Some((cmd, _)) => Err(format!(
            "unknown subcommand `{cmd}`\navailable: gen-fixtures, check-core-purity, convert"
        )),
        None => {
            Err("usage: cargo run -p xtask -- <gen-fixtures|check-core-purity|convert>".to_owned())
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The exit code is the only thing the process reports: 0 on success, 1 with a message.
    if let Err(message) = run(&args) {
        eprintln!("xtask: {message}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn unknown_subcommand_reports_available_names() {
        let error = run(&["nope".to_owned()]).unwrap_err();
        assert!(error.contains("unknown subcommand `nope`"), "{error}");
        assert!(error.contains("gen-fixtures"), "{error}");
    }

    #[test]
    fn missing_subcommand_prints_usage() {
        let error = run(&[]).unwrap_err();
        assert!(error.starts_with("usage:"), "{error}");
    }
}
