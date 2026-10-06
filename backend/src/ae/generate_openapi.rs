use ae::infrastructure::adapter_in::openapi::ApiDoc;
use std::path::Path;
use utoipa::OpenApi;

fn main() {
    let yaml = ApiDoc::openapi()
        .to_yaml()
        .expect("Failed to serialize OpenAPI spec to YAML");

    // `docs/` is shared with the iOS client and lives at the repository root, next to the crate
    // directory: resolved from the manifest so the output doesn't depend on the current directory.
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs");
    let output_path = output_dir.join("openapi.yml");
    std::fs::create_dir_all(&output_dir)
        .unwrap_or_else(|e| panic!("Failed to create directory {}: {e}", output_dir.display()));
    std::fs::write(&output_path, yaml)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output_path.display()));
}
