//! IDL Override System
//!
//! This module provides functionality to override missing or incorrect data in Solana IDL files
//! without modifying the upstream IDL sources. Supports program address and discriminator overrides.
//!
//! # Overview
//!
//! The override system allows developers to:
//! - Add missing program addresses to IDL files
//! - Correct incorrect program addresses (e.g., devnet vs mainnet)
//! - Fix incorrect account discriminators
//! - Fix incorrect event discriminators
//! - Fix incorrect instruction discriminators
//!
//! Override files are JSON files that follow convention-based discovery:
//! - `./overrides/{idl_name}.json` - Per-IDL override file
//! - `./idl-overrides.json` - Global fallback override file
//! - Explicit path via `--override-file` CLI flag
//!
//! # Example
//!
//! ```json
//! {
//!   "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
//!   "accounts": {
//!     "PoolState": {
//!       "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
//!     }
//!   }
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Root structure representing a complete override file for a single IDL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideFile {
    /// Optional program address override (base58-encoded Pubkey)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_address: Option<String>,

    /// Account discriminator overrides (account name → discriminator)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub accounts: HashMap<String, DiscriminatorOverride>,

    /// Event discriminator overrides (event name → discriminator)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub events: HashMap<String, DiscriminatorOverride>,

    /// Instruction discriminator overrides (instruction name → discriminator)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub instructions: HashMap<String, DiscriminatorOverride>,
}

/// Represents an 8-byte discriminator override for an account, event, or instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscriminatorOverride {
    /// 8-byte discriminator array
    pub discriminator: [u8; 8],
}

/// Result of override file discovery process
#[derive(Debug, Clone)]
pub enum OverrideDiscovery {
    /// Override file found at path
    Found(PathBuf),

    /// No override file found (not an error)
    NotFound,

    /// Multiple override files detected (error)
    Conflict {
        files: Vec<PathBuf>,
        sources: Vec<String>, // e.g., "convention-based", "explicit CLI"
    },
}

/// Validation errors for override files
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid program address: {address}. Must be valid base58-encoded Pubkey.")]
    InvalidProgramAddress { address: String },

    #[error("Invalid program address: {address}. Cannot be system default pubkey.")]
    SystemDefaultPubkey { address: String },

    #[error("Invalid discriminator for {entity_type} '{entity_name}': must be exactly 8 bytes")]
    InvalidDiscriminatorLength {
        entity_type: String,
        entity_name: String,
    },

    #[error("Invalid discriminator for {entity_type} '{entity_name}': cannot be all zeros")]
    AllZeroDiscriminator {
        entity_type: String,
        entity_name: String,
    },

    #[error("Empty override file: must contain at least one override")]
    EmptyOverrideFile,

    #[error("Unknown {entity_type} '{entity_name}' in override file. Available: {available}")]
    UnknownEntity {
        entity_type: String,
        entity_name: String,
        available: String,
    },
}

/// Tracks which overrides were successfully applied (for logging/debugging)
#[derive(Debug, Clone)]
pub struct AppliedOverride {
    pub override_type: OverrideType,
    pub entity_name: Option<String>, // None for program_address
    pub original_value: Option<String>,
    pub override_value: String,
}

#[derive(Debug, Clone)]
pub enum OverrideType {
    ProgramAddress,
    AccountDiscriminator,
    EventDiscriminator,
    InstructionDiscriminator,
}

// Public API functions
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Discover override file location using convention-based search or explicit path
///
/// # Discovery Order
/// 1. If `explicit_override` provided: use that exclusively (highest priority, bypasses convention)
/// 2. Convention-based: check `./overrides/{idl_name}.json`
/// 3. Global fallback: check `./idl-overrides.json`
///
/// # Returns
/// - `OverrideDiscovery::Found(path)` if override file found
/// - `OverrideDiscovery::NotFound` if no override file found (not an error)
/// - `OverrideDiscovery::Conflict` if multiple convention-based override files detected
pub fn discover_override_file(
    _idl_path: &Path,
    idl_name: &str,
    explicit_override: Option<&Path>,
) -> Result<OverrideDiscovery> {
    // If explicit override provided, use it exclusively (highest priority)
    if let Some(explicit_path) = explicit_override {
        if explicit_path.exists() {
            return Ok(OverrideDiscovery::Found(explicit_path.to_path_buf()));
        } else {
            return Ok(OverrideDiscovery::NotFound);
        }
    }

    // Otherwise, check convention-based discovery
    let mut found_files = Vec::new();
    let mut sources = Vec::new();

    // Check convention-based per-IDL file: ./overrides/{idl_name}.json
    let convention_path = PathBuf::from(format!("./overrides/{}.json", idl_name));
    if convention_path.exists() {
        found_files.push(convention_path.clone());
        sources.push("convention-based discovery".to_string());
    }

    // Check global fallback: ./idl-overrides.json
    let global_path = PathBuf::from("./idl-overrides.json");
    if global_path.exists() && !found_files.contains(&global_path) {
        found_files.push(global_path.clone());
        sources.push("global fallback".to_string());
    }

    // Return result based on found files
    match found_files.len() {
        0 => Ok(OverrideDiscovery::NotFound),
        1 => Ok(OverrideDiscovery::Found(found_files[0].clone())),
        _ => Ok(OverrideDiscovery::Conflict {
            files: found_files,
            sources,
        }),
    }
}

