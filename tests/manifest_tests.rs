use solana_idl_codegen::manifest::{load_manifest, validate_manifest};
use std::path::Path;

/// Test that the actual programs.json manifest loads and validates correctly.
#[test]
fn test_real_manifest_loads_and_validates() {
    let manifest_path = Path::new("manifests/programs.json");
    let manifest = load_manifest(manifest_path).expect("Failed to load programs.json");

    let manifest_dir = manifest_path.parent().unwrap();
    validate_manifest(&manifest, manifest_dir).expect("programs.json validation failed");

    assert_eq!(manifest.programs.len(), 5);
    assert_eq!(manifest.registry_crate, "solana_registry");
}

/// Test that all program names in the manifest are unique and snake_case.
#[test]
fn test_real_manifest_program_names() {
    let manifest_path = Path::new("manifests/programs.json");
    let manifest = load_manifest(manifest_path).unwrap();

    let expected_names = vec![
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    let names: Vec<&str> = manifest.programs.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, expected_names);
}

/// Test that all IDL paths in the real manifest resolve to existing files.
#[test]
fn test_real_manifest_idl_paths_exist() {
    let manifest_path = Path::new("manifests/programs.json");
    let manifest = load_manifest(manifest_path).unwrap();
    let manifest_dir = manifest_path.parent().unwrap();

    for entry in &manifest.programs {
        let idl_path = manifest_dir.join(&entry.idl);
        assert!(
            idl_path.exists(),
            "IDL path does not exist for '{}': {}",
            entry.name,
            idl_path.display()
        );
    }
}

/// Test that override paths in the real manifest resolve to existing files.
#[test]
fn test_real_manifest_override_paths_exist() {
    let manifest_path = Path::new("manifests/programs.json");
    let manifest = load_manifest(manifest_path).unwrap();
    let manifest_dir = manifest_path.parent().unwrap();

    for entry in &manifest.programs {
        if let Some(override_file) = &entry.override_file {
            let override_path = manifest_dir.join(override_file);
            assert!(
                override_path.exists(),
                "Override path does not exist for '{}': {}",
                entry.name,
                override_path.display()
            );
        }
    }
}

/// Test that only raydium_amm has an override file.
#[test]
fn test_real_manifest_overrides() {
    let manifest_path = Path::new("manifests/programs.json");
    let manifest = load_manifest(manifest_path).unwrap();

    let with_overrides: Vec<&str> = manifest
        .programs
        .iter()
        .filter(|p| p.override_file.is_some())
        .map(|p| p.name.as_str())
        .collect();

    assert_eq!(with_overrides, vec!["raydium_amm"]);
}
