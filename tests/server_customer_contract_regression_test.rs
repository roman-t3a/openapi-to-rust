use openapi_to_rust::config::ServerSection;
use openapi_to_rust::server::codegen::ServerCodegen;
use openapi_to_rust::{CodeGenerator, GeneratorConfig, SchemaAnalyzer};

fn customer_contract_spec() -> serde_json::Value {
    serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Customer API contract fixture",
            "version": "1.0.0"
        },
        "paths": {
            "/v1/entities": {
                "post": {
                    "operationId": "createEntity",
                    "tags": ["Entities"],
                    "parameters": [
                        {
                            "name": "type",
                            "in": "query",
                            "required": true,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "Idempotency-Key",
                            "in": "header",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateEntity" }
                            }
                        }
                    },
                    "responses": {
                        "201": { "$ref": "#/components/responses/EntityCreated" },
                        "403": { "$ref": "#/components/responses/PublicForbidden" }
                    }
                }
            }
        },
        "components": {
            "responses": {
                "EntityCreated": {
                    "description": "Created",
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/EntityRef" }
                        }
                    }
                },
                "PublicForbidden": {
                    "description": "Forbidden",
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/PublicErrorEnvelope" }
                        }
                    }
                }
            },
            "schemas": {
                "CreateEntity": {
                    "type": "object",
                    "required": ["entity_type"],
                    "properties": {
                        "entity_type": { "type": "string" }
                    }
                },
                "EntityRef": {
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": { "type": "string" }
                    }
                },
                "PublicErrorEnvelope": {
                    "type": "object",
                    "required": ["error"],
                    "properties": {
                        "error": { "$ref": "#/components/schemas/PublicError" }
                    }
                },
                "PublicError": {
                    "type": "object",
                    "required": ["message"],
                    "properties": {
                        "message": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn generated_server_files()
-> Result<Vec<openapi_to_rust::GeneratedFile>, Box<dyn std::error::Error>> {
    let mut analyzer = SchemaAnalyzer::new(customer_contract_spec())?;
    let analysis = analyzer.analyze()?;
    let config = GeneratorConfig {
        enable_async_client: false,
        server: Some(ServerSection {
            framework: "axum".to_string(),
            operations: vec!["createEntity".to_string()],
            prune_models: false,
            context_extensions: false,
        }),
        ..Default::default()
    };
    let generator = CodeGenerator::new(config);
    let Some(server) = generator.config().server.as_ref() else {
        return Err(std::io::Error::other("missing server config").into());
    };

    Ok(ServerCodegen::new(generator.config(), &analysis, server).generate()?)
}

fn generated_file<'a>(
    files: &'a [openapi_to_rust::GeneratedFile],
    path: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    Ok(files
        .iter()
        .find(|file| file.path.as_path() == std::path::Path::new(path))
        .ok_or_else(|| std::io::Error::other(format!("missing generated file {path}")))?
        .content
        .as_str())
}

#[test]
fn server_codegen_uses_axum_08_compatible_async_trait_path()
-> Result<(), Box<dyn std::error::Error>> {
    let files = generated_server_files()?;
    let api = generated_file(&files, "server/api.rs")?;

    assert!(
        api.contains("#[async_trait::async_trait]"),
        "server traits should not depend on axum re-exporting async_trait:\n{api}"
    );
    assert!(
        !api.contains("#[axum::async_trait]"),
        "server traits should be compatible with axum 0.8:\n{api}"
    );
    Ok(())
}

#[test]
fn server_codegen_escapes_keyword_parameter_idents_consistently()
-> Result<(), Box<dyn std::error::Error>> {
    let files = generated_server_files()?;
    let api = generated_file(&files, "server/api.rs")?;
    let router = generated_file(&files, "server/router.rs")?;

    assert!(
        api.contains("r#type: String"),
        "trait signature should escape query parameter `type`:\n{api}"
    );
    assert!(
        api.contains("idempotency_key: String"),
        "trait signature should keep header parameter as snake_case:\n{api}"
    );
    assert!(
        router.contains("pub r#type: ::std::option::Option<String>"),
        "query extractor struct should escape field `type`:\n{router}"
    );
    assert!(
        router.contains("let r#type = match __q.r#type"),
        "handler should read the escaped query field consistently:\n{router}"
    );
    assert!(
        router.contains("api.create_entity(r#type, idempotency_key, body).await"),
        "handler should pass escaped query and header idents to the trait:\n{router}"
    );
    Ok(())
}

#[test]
fn server_codegen_keeps_component_response_refs_as_typed_variants()
-> Result<(), Box<dyn std::error::Error>> {
    let files = generated_server_files()?;
    let errors = generated_file(&files, "server/errors.rs")?;

    assert!(
        errors.contains("Created(EntityRef)"),
        "201 component response ref should generate a typed response variant:\n{errors}"
    );
    assert!(
        errors.contains("Forbidden(PublicErrorEnvelope)"),
        "403 component response ref should generate a typed response variant:\n{errors}"
    );
    Ok(())
}

#[test]
fn server_generation_advises_framework_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let mut analyzer = SchemaAnalyzer::new(customer_contract_spec())?;
    let mut analysis = analyzer.analyze()?;
    let config = GeneratorConfig {
        enable_async_client: false,
        server: Some(ServerSection {
            framework: "axum".to_string(),
            operations: vec!["createEntity".to_string()],
            prune_models: false,
            context_extensions: false,
        }),
        ..Default::default()
    };

    let result = CodeGenerator::new(config).generate_all(&mut analysis)?;
    let deps: Vec<&str> = result
        .required_deps
        .iter()
        .map(|dep| dep.crate_name)
        .collect();

    assert!(
        deps.contains(&"async-trait"),
        "server REQUIRED_DEPS.toml should include async-trait: {deps:?}"
    );
    assert!(
        deps.contains(&"axum"),
        "server REQUIRED_DEPS.toml should include axum: {deps:?}"
    );
    assert!(
        deps.contains(&"serde"),
        "server REQUIRED_DEPS.toml should include serde: {deps:?}"
    );
    assert!(
        deps.contains(&"serde_json"),
        "server REQUIRED_DEPS.toml should include serde_json: {deps:?}"
    );
    Ok(())
}
