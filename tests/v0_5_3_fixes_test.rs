//! Golden fixtures for the v0.5.3 maintenance release:
//! 1. Dependency modernization (default features free of native-tls / openssl /
//!    rustls-pemfile / archived serde_yaml) is gated by a dedicated unit test
//!    that shells out to `cargo tree`.
//! 2. allOf if/then conditional-required: merged struct stays; doc + warning.
//! 3. Boolean schema parse gap (`properties: { split: false }`) that used to
//!    fail untagged `Schema` deserialization for full customer contracts.

use openapi_to_rust::openapi::{OpenApiSpec, Schema};
use openapi_to_rust::{CodeGenerator, GeneratorConfig, SchemaAnalyzer};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v0_5_3")
}

fn load_fixture(name: &str) -> Value {
    let path = fixtures_dir().join(name);
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn generate_types(spec: Value) -> String {
    let mut analyzer = SchemaAnalyzer::new(spec).expect("analyzer");
    let mut analysis = analyzer.analyze().expect("analyze");
    let generator = CodeGenerator::new(GeneratorConfig {
        module_name: "v053".into(),
        enable_async_client: false,
        ..Default::default()
    });
    generator.generate(&mut analysis).expect("generate")
}

/// Fix 3: boolean schemas (`false` under `then.properties`) parse, and the
/// minimized HoldAction fixture analyzes + generates.
#[test]
fn boolean_schema_hold_action_parses_and_generates() {
    let schema_json = serde_json::json!({
        "type": "object",
        "properties": {
            "split": false
        }
    });
    let parsed: Schema = serde_json::from_value(schema_json).expect("boolean property schema");
    match parsed {
        Schema::Typed { details, .. } => {
            let split = details
                .properties
                .as_ref()
                .expect("properties")
                .get("split")
                .expect("split prop");
            assert!(
                matches!(split, Schema::Boolean(false)),
                "split must be Schema::Boolean(false), got {split:?}"
            );
        }
        other => panic!("expected Typed schema, got {other:?}"),
    }

    let bare: Schema = serde_json::from_value(serde_json::json!(false)).expect("bare false");
    assert!(matches!(bare, Schema::Boolean(false)));
    let bare_true: Schema = serde_json::from_value(serde_json::json!(true)).expect("bare true");
    assert!(matches!(bare_true, Schema::Boolean(true)));

    let spec = load_fixture("boolean_schema_hold_action.json");
    serde_json::from_value::<OpenApiSpec>(spec.clone()).expect("HoldAction OpenApiSpec parse");

    let types = generate_types(spec);
    assert!(
        types.contains("pub struct HoldAction"),
        "HoldAction struct missing from:\n{types}"
    );
    assert!(
        types.contains("Conditional requirements (if/then)")
            || types.contains("if/then")
            || types.contains("const=cancel")
            || types.contains("const=\\\"cancel\\\""),
        "expected if/then documentation on HoldAction, got:\n{types}"
    );
    // `split` remains a normal optional field on the merged struct; the
    // `then: { properties: { split: false } }` forbid is documented, not typed.
    assert!(
        types.contains("split"),
        "split field should remain on the merged struct:\n{types}"
    );
}

/// Fix 2: allOf if/then conditional required is documented + warned, not
/// silently dropped with no trace. Unconditional `required` stays; conditional
/// `then.required` is NOT promoted into struct-level required fields.
#[test]
fn allof_if_then_conditional_required_documented() {
    let spec = load_fixture("allof_if_then_conditional_required.json");
    serde_json::from_value::<OpenApiSpec>(spec.clone())
        .expect("CreateBankDetails OpenApiSpec parse");

    let mut analyzer = SchemaAnalyzer::new(spec.clone()).expect("analyzer");
    let analysis = analyzer.analyze().expect("analyze");
    let create = analysis
        .schemas
        .get("CreateBankDetails")
        .expect("CreateBankDetails analyzed");

    // Unconditional required stays required.
    match &create.schema_type {
        openapi_to_rust::analysis::SchemaType::Object { required, .. } => {
            assert!(required.contains("role"));
            assert!(required.contains("account_type"));
            // Conditional then.required must NOT be promoted.
            assert!(
                !required.contains("routing_type"),
                "routing_type must stay optional (conditional only)"
            );
            assert!(
                !required.contains("routing_code"),
                "routing_code must stay optional (conditional only)"
            );
        }
        other => panic!("expected Object, got {other:?}"),
    }

    let desc = create.description.as_deref().unwrap_or("");
    assert!(
        desc.contains("Conditional requirements (if/then)")
            || desc.contains("routing_type")
            || desc.contains("if "),
        "description must name the dropped condition: {desc}"
    );
    assert!(
        desc.contains("routing_type") && desc.contains("routing_code"),
        "description must list conditional required fields: {desc}"
    );

    let types = generate_types(spec);
    assert!(types.contains("pub struct CreateBankDetails"));
    // Optional fields: routing_* should be Option in generated Rust.
    assert!(
        types.contains("routing_type") && types.contains("Option"),
        "routing_type should be optional Option on the merged struct:\n{types}"
    );
    assert!(
        types.contains("Conditional requirements (if/then)")
            || types.contains("not(const=")
            || types.contains("required=[routing_type"),
        "generated rustdoc should carry the if/then note:\n{types}"
    );
}

/// Fix 3 (full-contract angle): the minimized boolean-schema repro is also the
/// schema that made full customerapi openapi.yaml fail untagged Schema parse.
/// Keep a second assertion path that only deserializes the isolated then branch.
#[test]
fn then_properties_boolean_false_is_schema_boolean() {
    let then_only = serde_json::json!({
        "properties": {
            "split": false
        }
    });
    let schema: Schema = serde_json::from_value(then_only).expect("then branch");
    let details = schema.details();
    let split = details
        .properties
        .as_ref()
        .and_then(|p| p.get("split"))
        .expect("split");
    assert!(matches!(split, Schema::Boolean(false)));
}

/// Full customer contract (when present on disk next to a triple-a checkout)
/// must parse end-to-end. The minimized boolean-schema fixture above is the
/// permanent CI pin; this test is the integration check against the real file.
#[test]
fn full_customer_contract_parses_when_available() {
    let candidates = [
        // Worktree path used during the v0.5.3 fix development.
        "/Users/s/dev/triple-a/triple-a-rust/.claude/worktrees/triple-a-rust/Onboarding/product/customerapi/specs/openapi.yaml",
        // Common monorepo layout relative to a sibling checkout.
        "../triple-a-rust/product/customerapi/specs/openapi.yaml",
    ];
    let Some(path) = candidates.iter().map(PathBuf::from).find(|p| p.is_file()) else {
        eprintln!("full customer openapi.yaml not found; skipping integration parse check");
        return;
    };
    let content = fs::read_to_string(&path).expect("read full contract");
    let value = openapi_to_rust::spec_source::yaml_to_json_value(&content).expect("yaml→json");
    serde_json::from_value::<OpenApiSpec>(value.clone())
        .unwrap_or_else(|e| panic!("full OpenApiSpec parse of {}: {e}", path.display()));
    let mut analyzer = SchemaAnalyzer::new(value).expect("analyzer");
    let analysis = analyzer.analyze().expect("analyze full contract");
    assert!(
        analysis.schemas.contains_key("HoldAction"),
        "HoldAction must be analyzed in full contract"
    );
    assert!(
        analysis.schemas.contains_key("CreateBankDetails"),
        "CreateBankDetails must be analyzed in full contract"
    );
    assert!(
        analysis.schemas.len() > 100,
        "expected the full ~216-schema surface, got {}",
        analysis.schemas.len()
    );
}

/// Fix 1 acceptance: the library-only dependency graph must not contain
/// reqwest, native-tls, openssl, rustls-pemfile, or archived serde_yaml.
///
/// Upstream v0.12 intentionally enables the CLI by default; library and
/// build-dependency consumers select `--no-default-features`.
#[test]
fn library_dependency_graph_has_no_http_or_archived_yaml() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let output = Command::new("cargo")
        .args([
            "tree",
            "--no-default-features",
            "--prefix",
            "none",
            "--edges",
            "normal",
        ])
        .current_dir(manifest)
        .output()
        .expect("cargo tree");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout);

    for forbidden in [
        "native-tls",
        "openssl",
        "rustls-pemfile",
        // Archived crate name appears as `serde_yaml v…` (not serde_yaml_ng).
        "serde_yaml v",
    ] {
        assert!(
            !tree.lines().any(|l| l.contains(forbidden)),
            "library-only cargo tree must not contain `{forbidden}`:\n{tree}"
        );
    }

    // Positive checks: we want the maintained yaml fork, and no reqwest by default.
    assert!(
        tree.contains("serde_yaml_ng"),
        "expected serde_yaml_ng in default tree:\n{tree}"
    );
    assert!(
        !tree.lines().any(|l| l.starts_with("reqwest ")),
        "library-only graph must not pull reqwest:\n{tree}"
    );
}
