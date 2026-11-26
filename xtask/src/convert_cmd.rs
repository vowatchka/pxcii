//! `xtask convert <input> --to <ext>`: converts one file with the core library (stage 2 demo).

use std::path::PathBuf;

use pxcii_core::{EncodeOptions, Format};

use crate::workspace_paths;

/// Converts `<input>` to the format named by `--to` and writes it into `target/convert/`.
///
/// The output never lands next to the input, so the demo cannot overwrite a committed fixture.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let [input, flag, extension] = args else {
        return Err(USAGE.to_owned());
    };
    if flag != "--to" {
        return Err(USAGE.to_owned());
    }
    let format = Format::from_ext(extension)
        .ok_or_else(|| format!("unknown target format `{extension}`"))?;
    let directory = workspace_paths::workspace_root()
        .join("target")
        .join("convert");
    let output = convert_file(&PathBuf::from(input), format, &directory)?;
    println!("wrote {}", output.display());
    Ok(())
}

const USAGE: &str = "usage: convert <input> --to <png|jpg|webp|gif|bmp|tiff|ico|tga|pnm>";

fn convert_file(
    input: &std::path::Path,
    format: Format,
    directory: &std::path::Path,
) -> Result<PathBuf, String> {
    let bytes = std::fs::read(input)
        .map_err(|error| format!("cannot read {}: {error}", input.display()))?;
    let converted = pxcii_core::convert(&bytes, format, &EncodeOptions::default())
        .map_err(|error| format!("{}: {error}", input.display()))?;
    for warning in &converted.warnings {
        eprintln!("hint: {warning:?}");
    }
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let stem = input.file_stem().unwrap_or_default();
    let output = directory.join(stem).with_extension(format.extension());
    std::fs::write(&output, converted.bytes)
        .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{convert_file, run};
    use crate::workspace_paths;

    fn input() -> String {
        workspace_paths::fixtures_dir()
            .join("gradient.png")
            .to_string_lossy()
            .into_owned()
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn converts_into_the_target_directory() {
        run(&args(&[&input(), "--to", "bmp"])).expect("converts");
        let output = workspace_paths::workspace_root()
            .join("target")
            .join("convert")
            .join("gradient.bmp");
        assert!(output.is_file());
    }

    #[test]
    fn warnings_do_not_fail_the_conversion() {
        run(&args(&[&input(), "--to", "jpg"])).expect("converts with hints");
    }

    #[test]
    fn wrong_arguments_print_usage() {
        for bad in [args(&[]), args(&[&input(), "--as", "bmp"])] {
            let error = run(&bad).expect_err("usage error");
            assert!(error.starts_with("usage: convert"), "{error}");
        }
    }

    #[test]
    fn unknown_target_is_reported() {
        let error = run(&args(&[&input(), "--to", "heic"])).expect_err("unknown format");
        assert!(error.contains("unknown target format `heic`"), "{error}");
    }

    #[test]
    fn missing_input_is_reported() {
        let error = run(&args(&["no-such-file.png", "--to", "bmp"])).expect_err("missing");
        assert!(error.contains("cannot read"), "{error}");
    }

    #[test]
    fn undecodable_input_is_reported() {
        let garbage = workspace_paths::workspace_root().join("Cargo.toml");
        let error = run(&args(&[&garbage.to_string_lossy(), "--to", "png"])).expect_err("garbage");
        assert!(error.contains("unsupported image format"), "{error}");
    }

    #[test]
    fn blocked_output_directory_is_reported() {
        let blocker = workspace_paths::workspace_root()
            .join("target")
            .join("convert-blocked");
        std::fs::write(&blocker, b"a file, not a directory").expect("blocker");
        let error = convert_file(
            &workspace_paths::fixtures_dir().join("gradient.png"),
            pxcii_core::Format::Bmp,
            &blocker.join("nested"),
        )
        .expect_err("cannot create");
        assert!(error.contains("cannot create"), "{error}");
    }

    #[test]
    fn unwritable_output_is_reported() {
        let directory = workspace_paths::workspace_root()
            .join("target")
            .join("convert-unwritable");
        // A directory named like the output file makes `fs::write` fail.
        std::fs::create_dir_all(directory.join("gradient.bmp")).expect("blocker");
        let error = convert_file(
            &workspace_paths::fixtures_dir().join("gradient.png"),
            pxcii_core::Format::Bmp,
            &directory,
        )
        .expect_err("cannot write");
        assert!(error.contains("cannot write"), "{error}");
    }
}
