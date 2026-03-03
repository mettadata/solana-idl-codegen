//! Workspace auto-wiring for generated crates.
//!
//! Automatically updates root `Cargo.toml` workspace members and
//! `[workspace.dependencies]` to include newly generated program crates
//! and the registry crate. Preserves existing entries and comments.

use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::{DocumentMut, Item, Value};

/// Ensure a program crate is registered in the workspace Cargo.toml.
///
/// Adds the crate path to `workspace.members` and a path dependency to
/// `workspace.dependencies` if not already present.
pub fn ensure_workspace_member(
    cargo_toml_path: &Path,
    crate_path: &str,
    crate_name: &str,
) -> Result<bool> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;

    let mut changed = false;

    // Add to workspace.members if missing
    if let Some(workspace) = doc.get_mut("workspace") {
        if let Some(members) = workspace.get_mut("members") {
            if let Some(arr) = members.as_array_mut() {
                if !arr.iter().any(|v| v.as_str() == Some(crate_path)) {
                    arr.push(crate_path);
                    changed = true;
                }
            }
        }

        // Add to workspace.dependencies if missing
        if let Some(deps) = workspace.get_mut("dependencies") {
            if let Some(table) = deps.as_table_like_mut() {
                if !table.contains_key(crate_name) {
                    let mut inline = toml_edit::InlineTable::new();
                    inline.insert("path", Value::from(format!("./{}", crate_path)));
                    table.insert(crate_name, Item::Value(Value::InlineTable(inline)));
                    changed = true;
                }
            }
        }
    }

    if changed {
        std::fs::write(cargo_toml_path, doc.to_string())
            .with_context(|| format!("Failed to write {}", cargo_toml_path.display()))?;
    }

    Ok(changed)
}

/// Ensure a dependency is present in a downstream Cargo.toml.
///
/// Adds `<crate_name>.workspace = true` to `[dependencies]` if not present.
pub fn ensure_dependency(cargo_toml_path: &Path, crate_name: &str) -> Result<bool> {
    let content = std::fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;

    let mut changed = false;

    if let Some(deps) = doc.get_mut("dependencies") {
        if let Some(table) = deps.as_table_like_mut() {
            if !table.contains_key(crate_name) {
                let mut inline = toml_edit::InlineTable::new();
                inline.insert("workspace", Value::from(true));
                table.insert(crate_name, Item::Value(Value::InlineTable(inline)));
                changed = true;
            }
        }
    }

    if changed {
        std::fs::write(cargo_toml_path, doc.to_string())
            .with_context(|| format!("Failed to write {}", cargo_toml_path.display()))?;
    }

    Ok(changed)
}

/// Wire all generated program crates and the registry into the workspace.
///
/// - Adds each program to workspace members and dependencies
/// - Adds the registry crate to workspace members and dependencies
/// - Adds the registry as a dependency of the downstream consumer crate
pub fn wire_workspace(
    root_cargo_toml: &Path,
    programs: &[(String, String)], // (name, relative_path)
    registry_name: &str,
    registry_path: &str,
    downstream_cargo_toml: Option<&Path>,
) -> Result<()> {
    // Wire each program crate
    for (name, path) in programs {
        let changed = ensure_workspace_member(root_cargo_toml, path, name)?;
        if changed {
            eprintln!("  + Added {} to workspace", name);
        }
    }

    // Wire the registry crate
    let changed = ensure_workspace_member(root_cargo_toml, registry_path, registry_name)?;
    if changed {
        eprintln!("  + Added {} to workspace", registry_name);
    }

    // Wire registry as dependency of downstream crate (e.g., chains/solana)
    if let Some(downstream) = downstream_cargo_toml {
        let changed = ensure_dependency(downstream, registry_name)?;
        if changed {
            eprintln!(
                "  + Added {} dependency to {}",
                registry_name,
                downstream.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_workspace_toml(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("Cargo.toml");
        fs::write(
            &path,
            r#"[workspace]
members = [
    "existing_crate",
]

[workspace.dependencies]
existing_crate = { path = "./existing_crate" }
"#,
        )
        .unwrap();
        path
    }

    #[test]
    fn test_ensure_workspace_member_adds_new() {
        let dir = TempDir::new().unwrap();
        let toml_path = create_workspace_toml(dir.path());

        let changed =
            ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();

        assert!(changed);
        let content = fs::read_to_string(&toml_path).unwrap();
        assert!(content.contains("interfaces/solana/pumpfun"));
        assert!(content.contains("pumpfun"));
    }

    #[test]
    fn test_ensure_workspace_member_idempotent() {
        let dir = TempDir::new().unwrap();
        let toml_path = create_workspace_toml(dir.path());

        // First call adds
        ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();
        // Second call is idempotent
        let changed =
            ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();

        assert!(!changed);
    }

    #[test]
    fn test_ensure_workspace_member_preserves_existing() {
        let dir = TempDir::new().unwrap();
        let toml_path = create_workspace_toml(dir.path());

        ensure_workspace_member(&toml_path, "interfaces/solana/pumpfun", "pumpfun").unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        assert!(content.contains("existing_crate"));
    }

    #[test]
    fn test_ensure_dependency_adds_new() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
existing = { workspace = true }
"#,
        )
        .unwrap();

        let changed = ensure_dependency(&path, "solana_registry").unwrap();

        assert!(changed);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("solana_registry"));
    }

    #[test]
    fn test_ensure_dependency_idempotent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
solana_registry = { workspace = true }
"#,
        )
        .unwrap();

        let changed = ensure_dependency(&path, "solana_registry").unwrap();
        assert!(!changed);
    }
}