/// Load and parse override file from disk
///
/// # Errors
/// - File not found or cannot be read
/// - Invalid JSON syntax
/// - JSON structure doesn't match OverrideFile schema
pub fn load_override_file(path: &Path) -> Result<OverrideFile> {
    let content =
        fs::read_to_string(path).context(format!("Failed to read override file: {:?}", path))?;

    let override_file: OverrideFile = serde_json::from_str(&content)
        .context(format!("Failed to parse override file JSON: {:?}", path))?;

    Ok(override_file)
}

/// Validate that discriminators are not all zeros
///
/// # Arguments
/// - `entity_type`: Type of entity ("account", "event", "instruction")
/// - `overrides`: Map of entity name to discriminator override
///
/// # Returns
/// - `Ok(())` if all discriminators are valid
/// - `Err(ValidationError::AllZeroDiscriminator)` if any discriminator is all zeros
fn validate_discriminators(
    entity_type: &str,
    overrides: &std::collections::HashMap<String, DiscriminatorOverride>,
) -> Result<(), ValidationError> {
    for (name, disc_override) in overrides {
        if disc_override.discriminator == [0u8; 8] {
            return Err(ValidationError::AllZeroDiscriminator {
                entity_type: entity_type.to_string(),
                entity_name: name.clone(),
            });
        }
    }
    Ok(())
}

/// Validate that entity names exist in the IDL
///
/// # Arguments
/// - `entity_type`: Type of entity ("account", "event", "instruction")
/// - `override_names`: Names from the override file to validate
/// - `idl_names`: Optional list of valid names from the IDL
///
/// # Returns
/// - `Ok(())` if all entity names are valid
/// - `Err(ValidationError::UnknownEntity)` if any name doesn't exist in IDL
fn validate_entity_names(
    entity_type: &str,
    override_names: &[String],
    idl_names: Option<&[&str]>,
) -> Result<(), ValidationError> {
    // If no overrides, nothing to validate
    if override_names.is_empty() {
        return Ok(());
    }

    match idl_names {
        Some(names) => {
            // Check each override name exists in IDL
            for override_name in override_names {
                if !names.contains(&override_name.as_str()) {
                    return Err(ValidationError::UnknownEntity {
                        entity_type: entity_type.to_string(),
                        entity_name: override_name.clone(),
                        available: if names.is_empty() {
                            "(none)".to_string()
                        } else {
                            names.join(", ")
                        },
                    });
                }
            }
            Ok(())
        }
        None => {
            // IDL has no entities of this type but override file has overrides
            // Return error for the first override name
            let first_name = &override_names[0];
            Err(ValidationError::UnknownEntity {
                entity_type: entity_type.to_string(),
                entity_name: first_name.clone(),
                available: format!("(none - IDL has no {}s defined)", entity_type),
            })
        }
    }
}

