//! Golden fixtures for v0.5.4: opt-in `[server].context_extensions`.
//!
//! - WITH flag: trait methods take leading `ctx: RequestContext`; handlers
//!   build it via `RequestContext(parts.extensions.clone())` before body
//!   extraction.
//! - WITHOUT flag: output matches the locked v0.5.3 server baseline (byte-
//!   identical backward-compat proof).

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

fn read_golden(name: &str) -> String {
    let path = fixtures_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read golden {}: {e}", path.display()))
}

/// Backward-compat proof: flag off (default) matches locked v0.5.3 server
/// output for this fixture.
#[test]
fn context_extensions_off_matches_v0_5_3_baseline() {
    let files = generate_server(false);
    let api = file_content(&files, "server/api.rs");
    let router = file_content(&files, "server/router.rs");
    let errors = file_content(&files, "server/errors.rs");
    let mod_rs = file_content(&files, "server/mod.rs");

    assert_eq!(
        api,
        read_golden("baseline_off_api.rs"),
        "server/api.rs without flag must stay byte-identical to v0.5.3 baseline"
    );
    assert_eq!(
        router,
        read_golden("baseline_off_router.rs"),
        "server/router.rs without flag must stay byte-identical to v0.5.3 baseline"
    );
    assert_eq!(
        errors,
        read_golden("baseline_off_errors.rs"),
        "server/errors.rs without flag must stay byte-identical to v0.5.3 baseline"
    );
    assert_eq!(
        mod_rs,
        read_golden("baseline_off_mod.rs"),
        "server/mod.rs without flag must stay byte-identical to v0.5.3 baseline"
    );

    assert!(
        !api.contains("RequestContext"),
        "flag off must not emit RequestContext"
    );
    assert!(
        !router.contains("RequestContext"),
        "flag off must not reference RequestContext in handlers"
    );
}

/// Golden for flag on: RequestContext type, leading trait param, handler
/// constructs from parts.extensions before body extraction.
#[test]
fn context_extensions_on_matches_golden() {
    let files = generate_server(true);
    let api = file_content(&files, "server/api.rs");
    let router = file_content(&files, "server/router.rs");
    let errors = file_content(&files, "server/errors.rs");
    let mod_rs = file_content(&files, "server/mod.rs");

    assert_eq!(api, read_golden("baseline_on_api.rs"));
    assert_eq!(router, read_golden("baseline_on_router.rs"));
    assert_eq!(errors, read_golden("baseline_on_errors.rs"));
    assert_eq!(mod_rs, read_golden("baseline_on_mod.rs"));

    // Structural assertions (beyond pure snapshot equality).
    assert!(
        api.contains("pub struct RequestContext(pub ::axum::http::Extensions);"),
        "must emit RequestContext once:\n{api}"
    );
    assert!(
        api.contains("pub fn get<T: ::std::clone::Clone + Send + Sync + 'static>"),
        "RequestContext::get convenience:\n{api}"
    );
    assert!(
        api.contains(
            "async fn ping(&self, ctx: RequestContext, body: PingRequest) -> PingResponse"
        ),
        "trait method must take leading ctx:\n{api}"
    );
    assert!(
        router.contains("let ctx = RequestContext(parts.extensions.clone());"),
        "handler must clone extensions from parts before body extraction:\n{router}"
    );
    assert!(
        router.contains("api.ping(ctx, body).await"),
        "handler must pass ctx first:\n{router}"
    );
    // Response enums unchanged in shape relative to off baseline.
    assert_eq!(
        errors,
        read_golden("baseline_off_errors.rs"),
        "errors.rs must be unchanged by the flag"
    );
    assert_eq!(
        mod_rs,
        read_golden("baseline_off_mod.rs"),
        "mod.rs must be unchanged by the flag"
    );
}

/// TOML default: omitted context_extensions is false.
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
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{content}").unwrap();
    let cfg = ConfigFile::load(f.path()).expect("load");
    assert!(!cfg.server.unwrap().context_extensions);
}
