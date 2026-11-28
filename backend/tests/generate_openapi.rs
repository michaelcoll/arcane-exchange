use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn generate_openapi(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_generate-openapi"))
        .args(args)
        .output()
        .expect("Failed to run generate-openapi")
}

/// A temp path removed on drop, so a failing assertion doesn't leave files behind.
struct TempSpec(PathBuf);

impl TempSpec {
    fn new(name: &str) -> Self {
        Self(std::env::temp_dir().join(format!("{}-{name}-openapi.yml", std::process::id())))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn arg(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for TempSpec {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_dir(&self.0);
    }
}

#[test]
fn check_succeeds_when_the_file_is_up_to_date() {
    let spec = TempSpec::new("up-to-date");
    assert!(generate_openapi(&[spec.arg()]).status.success());
    let before = std::fs::read(spec.path()).unwrap();

    let output = generate_openapi(&["--check", spec.arg()]);

    assert!(output.status.success());
    assert_eq!(std::fs::read(spec.path()).unwrap(), before);
}

#[test]
fn check_fails_and_leaves_the_file_untouched_when_it_is_stale() {
    let spec = TempSpec::new("stale");
    std::fs::write(spec.path(), "openapi: stale\n").unwrap();

    let output = generate_openapi(&["--check", spec.arg()]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("mise run rebuild-docs"));
    assert_eq!(
        std::fs::read_to_string(spec.path()).unwrap(),
        "openapi: stale\n"
    );
}

#[test]
fn check_fails_when_the_file_is_missing() {
    let spec = TempSpec::new("missing");

    let output = generate_openapi(&["--check", spec.arg()]);

    assert!(!output.status.success());
    assert!(!spec.path().exists());
}

#[test]
fn check_reports_the_read_error_when_the_file_cannot_be_read() {
    let spec = TempSpec::new("unreadable");
    std::fs::create_dir(spec.path()).unwrap();

    let output = generate_openapi(&["--check", spec.arg()]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("Failed to read"));
    assert!(!stderr.contains("mise run rebuild-docs"));
}

#[test]
fn an_unknown_option_fails_without_writing_a_file() {
    let spec = TempSpec::new("unknown-option");

    let output = generate_openapi(&["--chek", spec.arg()]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--chek"));
    assert!(!spec.path().exists());
}