/// Validate override file structure and values
///
/// # Returns
/// - `Ok(())` if validation passes
/// - `Err(ValidationError)` if validation fails
///
/// # Validation Rules
/// - At least one field must be non-empty
/// - Program address must be valid base58 Pubkey (if present)
/// - Program address cannot be system default (11111...1111)
/// - Discriminators must be exactly 8 bytes (enforced by type)
/// - Discriminators cannot be all zeros
/// - Entity names MUST exist in IDL (errors for unknown names)
pub fn validate_override_file(
    override_file: &OverrideFile,
    idl: &crate::idl::Idl,
) -> Result<(), ValidationError> {
    // Check that at least one field is non-empty
    if override_file.program_address.is_none()
        && override_file.accounts.is_empty()
        && override_file.events.is_empty()
        && override_file.instructions.is_empty()
    {
        return Err(ValidationError::EmptyOverrideFile);
    }

    // Validate program address if present
    if let Some(ref address) = override_file.program_address {
        // Validate base58 format by attempting to decode
        // Solana Pubkeys are 32 bytes when decoded from base58
        match bs58::decode(address).into_vec() {
            Ok(decoded) => {
                if decoded.len() != 32 {
                    return Err(ValidationError::InvalidProgramAddress {
                        address: address.clone(),
                    });
                }

                // Check for system default pubkey (all 1s in base58 = 32 bytes of 0x00)
                if decoded == vec![0u8; 32] {
                    return Err(ValidationError::SystemDefaultPubkey {
                        address: address.clone(),
                    });
                }
            }
            Err(_) => {
                return Err(ValidationError::InvalidProgramAddress {
                    address: address.clone(),
                });
            }
        }
    }

    // Validate discriminators are not all zeros
    validate_discriminators("account", &override_file.accounts)?;
    validate_discriminators("event", &override_file.events)?;
    validate_discriminators("instruction", &override_file.instructions)?;

    // T056 [US3]: Validate account names exist in IDL
    let account_names: Option<Vec<&str>> = idl
        .accounts
        .as_ref()
        .map(|accounts| accounts.iter().map(|a| a.name.as_str()).collect());
    let override_account_names: Vec<String> = override_file.accounts.keys().cloned().collect();
    validate_entity_names("account", &override_account_names, account_names.as_deref())?;

    // T069 [US4]: Validate event names exist in IDL
    let event_names: Option<Vec<&str>> = idl
        .events
        .as_ref()
        .map(|events| events.iter().map(|e| e.name.as_str()).collect());
    let override_event_names: Vec<String> = override_file.events.keys().cloned().collect();
    validate_entity_names("event", &override_event_names, event_names.as_deref())?;

    // T081 [US5]: Validate instruction names exist in IDL
    let instruction_names: Option<Vec<&str>> = if !idl.instructions.is_empty() {
        Some(idl.instructions.iter().map(|i| i.name.as_str()).collect())
    } else {
        None
    };
    let override_instruction_names: Vec<String> =
        override_file.instructions.keys().cloned().collect();
    validate_entity_names(
        "instruction",
        &override_instruction_names,
        instruction_names.as_deref(),
    )?;

    Ok(())
}

