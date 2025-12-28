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

    /// T013 [P] [US1] Unit test for discover_override_file with found file
    #[test]
    fn test_discover_override_file_found() {
        let temp_dir = std::env::temp_dir();
        // Create the correct convention-based directory: ./overrides/
        let overrides_dir = temp_dir.join("overrides");
        fs::create_dir_all(&overrides_dir).unwrap();

        let override_file = overrides_dir.join("test_idl.json");
        fs::write(
            &override_file,
            r#"{"program_address": "11111111111111111111111111111112"}"#,
        )
        .unwrap();

        let idl_path = temp_dir.join("test_idl.json");
        let idl_name = "test_idl";

        // Change to the temp directory temporarily
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = discover_override_file(&idl_path, idl_name, None).unwrap();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Clean up
        fs::remove_file(&override_file).ok();

        assert!(matches!(result, OverrideDiscovery::Found(_)));
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
}

// Public API functions
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Discover override file location using convention-based search or explicit path
///
/// # Discovery Order
/// 1. If `explicit_override` provided: check if it exists and detect conflicts with convention
/// 2. Convention-based: check `./overrides/{idl_name}.json`
/// 3. Global fallback: check `./idl-overrides.json`
///
/// # Returns
/// - `OverrideDiscovery::Found(path)` if override file found
/// - `OverrideDiscovery::NotFound` if no override file found (not an error)
/// - `OverrideDiscovery::Conflict` if multiple override files detected
pub fn discover_override_file(
    _idl_path: &Path,
    idl_name: &str,
    explicit_override: Option<&Path>,
) -> Result<OverrideDiscovery> {
    let mut found_files = Vec::new();
    let mut sources = Vec::new();

    // Check explicit override file
    if let Some(explicit_path) = explicit_override {
        if explicit_path.exists() {
            found_files.push(explicit_path.to_path_buf());
            sources.push("explicit CLI --override-file".to_string());
        }
    }

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

/// Validate override file structure and values
///
/// # Returns
/// - `Ok(warnings)` if validation passes, with list of warning messages
/// - `Err(ValidationError)` if validation fails
///
/// # Validation Rules
/// - At least one field must be non-empty
/// - Program address must be valid base58 Pubkey (if present)
/// - Program address cannot be system default (11111...1111)
/// - Discriminators must be exactly 8 bytes (enforced by type)
/// - Discriminators cannot be all zeros
/// - Entity names should exist in IDL (warnings only)
pub fn validate_override_file(
    override_file: &OverrideFile,
    _idl: &crate::idl::Idl,
) -> Result<Vec<String>, ValidationError> {
    let warnings = Vec::new();

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

    // Validate discriminators (will be expanded in US3)
    // For now, just check they're not all zeros
    for (name, disc_override) in &override_file.accounts {
        if disc_override.discriminator == [0u8; 8] {
            return Err(ValidationError::AllZeroDiscriminator {
                entity_type: "account".to_string(),
                entity_name: name.clone(),
            });
        }
    }

    for (name, disc_override) in &override_file.events {
        if disc_override.discriminator == [0u8; 8] {
            return Err(ValidationError::AllZeroDiscriminator {
                entity_type: "event".to_string(),
                entity_name: name.clone(),
            });
        }
    }

    for (name, disc_override) in &override_file.instructions {
        if disc_override.discriminator == [0u8; 8] {
            return Err(ValidationError::AllZeroDiscriminator {
                entity_type: "instruction".to_string(),
                entity_name: name.clone(),
            });
        }
    }

    Ok(warnings)
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

    // Account discriminators will be added in User Story 3
    // Event discriminators will be added in User Story 4
    // Instruction discriminators will be added in User Story 5

    Ok((idl, applied))
}
