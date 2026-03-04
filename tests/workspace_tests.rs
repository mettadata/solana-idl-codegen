//! Integration tests for workspace wiring (T043-T045, T050).

use solana_idl_codegen::workspace::{ensure_dependency, ensure_workspace_member, wire_workspace};
use std::fs;
use tempfile::TempDir;

/// T043: Verify new program appears in workspace members list.
#[test]
fn test_workspace_member_insertion() {
    let dir = TempDir::new().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    fs::write(
        &toml_path,
        r#"[workspace]
members = [
    "core",
]

[workspace.dependencies]
core = { path = "./core" }
"#,
    )
    .unwrap();

    let changed =
        ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();

    assert!(changed, "First insertion should report changed");

    let content = fs::read_to_string(&toml_path).unwrap();
    assert!(
        content.contains("interfaces/solana/pumpfun"),
        "New member should appear in members array"
    );
    assert!(
        content.contains("pumpfun"),
        "New dependency should appear in workspace.dependencies"
    );
}

/// T044: Verify new program appears in downstream Cargo.toml dependencies.
#[test]
fn test_dependency_insertion() {
    let dir = TempDir::new().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    fs::write(
        &toml_path,
        r#"[package]
name = "chains-solana"
version = "0.1.0"

[dependencies]
pandora_core = { workspace = true }
"#,
    )
    .unwrap();

    let changed = ensure_dependency(&toml_path, "solana_registry").unwrap();

    assert!(changed, "First insertion should report changed");

    let content = fs::read_to_string(&toml_path).unwrap();
    assert!(
        content.contains("solana_registry"),
        "New dependency should appear in [dependencies]"
    );
    assert!(
        content.contains("workspace = true"),
        "Dependency should use workspace reference"
    );
}

/// T045: Verify codegen doesn't remove existing entries from Cargo.toml.
#[test]
fn test_existing_entry_preservation() {
    let dir = TempDir::new().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    fs::write(
        &toml_path,
        r#"[workspace]
members = [
    "core",
    "chains/solana",
    "services/gateway",
]

[workspace.dependencies]
core = { path = "./core" }
chains-solana = { path = "./chains/solana" }
"#,
    )
    .unwrap();

    // Add a new member
    ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();

    let content = fs::read_to_string(&toml_path).unwrap();

    // All existing entries should still be present
    assert!(content.contains("core"), "core should still exist");
    assert!(
        content.contains("chains/solana"),
        "chains/solana should still exist"
    );
    assert!(
        content.contains("services/gateway"),
        "services/gateway should still exist"
    );

    // New entry should also be present
    assert!(
        content.contains("interfaces/solana/pumpfun"),
        "New member should be added"
    );
}

/// T045 (continued): Verify idempotent — second call doesn't duplicate.
#[test]
fn test_workspace_wiring_idempotent() {
    let dir = TempDir::new().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    fs::write(
        &toml_path,
        r#"[workspace]
members = [
    "core",
]

[workspace.dependencies]
core = { path = "./core" }
"#,
    )
    .unwrap();

    // Wire twice
    ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();
    let changed =
        ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();

    assert!(!changed, "Second call should not report changes");

    let content = fs::read_to_string(&toml_path).unwrap();
    // The path appears in both members and dependencies, so count occurrences
    // in just the members array to verify no duplication there.
    let members_section = content
        .split("[workspace.dependencies]")
        .next()
        .unwrap_or(&content);
    let count = members_section
        .matches("interfaces/solana/pumpfun")
        .count();
    assert_eq!(
        count, 1,
        "Should appear exactly once in members, not duplicated"
    );
}

/// T050 (partial): Wire multiple programs and registry via wire_workspace().
#[test]
fn test_wire_workspace_multiple_programs() {
    let dir = TempDir::new().unwrap();

    // Create root Cargo.toml
    let root_toml = dir.path().join("Cargo.toml");
    fs::write(
        &root_toml,
        r#"[workspace]
members = [
    "core",
]

[workspace.dependencies]
core = { path = "./core" }
"#,
    )
    .unwrap();

    // Create downstream Cargo.toml
    let downstream_toml = dir.path().join("chains_solana_Cargo.toml");
    fs::write(
        &downstream_toml,
        r#"[package]
name = "chains-solana"
version = "0.1.0"

[dependencies]
pandora_core = { workspace = true }
"#,
    )
    .unwrap();

    let programs = vec![
        (
            "pumpfun".to_string(),
            "interfaces/solana/pumpfun".to_string(),
        ),
        (
            "raydium_clmm".to_string(),
            "interfaces/solana/raydium_clmm".to_string(),
        ),
    ];

    wire_workspace(
        &root_toml,
        &programs,
        "solana_registry",
        "interfaces/solana/solana_registry",
        Some(&downstream_toml),
    )
    .unwrap();

    // Verify root Cargo.toml
    let root_content = fs::read_to_string(&root_toml).unwrap();
    assert!(root_content.contains("interfaces/solana/pumpfun"));
    assert!(root_content.contains("interfaces/solana/raydium_clmm"));
    assert!(root_content.contains("interfaces/solana/solana_registry"));

    // Verify downstream Cargo.toml
    let downstream_content = fs::read_to_string(&downstream_toml).unwrap();
    assert!(
        downstream_content.contains("solana_registry"),
        "Registry should be added as dependency"
    );
    assert!(
        downstream_content.contains("pandora_core"),
        "Existing deps should be preserved"
    );
}
