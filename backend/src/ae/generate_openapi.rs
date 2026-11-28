use ae::infrastructure::adapter_in::openapi::ApiDoc;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use utoipa::OpenApi;

/// `generate-openapi [--check] [PATH]`: writes the spec to `PATH` (default `docs/openapi.yml`), or with
/// `--check` compares it to the file and exits non-zero if it differs, leaving the file as it is.
fn main() -> ExitCode {
    let mut check = false;
    let mut path_arg = None;
    for arg in std::env::args().skip(1) {
        if arg == "--check" {
            check = true;
        } else if arg.starts_with("--") {
            eprintln!("Unknown option {arg}. Usage: generate-openapi [--check] [PATH]");
            return ExitCode::FAILURE;
        } else {
            path_arg = Some(PathBuf::from(arg));
        }
    }

    let yaml = ApiDoc::openapi()
        .to_yaml()
        .expect("Failed to serialize OpenAPI spec to YAML");

    // `docs/` is shared with the iOS client and lives at the repository root, next to the crate
    // directory: resolved from the manifest so the output doesn't depend on the current directory.
    let output_path = path_arg
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/openapi.yml"));

    if check {
        return match std::fs::read_to_string(&output_path) {
            Ok(current) if current == yaml => ExitCode::SUCCESS,
            Ok(_) => out_of_date(&output_path),
            Err(e) if e.kind() == ErrorKind::NotFound => out_of_date(&output_path),
            Err(e) => {
                eprintln!("Failed to read {}: {e}", output_path.display());
                ExitCode::FAILURE
            }
        };
    }

    if let Some(output_dir) = output_path.parent() {
        std::fs::create_dir_all(output_dir)
            .unwrap_or_else(|e| panic!("Failed to create directory {}: {e}", output_dir.display()));
    }
    std::fs::write(&output_path, yaml)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output_path.display()));
    ExitCode::SUCCESS
}

fn out_of_date(path: &Path) -> ExitCode {
    eprintln!(
        "{} is out of date. Run 'mise run rebuild-docs' and commit the changes.",
        path.display()
    );
    ExitCode::FAILURE
}