/// Apply validated overrides to IDL structure
///
/// # Returns
/// - `Ok((modified_idl, applied_overrides))` with IDL and list of applied overrides
/// - `Err` if override application fails (should be rare after validation)
///
/// # Behavior
/// - Applies program address override (if present)
/// - Applies account discriminator overrides (User Story 3)
/// - Applies event discriminator overrides (User Story 4)
/// - Applies instruction discriminator overrides (User Story 5)
/// - Tracks all applied overrides for logging
pub fn apply_overrides(
    mut idl: crate::idl::Idl,
    override_file: &OverrideFile,
) -> Result<(crate::idl::Idl, Vec<AppliedOverride>)> {
    let mut applied = Vec::new();

    // Apply program address override
    if let Some(ref new_address) = override_file.program_address {
        let original_value = idl.address.clone();

        // Update address field
        idl.address = Some(new_address.clone());

        applied.push(AppliedOverride {
            override_type: OverrideType::ProgramAddress,
            entity_name: None,
            original_value,
            override_value: new_address.clone(),
        });
    }

    // T057 [US3]: Apply account discriminator overrides
    if let Some(ref mut accounts) = idl.accounts {
        for account in accounts.iter_mut() {
            if let Some(disc_override) = override_file.accounts.get(&account.name) {
                // Capture original value for logging
                let original = account
                    .discriminator
                    .as_ref()
                    .map(|d| format!("{:?}", d))
                    .unwrap_or("(none)".to_string());

                // Apply the override
                account.discriminator = Some(disc_override.discriminator.to_vec());

                applied.push(AppliedOverride {
                    override_type: OverrideType::AccountDiscriminator,
                    entity_name: Some(account.name.clone()),
                    original_value: Some(original),
                    override_value: format!("{:?}", disc_override.discriminator),
                });
            }
        }
    }

    // T070, T072 [US4]: Apply event discriminator overrides
    if let Some(ref mut events) = idl.events {
        for event in events.iter_mut() {
            if let Some(disc_override) = override_file.events.get(&event.name) {
                // Capture original value for logging
                let original = event
                    .discriminator
                    .as_ref()
                    .map(|d| format!("{:?}", d))
                    .unwrap_or("(none)".to_string());

                // Apply the override
                event.discriminator = Some(disc_override.discriminator.to_vec());

                applied.push(AppliedOverride {
                    override_type: OverrideType::EventDiscriminator,
                    entity_name: Some(event.name.clone()),
                    original_value: Some(original),
                    override_value: format!("{:?}", disc_override.discriminator),
                });
            }
        }
    }

    // T082, T084 [US5]: Apply instruction discriminator overrides
    for instruction in idl.instructions.iter_mut() {
        if let Some(disc_override) = override_file.instructions.get(&instruction.name) {
            // Capture original value for logging
            let original = instruction
                .discriminator
                .as_ref()
                .map(|d| format!("{:?}", d))
                .unwrap_or("(none)".to_string());

            // Apply the override
            instruction.discriminator = Some(disc_override.discriminator.to_vec());

            applied.push(AppliedOverride {
                override_type: OverrideType::InstructionDiscriminator,
                entity_name: Some(instruction.name.clone()),
                original_value: Some(original),
                override_value: format!("{:?}", disc_override.discriminator),
            });
        }
    }

    Ok((idl, applied))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ====================
    // User Story 1 Tests: Override Missing Program Addresses
    // ====================

    /// T012 [P] [US1] Unit test for discover_override_file with missing file
    #[test]
    fn test_discover_override_file_missing() {
        // Create isolated temp directory for this test
        let test_dir = std::env::temp_dir().join("override_test_missing");
        let _ = fs::remove_dir_all(&test_dir); // Clean up from previous runs
        fs::create_dir_all(&test_dir).unwrap();

        let idl_path = test_dir.join("nonexistent.json");
        let idl_name = "nonexistent_test_file_xyz"; // Use unique name unlikely to exist

        // Change to test directory so convention-based discovery doesn't find project files
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&test_dir).unwrap();

        let result = discover_override_file(&idl_path, idl_name, None).unwrap();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Clean up
        let _ = fs::remove_dir_all(&test_dir);

        assert!(matches!(result, OverrideDiscovery::NotFound));
    }

    /// T013 [P] [US1] Unit test for discover_override_file with explicit override
    #[test]
    fn test_discover_override_file_found() {
        use tempfile::TempDir;

        // Create unique temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create explicit override file
        let override_file = temp_path.join("explicit_override.json");
        fs::write(
            &override_file,
            r#"{"program_address": "11111111111111111111111111111112"}"#,
        )
        .unwrap();

        let idl_path = temp_path.join("test_idl.json");
        let idl_name = "test_idl";

        // Test with explicit override path (highest priority)
        let result = discover_override_file(&idl_path, idl_name, Some(&override_file)).unwrap();

        // Should find the explicit override file
        assert!(matches!(result, OverrideDiscovery::Found(_)));
        match result {
            OverrideDiscovery::Found(path) => {
                assert_eq!(
                    path, override_file,
                    "Should return the explicit override path"
                );
            }
            _ => panic!("Expected Found, got {:?}", result),
        }
    }

    /// T014 [P] [US1] Unit test for load_override_file with valid JSON
    #[test]
    fn test_load_override_file_valid_json() {
        let temp_dir = std::env::temp_dir();
        let override_file = temp_dir.join("test_valid_override.json");

        let json_content = r#"{
            "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
        }"#;

        fs::write(&override_file, json_content).unwrap();

        let result = load_override_file(&override_file);

        // Clean up
        fs::remove_file(&override_file).ok();

        assert!(result.is_ok());
        let override_data = result.unwrap();
        assert_eq!(
            override_data.program_address,
            Some("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string())
        );
    }

    /// T015 [P] [US1] Unit test for load_override_file with invalid JSON error
    #[test]
    fn test_load_override_file_invalid_json() {
        let temp_dir = std::env::temp_dir();
        let override_file = temp_dir.join("test_invalid_override.json");

        let invalid_json = r#"{ invalid json }"#;

        fs::write(&override_file, invalid_json).unwrap();

        let result = load_override_file(&override_file);

        // Clean up
        fs::remove_file(&override_file).ok();

        assert!(result.is_err());
    }

    /// T016 [P] [US1] Unit test for validate_override_file with valid program address
    #[test]
    fn test_validate_program_address_valid() {
        let override_file = OverrideFile {
            program_address: Some("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string()),
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        // Create minimal IDL for validation
        let idl = crate::idl::Idl {
            address: None,
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_ok());
    }

    /// T017 [P] [US1] Unit test for validate_override_file with invalid base58 address
    #[test]
    fn test_validate_program_address_invalid_base58() {
        let override_file = OverrideFile {
            program_address: Some("not-a-valid-pubkey".to_string()),
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        let idl = crate::idl::Idl {
            address: None,
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ValidationError::InvalidProgramAddress { .. }));
    }

    /// T018 [P] [US1] Unit test for validate_override_file with system default pubkey error
    #[test]
    fn test_validate_program_address_system_default() {
        let override_file = OverrideFile {
            program_address: Some("11111111111111111111111111111111".to_string()),
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        let idl = crate::idl::Idl {
            address: None,
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ValidationError::SystemDefaultPubkey { .. }));
    }

    // ====================
    // User Story 2 Tests: Override Incorrect Program Addresses
    // ====================

    /// T031 [P] [US2] Unit test for override with conflicting address
    #[test]
    fn test_override_conflicting_address() {
        let override_file = OverrideFile {
            program_address: Some("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string()),
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        // IDL with different program address
        let idl = crate::idl::Idl {
            address: Some("11111111111111111111111111111112".to_string()),
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        // Validation should pass (we allow overriding existing addresses)
        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_ok());

        // Apply overrides and verify original value is captured
        let (modified_idl, applied) = apply_overrides(idl, &override_file).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(
            applied[0].original_value,
            Some("11111111111111111111111111111112".to_string())
        );
        assert_eq!(
            applied[0].override_value,
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
        );
        assert_eq!(
            modified_idl.address,
            Some("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string())
        );
    }

    /// T032 [P] [US2] Unit test for override with same address (no-op case)
    #[test]
    fn test_override_same_address() {
        let same_address = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string();

        let override_file = OverrideFile {
            program_address: Some(same_address.clone()),
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        // IDL with same program address
        let idl = crate::idl::Idl {
            address: Some(same_address.clone()),
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        // Validation should pass
        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_ok());

        // Apply overrides - should still apply even if same
        let (modified_idl, applied) = apply_overrides(idl, &override_file).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].original_value, Some(same_address.clone()));
        assert_eq!(applied[0].override_value, same_address);
        assert_eq!(modified_idl.address, Some(same_address));
    }

    /// T033 [P] [US2] Unit test for warning message generation when overriding existing address
    #[test]
    fn test_warning_for_existing_address_override() {
        let override_file = OverrideFile {
            program_address: Some("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string()),
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        let original_address = "11111111111111111111111111111112".to_string();
        let idl = crate::idl::Idl {
            address: Some(original_address.clone()),
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        let (_modified_idl, applied) = apply_overrides(idl, &override_file).unwrap();

        // Verify that original_value contains the old address (this is what triggers the warning)
        assert!(applied[0].original_value.is_some());
        assert_eq!(
            applied[0].original_value.as_deref().unwrap(),
            original_address.as_str()
        );

        // In practice, main.rs checks if original_value.is_some() and != "(none)" to show warning
        // The warning format is: "⚠ Program address: {original} → {new}"
    }

    // ====================
    // User Story 3 Tests: Override Incorrect Account Discriminators
    // ====================

    /// T044 [P] [US3] Unit test for DiscriminatorOverride parsing from JSON
    #[test]
    fn test_discriminator_override_parsing() {
        let json = r#"{
            "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
        }"#;

        let disc_override: DiscriminatorOverride = serde_json::from_str(json).unwrap();
        assert_eq!(disc_override.discriminator, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    /// T045 [P] [US3] Unit test for discriminator validation (exactly 8 bytes)
    #[test]
    fn test_discriminator_exactly_8_bytes() {
        // The discriminator field is typed as [u8; 8], so it's always exactly 8 bytes
        // This test verifies the type system enforces this
        let disc = DiscriminatorOverride {
            discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        assert_eq!(disc.discriminator.len(), 8);
    }

    /// T046 [P] [US3] Unit test for discriminator validation (not all zeros)
    #[test]
    fn test_discriminator_not_all_zeros() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: {
                let mut map = HashMap::new();
                map.insert(
                    "TestAccount".to_string(),
                    DiscriminatorOverride {
                        discriminator: [0, 0, 0, 0, 0, 0, 0, 0],
                    },
                );
                map
            },
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        let idl = crate::idl::Idl {
            address: None,
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ValidationError::AllZeroDiscriminator { .. }));
    }

    /// T047 [P] [US3] Unit test for account discriminator override application
    #[test]
    fn test_account_discriminator_override_application() {
        // This test will be fully implemented once apply_overrides supports account discriminators
        let override_file = OverrideFile {
            program_address: None,
            accounts: {
                let mut map = HashMap::new();
                map.insert(
                    "PoolState".to_string(),
                    DiscriminatorOverride {
                        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                    },
                );
                map
            },
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        // For now, just verify the structure is correct
        assert_eq!(override_file.accounts.len(), 1);
        assert!(override_file.accounts.contains_key("PoolState"));
    }

    /// T048 [P] [US3] Unit test for unknown account name error
    #[test]
    fn test_unknown_account_name_warning() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: {
                let mut map = HashMap::new();
                map.insert(
                    "NonExistentAccount".to_string(),
                    DiscriminatorOverride {
                        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                    },
                );
                map
            },
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        // IDL with no accounts defined
        let idl = crate::idl::Idl {
            address: None,
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        // Validation should fail with UnknownEntity error
        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ValidationError::UnknownEntity { .. }));

        if let ValidationError::UnknownEntity {
            entity_type,
            entity_name,
            available,
        } = err
        {
            assert_eq!(entity_type, "account");
            assert_eq!(entity_name, "NonExistentAccount");
            assert!(available.contains("none"));
        }
    }

    // ====================
    // User Story 4 Tests: Override Incorrect Event Discriminators
    // ====================

    /// T062 [P] [US4] Unit test for event discriminator override application
    #[test]
    fn test_event_discriminator_override_application() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: HashMap::new(),
            events: vec![
                (
                    "TradeEvent".to_string(),
                    DiscriminatorOverride {
                        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                    },
                ),
                (
                    "SwapEvent".to_string(),
                    DiscriminatorOverride {
                        discriminator: [11, 12, 13, 14, 15, 16, 17, 18],
                    },
                ),
            ]
            .into_iter()
            .collect(),
            instructions: HashMap::new(),
        };

        let idl = crate::idl::Idl {
            version: Some("0.1.0".to_string()),
            name: Some("test_program".to_string()),
            address: None,
            instructions: vec![],
            accounts: None,
            types: None,
            events: Some(vec![
                crate::idl::Event {
                    name: "TradeEvent".to_string(),
                    discriminator: Some(vec![255, 255, 255, 255, 255, 255, 255, 255]),
                    fields: None,
                },
                crate::idl::Event {
                    name: "SwapEvent".to_string(),
                    discriminator: Some(vec![254, 254, 254, 254, 254, 254, 254, 254]),
                    fields: None,
                },
            ]),
            errors: None,
            constants: None,
            metadata: None,
        };

        let (modified_idl, applied) = apply_overrides(idl, &override_file)
            .expect("Failed to apply event discriminator overrides");

        assert_eq!(applied.len(), 2, "Should apply 2 event overrides");

        // Verify TradeEvent discriminator was updated
        let events = modified_idl.events.as_ref().unwrap();
        let trade_event = events.iter().find(|e| e.name == "TradeEvent").unwrap();
        assert_eq!(
            trade_event.discriminator.as_ref().unwrap(),
            &vec![1, 2, 3, 4, 5, 6, 7, 8],
            "TradeEvent discriminator should be overridden"
        );

        // Verify SwapEvent discriminator was updated
        let swap_event = events.iter().find(|e| e.name == "SwapEvent").unwrap();
        assert_eq!(
            swap_event.discriminator.as_ref().unwrap(),
            &vec![11, 12, 13, 14, 15, 16, 17, 18],
            "SwapEvent discriminator should be overridden"
        );
    }

    /// T063 [P] [US4] Unit test for unknown event name error
    #[test]
    fn test_unknown_event_name_warning() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: HashMap::new(),
            events: vec![
                (
                    "UnknownEvent".to_string(),
                    DiscriminatorOverride {
                        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                    },
                ),
                (
                    "TradeEvent".to_string(),
                    DiscriminatorOverride {
                        discriminator: [11, 12, 13, 14, 15, 16, 17, 18],
                    },
                ),
            ]
            .into_iter()
            .collect(),
            instructions: HashMap::new(),
        };

        let idl = crate::idl::Idl {
            version: Some("0.1.0".to_string()),
            name: Some("test_program".to_string()),
            address: None,
            instructions: vec![],
            accounts: None,
            types: None,
            events: Some(vec![crate::idl::Event {
                name: "TradeEvent".to_string(),
                discriminator: Some(vec![255, 255, 255, 255, 255, 255, 255, 255]),
                fields: None,
            }]),
            errors: None,
            constants: None,
            metadata: None,
        };

        // Validation should fail with UnknownEntity error for UnknownEvent
        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ValidationError::UnknownEntity { .. }));

        if let ValidationError::UnknownEntity {
            entity_type,
            entity_name,
            available,
        } = err
        {
            assert_eq!(entity_type, "event");
            assert_eq!(entity_name, "UnknownEvent");
            assert!(available.contains("TradeEvent"));
        }
    }

    /// T064 [P] [US4] Unit test for multiple event overrides in same file
    #[test]
    fn test_multiple_event_overrides() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: HashMap::new(),
            events: vec![
                (
                    "Event1".to_string(),
                    DiscriminatorOverride {
                        discriminator: [1, 1, 1, 1, 1, 1, 1, 1],
                    },
                ),
                (
                    "Event2".to_string(),
                    DiscriminatorOverride {
                        discriminator: [2, 2, 2, 2, 2, 2, 2, 2],
                    },
                ),
                (
                    "Event3".to_string(),
                    DiscriminatorOverride {
                        discriminator: [3, 3, 3, 3, 3, 3, 3, 3],
                    },
                ),
            ]
            .into_iter()
            .collect(),
            instructions: HashMap::new(),
        };

        let idl = crate::idl::Idl {
            version: Some("0.1.0".to_string()),
            name: Some("test_program".to_string()),
            address: None,
            instructions: vec![],
            accounts: None,
            types: None,
            events: Some(vec![
                crate::idl::Event {
                    name: "Event1".to_string(),
                    discriminator: Some(vec![255, 255, 255, 255, 255, 255, 255, 255]),
                    fields: None,
                },
                crate::idl::Event {
                    name: "Event2".to_string(),
                    discriminator: Some(vec![254, 254, 254, 254, 254, 254, 254, 254]),
                    fields: None,
                },
                crate::idl::Event {
                    name: "Event3".to_string(),
                    discriminator: Some(vec![253, 253, 253, 253, 253, 253, 253, 253]),
                    fields: None,
                },
            ]),
            errors: None,
            constants: None,
            metadata: None,
        };

        let (modified_idl, applied) =
            apply_overrides(idl, &override_file).expect("Failed to apply multiple event overrides");

        assert_eq!(applied.len(), 3, "Should apply all 3 event overrides");

        let events = modified_idl.events.as_ref().unwrap();
        assert_eq!(events.len(), 3, "Should have 3 events");

        // Verify all discriminators were updated
        let event1 = events.iter().find(|e| e.name == "Event1").unwrap();
        assert_eq!(
            event1.discriminator.as_ref().unwrap(),
            &vec![1, 1, 1, 1, 1, 1, 1, 1]
        );

        let event2 = events.iter().find(|e| e.name == "Event2").unwrap();
        assert_eq!(
            event2.discriminator.as_ref().unwrap(),
            &vec![2, 2, 2, 2, 2, 2, 2, 2]
        );

        let event3 = events.iter().find(|e| e.name == "Event3").unwrap();
        assert_eq!(
            event3.discriminator.as_ref().unwrap(),
            &vec![3, 3, 3, 3, 3, 3, 3, 3]
        );
    }

    // ====================
    // User Story 5 Tests: Override Incorrect Instruction Discriminators
    // ====================

    /// T075 [P] [US5] Unit test for instruction discriminator override application
    #[test]
    fn test_instruction_discriminator_override_application() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: vec![
                (
                    "Initialize".to_string(),
                    DiscriminatorOverride {
                        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                    },
                ),
                (
                    "Trade".to_string(),
                    DiscriminatorOverride {
                        discriminator: [11, 12, 13, 14, 15, 16, 17, 18],
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let idl = crate::idl::Idl {
            version: Some("0.1.0".to_string()),
            name: Some("test_program".to_string()),
            address: None,
            instructions: vec![
                crate::idl::Instruction {
                    name: "Initialize".to_string(),
                    discriminator: Some(vec![255, 255, 255, 255, 255, 255, 255, 255]),
                    accounts: vec![],
                    args: vec![],
                    docs: None,
                },
                crate::idl::Instruction {
                    name: "Trade".to_string(),
                    discriminator: Some(vec![254, 254, 254, 254, 254, 254, 254, 254]),
                    accounts: vec![],
                    args: vec![],
                    docs: None,
                },
            ],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        let (modified_idl, applied) = apply_overrides(idl, &override_file)
            .expect("Failed to apply instruction discriminator overrides");

        assert_eq!(applied.len(), 2, "Should apply 2 instruction overrides");

        // Verify Initialize instruction discriminator was updated
        let initialize_ix = modified_idl
            .instructions
            .iter()
            .find(|i| i.name == "Initialize")
            .unwrap();
        assert_eq!(
            initialize_ix.discriminator.as_ref().unwrap(),
            &vec![1, 2, 3, 4, 5, 6, 7, 8],
            "Initialize discriminator should be overridden"
        );

        // Verify Trade instruction discriminator was updated
        let trade_ix = modified_idl
            .instructions
            .iter()
            .find(|i| i.name == "Trade")
            .unwrap();
        assert_eq!(
            trade_ix.discriminator.as_ref().unwrap(),
            &vec![11, 12, 13, 14, 15, 16, 17, 18],
            "Trade discriminator should be overridden"
        );
    }

    /// T076 [P] [US5] Unit test for unknown instruction name error
    #[test]
    fn test_unknown_instruction_name_warning() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: vec![
                (
                    "UnknownInstruction".to_string(),
                    DiscriminatorOverride {
                        discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                    },
                ),
                (
                    "Initialize".to_string(),
                    DiscriminatorOverride {
                        discriminator: [11, 12, 13, 14, 15, 16, 17, 18],
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let idl = crate::idl::Idl {
            version: Some("0.1.0".to_string()),
            name: Some("test_program".to_string()),
            address: None,
            instructions: vec![crate::idl::Instruction {
                name: "Initialize".to_string(),
                discriminator: Some(vec![255, 255, 255, 255, 255, 255, 255, 255]),
                accounts: vec![],
                args: vec![],
                docs: None,
            }],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        // Validation should fail with UnknownEntity error for UnknownInstruction
        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, ValidationError::UnknownEntity { .. }));

        if let ValidationError::UnknownEntity {
            entity_type,
            entity_name,
            available,
        } = err
        {
            assert_eq!(entity_type, "instruction");
            assert_eq!(entity_name, "UnknownInstruction");
            assert!(available.contains("Initialize"));
        }
    }

    // ====================
    // Phase 8 Tests: Edge Cases & Error Handling
    // ====================

    /// T087 [P] Unit test for multiple override files detected (Conflict error)
    #[test]
    fn test_multiple_override_files_conflict() {
        use std::fs;
        use tempfile::TempDir;

        // Create temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create overrides directory
        let overrides_dir = temp_path.join("overrides");
        fs::create_dir_all(&overrides_dir).unwrap();

        // Create convention-based override file
        let convention_override = overrides_dir.join("test_program.json");
        fs::write(
            &convention_override,
            r#"{"program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"}"#,
        )
        .unwrap();

        // Create global fallback override file
        let global_override = temp_path.join("idl-overrides.json");
        fs::write(
            &global_override,
            r#"{"program_address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"}"#,
        )
        .unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Test conflict detection
        let result =
            discover_override_file(Path::new("test_program.json"), "test_program", None).unwrap();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Verify conflict was detected
        match result {
            OverrideDiscovery::Conflict { files, sources } => {
                assert_eq!(files.len(), 2, "Should detect 2 conflicting override files");
                assert_eq!(sources.len(), 2, "Should have 2 sources");
                assert!(
                    sources.contains(&"convention-based discovery".to_string()),
                    "Should include convention-based source"
                );
                assert!(
                    sources.contains(&"global fallback".to_string()),
                    "Should include global fallback source"
                );
            }
            _ => panic!("Expected Conflict, got {:?}", result),
        }
    }

    /// T088 [P] Unit test for empty override file (EmptyOverrideFile error)
    #[test]
    fn test_empty_override_file_error() {
        let override_file = OverrideFile {
            program_address: None,
            accounts: HashMap::new(),
            events: HashMap::new(),
            instructions: HashMap::new(),
        };

        let idl = crate::idl::Idl {
            address: None,
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            instructions: vec![],
            accounts: None,
            types: None,
            events: None,
            errors: None,
            constants: None,
            metadata: None,
        };

        let result = validate_override_file(&override_file, &idl);
        assert!(result.is_err(), "Empty override file should return error");

        let err = result.unwrap_err();
        assert!(
            matches!(err, ValidationError::EmptyOverrideFile),
            "Should be EmptyOverrideFile error"
        );
    }

    /// T089 [P] Unit test for malformed JSON error handling
    #[test]
    fn test_malformed_json_error() {
        use tempfile::NamedTempFile;

        // Create temporary file with malformed JSON
        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_file
            .write_all(b"{invalid json missing quotes and commas}")
            .unwrap();

        let result = load_override_file(temp_file.path());
        assert!(
            result.is_err(),
            "Malformed JSON should return error: {:?}",
            result
        );

        // Verify error message contains helpful context
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Failed to parse override file JSON"),
            "Error should mention JSON parsing failure"
        );
    }

    /// T090 [P] Unit test for file not found error handling
    #[test]
    fn test_file_not_found_error() {
        let non_existent_path = Path::new("/tmp/this_file_definitely_does_not_exist_12345.json");

        let result = load_override_file(non_existent_path);
        assert!(
            result.is_err(),
            "Non-existent file should return error: {:?}",
            result
        );

        // Verify error message contains helpful context
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Failed to read override file"),
            "Error should mention file reading failure"
        );
    }
}
