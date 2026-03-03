use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Program manifest — single source of truth for codegen.
///
/// Lists all Solana programs with their IDL and optional override paths.
/// Used by `--manifest` batch mode to generate all programs in one invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// List of programs to generate code for
    pub programs: Vec<ProgramEntry>,

    /// Output directory for generated crates (relative to manifest file)
    pub output_dir: String,

    /// Name of the registry crate to generate
    pub registry_crate: String,
}

/// A single program entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramEntry {
    /// Module name (snake_case, e.g., "pumpfun")
    pub name: String,

    /// Relative path to IDL JSON file (relative to manifest file's parent directory)
    pub idl: String,

    /// Optional relative path to override JSON file
    #[serde(rename = "override")]
    pub override_file: Option<String>,
}

/// Load and parse a manifest file from disk.
pub fn load_manifest(path: &Path) -> Result<Manifest> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read manifest file: {}", path.display()))?;

    let manifest: Manifest = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse manifest JSON: {}", path.display()))?;

    Ok(manifest)
}

/// Validate a manifest for correctness.
///
/// Checks:
/// - At least one program defined
/// - No duplicate program names
/// - All IDL paths exist (resolved relative to manifest_dir)
/// - All override paths exist if specified (resolved relative to manifest_dir)
/// - Program names are valid Rust identifiers (snake_case, non-empty)
pub fn validate_manifest(manifest: &Manifest, manifest_dir: &Path) -> Result<()> {
    if manifest.programs.is_empty() {
        bail!("Manifest must contain at least one program");
    }

    // Check for duplicate names
    let mut seen_names = HashSet::new();
    for entry in &manifest.programs {
        if !seen_names.insert(&entry.name) {
            bail!("Duplicate program name in manifest: '{}'", entry.name);
        }
    }

    // Validate each program entry
    for entry in &manifest.programs {
        validate_program_entry(entry, manifest_dir)?;
    }

    // Validate output_dir is non-empty
    if manifest.output_dir.is_empty() {
        bail!("Manifest output_dir must not be empty");
    }

    // Validate registry_crate is non-empty
    if manifest.registry_crate.is_empty() {
        bail!("Manifest registry_crate must not be empty");
    }

    Ok(())
}

/// Validate a single program entry.
fn validate_program_entry(entry: &ProgramEntry, manifest_dir: &Path) -> Result<()> {
    // Name must be non-empty and valid snake_case
    if entry.name.is_empty() {
        bail!("Program name must not be empty");
    }
    if entry.name != entry.name.to_lowercase()
        || entry.name.contains(' ')
        || entry.name.contains('-')
    {
        bail!(
            "Program name '{}' must be snake_case (lowercase, no spaces or hyphens)",
            entry.name
        );
    }

    // IDL path must exist
    let idl_path = manifest_dir.join(&entry.idl);
    if !idl_path.exists() {
        bail!(
            "IDL file not found for program '{}': {}",
            entry.name,
            idl_path.display()
        );
    }

    // Override path must exist if specified
    if let Some(override_path) = &entry.override_file {
        let override_full = manifest_dir.join(override_path);
        if !override_full.exists() {
            bail!(
                "Override file not found for program '{}': {}",
                entry.name,
                override_full.display()
            );
        }
    }

    Ok(())
}

/// Resolve a program entry's IDL path to an absolute path.
pub fn resolve_idl_path(entry: &ProgramEntry, manifest_dir: &Path) -> PathBuf {
    manifest_dir.join(&entry.idl)
}

