use ae::infrastructure::adapter_in::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let yaml = ApiDoc::openapi()
        .to_yaml()
        .expect("Failed to serialize OpenAPI spec to YAML");

    let output_dir = "doc";
    let output_path = format!("{output_dir}/openapi.yml");
    std::fs::create_dir_all(output_dir)
        .unwrap_or_else(|e| panic!("Failed to create directory {output_dir}: {e}"));
    std::fs::write(&output_path, yaml)
        .unwrap_or_else(|e| panic!("Failed to write {output_path}: {e}"));
}
