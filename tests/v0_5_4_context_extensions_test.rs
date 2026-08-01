//! Regression coverage for the fork's opt-in `[server].context_extensions`.
//!
//! The upstream server generator has evolved since this feature was added, so
//! these tests pin its public contract rather than obsolete full-file output.

use openapi_to_rust::config::ServerSection;
use openapi_to_rust::server::codegen::ServerCodegen;
use openapi_to_rust::{GeneratedFile, GeneratorConfig, SchemaAnalyzer};
use std::fs;
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v0_5_4")
}

fn small_spec() -> serde_json::Value {
    let path = fixtures_dir().join("context_extensions_spec.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn generate_server(context_extensions: bool) -> Vec<GeneratedFile> {
    let mut analyzer = SchemaAnalyzer::new(small_spec()).expect("analyzer");
    let analysis = analyzer.analyze().expect("analyze");
    let config = GeneratorConfig {
        enable_async_client: false,
        server: Some(ServerSection {
            framework: "axum".to_string(),
            operations: vec!["ping".to_string()],
            prune_models: false,
            context_extensions,
            validation: Default::default(),
        }),
        ..Default::default()
    };
    let generator = openapi_to_rust::CodeGenerator::new(config);
    let server = generator.config().server.as_ref().expect("server");
    ServerCodegen::new(generator.config(), &analysis, server)
        .generate()
        .expect("server codegen")
}

fn file_content<'a>(files: &'a [GeneratedFile], path: &str) -> &'a str {
    files
        .iter()
        .find(|f| f.path.as_path() == Path::new(path))
        .unwrap_or_else(|| panic!("missing generated file {path}"))
        .content
        .as_str()
}

#[test]
fn context_extensions_off_emits_no_context_surface() {
    let files = generate_server(false);
    let api = file_content(&files, "server/api.rs");
    let router = file_content(&files, "server/router.rs");

    assert!(!api.contains("RequestContext"));
    assert!(!router.contains("RequestContext"));
    assert!(api.contains("async fn ping(&self, body: PingRequest) -> PingResponse"));
    assert!(router.contains("api.ping(body).await"));
}

#[test]
fn context_extensions_on_adds_parts_extractor_and_leading_trait_argument() {
    let off = generate_server(false);
    let on = generate_server(true);
    let api = file_content(&on, "server/api.rs");
    let router = file_content(&on, "server/router.rs");

    assert!(api.contains("pub struct RequestContext(pub ::axum::http::Extensions);"));
    assert!(api.contains("pub fn get<T: ::std::clone::Clone + Send + Sync + 'static>"));
    assert!(api.contains("::axum::extract::FromRequestParts<S> for RequestContext"));
    assert!(api.contains("Ok(Self(parts.extensions.clone()))"));
    assert!(
        api.contains(
            "async fn ping(&self, ctx: RequestContext, body: PingRequest) -> PingResponse"
        )
    );
    assert!(router.contains("ctx: RequestContext"));
    assert!(router.contains("api.ping(ctx, body).await"));

    // The opt-in affects only the API and router surface.
    for path in ["server/errors.rs", "server/mod.rs", "server/validation.rs"] {
        assert_eq!(file_content(&on, path), file_content(&off, path), "{path}");
    }
}

#[test]
fn context_extensions_defaults_false_in_toml() {
    use openapi_to_rust::ConfigFile;
    use std::io::Write;
    use tempfile::NamedTempFile;

    let spec = fixtures_dir().join("context_extensions_spec.json");
    let content = format!(
        r#"[generator]
spec_path = "{}"
output_dir = "src/generated"
module_name = "api"

[features]

[server]
framework = "axum"
operations = ["ping"]
"#,
        spec.display()
    );
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{content}").unwrap();
    let config = ConfigFile::load(file.path()).expect("load");
    assert!(!config.server.unwrap().context_extensions);
}