/// Resolve a program entry's override path to an absolute path, if present.
pub fn resolve_override_path(entry: &ProgramEntry, manifest_dir: &Path) -> Option<PathBuf> {
    entry
        .override_file
        .as_ref()
        .map(|p| manifest_dir.join(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_manifest(dir: &Path, content: &str) -> PathBuf {
        let manifest_path = dir.join("programs.json");
        fs::write(&manifest_path, content).unwrap();
        manifest_path
    }

    fn create_test_files(dir: &Path, files: &[&str]) {
        for file in files {
            let path = dir.join(file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, "{}").unwrap();
        }
    }

    #[test]
    fn test_load_manifest_valid() {
        let dir = TempDir::new().unwrap();
        let manifest = create_test_manifest(
            dir.path(),
            r#"{
                "programs": [
                    {"name": "pumpfun", "idl": "idl/pump.json"},
                    {"name": "raydium_amm", "idl": "idl/raydium.json", "override": "overrides/raydium.json"}
                ],
                "output_dir": "../../interfaces/solana",
                "registry_crate": "solana_registry"
            }"#,
        );

        let result = load_manifest(&manifest).unwrap();
        assert_eq!(result.programs.len(), 2);
        assert_eq!(result.programs[0].name, "pumpfun");
        assert_eq!(result.programs[1].override_file.as_deref(), Some("overrides/raydium.json"));
        assert_eq!(result.output_dir, "../../interfaces/solana");
        assert_eq!(result.registry_crate, "solana_registry");
    }

    #[test]
    fn test_load_manifest_missing_file() {
        let result = load_manifest(Path::new("/nonexistent/manifest.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_manifest_invalid_json() {
        let dir = TempDir::new().unwrap();
        let manifest = create_test_manifest(dir.path(), "not json");
        let result = load_manifest(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_manifest_valid() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path(), &["idl/pump.json", "overrides/pump.json"]);

        let manifest = Manifest {
            programs: vec![ProgramEntry {
                name: "pumpfun".to_string(),
                idl: "idl/pump.json".to_string(),
                override_file: Some("overrides/pump.json".to_string()),
            }],
            output_dir: "../../interfaces/solana".to_string(),
            registry_crate: "solana_registry".to_string(),
        };

        assert!(validate_manifest(&manifest, dir.path()).is_ok());
    }

    #[test]
    fn test_validate_manifest_empty_programs() {
        let dir = TempDir::new().unwrap();
        let manifest = Manifest {
            programs: vec![],
            output_dir: "output".to_string(),
            registry_crate: "registry".to_string(),
        };

        let result = validate_manifest(&manifest, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one program"));
    }

    #[test]
    fn test_validate_manifest_duplicate_names() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path(), &["idl/a.json", "idl/b.json"]);

        let manifest = Manifest {
            programs: vec![
                ProgramEntry {
                    name: "pumpfun".to_string(),
                    idl: "idl/a.json".to_string(),
                    override_file: None,
                },
                ProgramEntry {
                    name: "pumpfun".to_string(),
                    idl: "idl/b.json".to_string(),
                    override_file: None,
                },
            ],
            output_dir: "output".to_string(),
            registry_crate: "registry".to_string(),
        };

        let result = validate_manifest(&manifest, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate program name"));
    }

    #[test]
    fn test_validate_manifest_missing_idl() {
        let dir = TempDir::new().unwrap();
        let manifest = Manifest {
            programs: vec![ProgramEntry {
                name: "pumpfun".to_string(),
                idl: "idl/nonexistent.json".to_string(),
                override_file: None,
            }],
            output_dir: "output".to_string(),
            registry_crate: "registry".to_string(),
        };

        let result = validate_manifest(&manifest, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("IDL file not found"));
    }

    #[test]
    fn test_validate_manifest_missing_override() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path(), &["idl/pump.json"]);

        let manifest = Manifest {
            programs: vec![ProgramEntry {
                name: "pumpfun".to_string(),
                idl: "idl/pump.json".to_string(),
                override_file: Some("overrides/nonexistent.json".to_string()),
            }],
            output_dir: "output".to_string(),
            registry_crate: "registry".to_string(),
        };

        let result = validate_manifest(&manifest, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Override file not found"));
    }

    #[test]
    fn test_validate_manifest_invalid_name_hyphen() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path(), &["idl/pump.json"]);

        let manifest = Manifest {
            programs: vec![ProgramEntry {
                name: "pump-fun".to_string(),
                idl: "idl/pump.json".to_string(),
                override_file: None,
            }],
            output_dir: "output".to_string(),
            registry_crate: "registry".to_string(),
        };

        let result = validate_manifest(&manifest, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("snake_case"));
    }

    #[test]
    fn test_validate_manifest_empty_output_dir() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path(), &["idl/pump.json"]);

        let manifest = Manifest {
            programs: vec![ProgramEntry {
                name: "pumpfun".to_string(),
                idl: "idl/pump.json".to_string(),
                override_file: None,
            }],
            output_dir: "".to_string(),
            registry_crate: "registry".to_string(),
        };

        let result = validate_manifest(&manifest, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("output_dir"));
    }

    #[test]
    fn test_resolve_idl_path() {
        let entry = ProgramEntry {
            name: "pumpfun".to_string(),
            idl: "idl/pump.json".to_string(),
            override_file: None,
        };

        let resolved = resolve_idl_path(&entry, Path::new("/workspace/codegen"));
        assert_eq!(resolved, PathBuf::from("/workspace/codegen/idl/pump.json"));
    }

    #[test]
    fn test_resolve_override_path() {
        let entry_with = ProgramEntry {
            name: "pumpfun".to_string(),
            idl: "idl/pump.json".to_string(),
            override_file: Some("overrides/pump.json".to_string()),
        };
        let entry_without = ProgramEntry {
            name: "pumpfun".to_string(),
            idl: "idl/pump.json".to_string(),
            override_file: None,
        };

        assert_eq!(
            resolve_override_path(&entry_with, Path::new("/workspace")),
            Some(PathBuf::from("/workspace/overrides/pump.json"))
        );
        assert_eq!(resolve_override_path(&entry_without, Path::new("/workspace")), None);
    }
}
