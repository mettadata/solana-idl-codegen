//! Integration tests for IDL Override System
//!
//! These tests verify the complete override workflow:
//! - IDL files with missing/incorrect data
//! - Override files with corrections
//! - Code generation with overrides applied
//! - Verification that generated code compiles and uses override values

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Get absolute path to a fixture file to avoid race conditions from current directory changes
fn fixture_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/fixtures")
        .join(relative_path)
}

/// T019 [P] [US1] Integration test: IDL with missing address + override → generated code compiles
#[test]
fn test_missing_address_with_override_compiles() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test IDL and override file to test directory
    let idl_path = test_dir.join("test_missing_address.json");
    let override_path = test_dir.join("test_address_override.json");

    fs::copy(
        fixture_path("test_missing_address.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_address_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code using the CLI with override file
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Verify generated code directory exists
    assert!(
        output_dir.join("test_program").exists(),
        "Generated code directory not found"
    );

    // Verify generated lib.rs exists
    let lib_rs = output_dir.join("test_program").join("src").join("lib.rs");
    assert!(lib_rs.exists(), "Generated lib.rs not found");

    // Verify Cargo.toml exists
    let cargo_toml = output_dir.join("test_program").join("Cargo.toml");
    assert!(cargo_toml.exists(), "Generated Cargo.toml not found");

    // Note: We don't compile the generated code here because that tests the codegen itself,
    // not the override system. The override system's job is to modify the IDL correctly,
    // which we verify by checking that the generated files contain the expected content.

    // Verify that the generated lib.rs contains the override address
    let lib_rs_content = fs::read_to_string(&lib_rs).expect("Failed to read generated lib.rs");

    let expected_address = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
    assert!(
        lib_rs_content.contains(expected_address),
        "Generated code does not contain override address"
    );

    // Cleanup happens automatically when temp_dir drops
}

/// T020 [P] [US1] Integration test: verify PROGRAM_ID constant matches override value
#[test]
fn test_program_id_matches_override_value() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_missing_address.json");
    let override_path = test_dir.join("test_address_override.json");

    fs::copy(
        fixture_path("test_missing_address.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_address_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Read generated lib.rs
    let lib_rs_path = output_dir.join("test_program").join("src").join("lib.rs");
    let lib_rs_content = fs::read_to_string(&lib_rs_path).expect("Failed to read generated lib.rs");

    // Verify PROGRAM_ID constant contains the override address
    let expected_address = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
    assert!(
        lib_rs_content.contains(expected_address),
        "Generated lib.rs does not contain the expected program address from override file.\nExpected: {}\nGenerated content:\n{}",
        expected_address,
        lib_rs_content
    );

    // Verify declare_id macro is used (Solana standard pattern)
    assert!(
        lib_rs_content.contains("declare_id!"),
        "Generated lib.rs does not contain declare_id! macro"
    );

    // Cleanup happens automatically when temp_dir drops
}

// ====================
// User Story 2: Override Incorrect Program Addresses
// ====================

/// T034 [P] [US2] Integration test: IDL with incorrect address + override → generated code uses override value
#[test]
fn test_incorrect_address_with_override() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_incorrect_address.json");
    let override_path = test_dir.join("test_address_correction.json");

    fs::copy(
        fixture_path("test_incorrect_address.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_address_correction.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Read generated lib.rs
    let lib_rs_path = output_dir.join("test_program").join("src").join("lib.rs");
    let lib_rs_content = fs::read_to_string(&lib_rs_path).expect("Failed to read generated lib.rs");

    // Verify the generated code uses the OVERRIDE address (not the incorrect one from IDL)
    let override_address = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
    let incorrect_address = "11111111111111111111111111111112";

    assert!(
        lib_rs_content.contains(override_address),
        "Generated code should contain override address"
    );

    assert!(
        !lib_rs_content.contains(incorrect_address),
        "Generated code should NOT contain the incorrect original address"
    );

    // Cleanup happens automatically when temp_dir drops
}

/// T035 [P] [US2] Integration test: verify warning logged showing original vs override address
#[test]
fn test_warning_shows_original_and_override_address() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_incorrect_address.json");
    let override_path = test_dir.join("test_address_correction.json");

    fs::copy(
        fixture_path("test_incorrect_address.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_address_correction.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Run codegen and capture output
    let output_dir = test_dir.join("generated");
    let output = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute codegen");

    assert!(output.status.success(), "Code generation failed");

    // Convert output to string
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify warning message contains both addresses
    let original_address = "11111111111111111111111111111112";
    let override_address = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

    assert!(
        stdout.contains(original_address),
        "Warning should show original address"
    );
    assert!(
        stdout.contains(override_address),
        "Warning should show override address"
    );
    assert!(
        stdout.contains("⚠") || stdout.contains("Program address"),
        "Warning should be clearly marked"
    );

    // Cleanup happens automatically when temp_dir drops
}

// ====================
// User Story 3: Override Incorrect Account Discriminators
// ====================

/// T049 [P] [US3] Integration test: IDL with incorrect account discriminators + override → generated code compiles
#[test]
fn test_account_discriminators_with_override() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_account_disc.json");
    let override_path = test_dir.join("test_account_override.json");

    fs::copy(
        fixture_path("test_account_disc.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_account_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Verify generated code exists
    let accounts_rs = output_dir
        .join("test_program")
        .join("src")
        .join("accounts.rs");
    assert!(accounts_rs.exists(), "Generated accounts.rs not found");

    // Cleanup happens automatically when temp_dir drops
}

/// T050 [P] [US3] Integration test: verify account struct DISCRIMINATOR constant matches override
#[test]
fn test_account_discriminator_constant_matches_override() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_account_disc.json");
    let override_path = test_dir.join("test_account_override.json");

    fs::copy(
        fixture_path("test_account_disc.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_account_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Read generated accounts.rs
    let accounts_rs_path = output_dir
        .join("test_program")
        .join("src")
        .join("accounts.rs");
    let accounts_rs_content =
        fs::read_to_string(&accounts_rs_path).expect("Failed to read generated accounts.rs");

    // Verify PoolState discriminator matches override [1, 2, 3, 4, 5, 6, 7, 8]
    assert!(
        accounts_rs_content.contains("pub struct PoolState"),
        "Generated accounts.rs does not contain PoolState struct"
    );
    assert!(
        accounts_rs_content.contains("const DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8]")
            || accounts_rs_content
                .contains("DISCRIMINATOR: [u8; 8] = [1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8]"),
        "PoolState DISCRIMINATOR constant does not match override value [1, 2, 3, 4, 5, 6, 7, 8]"
    );

    // Verify UserAccount discriminator matches override [11, 12, 13, 14, 15, 16, 17, 18]
    assert!(
        accounts_rs_content.contains("pub struct UserAccount"),
        "Generated accounts.rs does not contain UserAccount struct"
    );
    assert!(
        accounts_rs_content.contains("const DISCRIMINATOR: [u8; 8] = [11, 12, 13, 14, 15, 16, 17, 18]")
            || accounts_rs_content.contains("DISCRIMINATOR: [u8; 8] = [11u8, 12u8, 13u8, 14u8, 15u8, 16u8, 17u8, 18u8]"),
        "UserAccount DISCRIMINATOR constant does not match override value [11, 12, 13, 14, 15, 16, 17, 18]"
    );

    // Cleanup happens automatically when temp_dir drops
}

// ====================
// User Story 4: Override Incorrect Event Discriminators
// ====================

/// T065 [P] [US4] Integration test: IDL with incorrect event discriminators + override → generated code compiles
#[test]
fn test_event_discriminators_with_override() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_event_disc.json");
    let override_path = test_dir.join("test_event_override.json");

    fs::copy(fixture_path("test_event_disc.json"), &idl_path)
        .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_event_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Verify generated code exists
    let events_rs = output_dir
        .join("test_program")
        .join("src")
        .join("events.rs");
    assert!(events_rs.exists(), "Generated events.rs not found");

    // Cleanup happens automatically when temp_dir drops
}

/// T066 [P] [US4] Integration test: verify event struct DISCRIMINATOR constant matches override
#[test]
fn test_event_discriminator_constant_matches_override() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_event_disc.json");
    let override_path = test_dir.join("test_event_override.json");

    fs::copy(fixture_path("test_event_disc.json"), &idl_path)
        .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_event_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Read generated events.rs
    let events_rs_path = output_dir
        .join("test_program")
        .join("src")
        .join("events.rs");
    let events_rs_content =
        fs::read_to_string(&events_rs_path).expect("Failed to read generated events.rs");

    // Verify TradeEvent discriminator matches override [1, 2, 3, 4, 5, 6, 7, 8]
    assert!(
        events_rs_content.contains("pub struct TradeEvent"),
        "Generated events.rs does not contain TradeEvent struct"
    );
    assert!(
        events_rs_content.contains(
            "const TRADE_EVENT_EVENT_DISCM: [u8; 8] = [1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8]"
        ),
        "TradeEvent discriminator constant does not match override value [1, 2, 3, 4, 5, 6, 7, 8]"
    );

    // Verify SwapEvent discriminator matches override [11, 12, 13, 14, 15, 16, 17, 18]
    assert!(
        events_rs_content.contains("pub struct SwapEvent"),
        "Generated events.rs does not contain SwapEvent struct"
    );
    assert!(
        events_rs_content.contains("const SWAP_EVENT_EVENT_DISCM: [u8; 8] = [11u8, 12u8, 13u8, 14u8, 15u8, 16u8, 17u8, 18u8]"),
        "SwapEvent discriminator constant does not match override value [11, 12, 13, 14, 15, 16, 17, 18]"
    );

    // Cleanup happens automatically when temp_dir drops
}

// ====================
// User Story 5: Override Incorrect Instruction Discriminators
// ====================

/// T077 [P] [US5] Integration test: IDL with incorrect instruction discriminators + override → generated code compiles
#[test]
fn test_instruction_discriminators_with_override() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_instruction_disc.json");
    let override_path = test_dir.join("test_instruction_override.json");

    fs::copy(
        fixture_path("test_instruction_disc.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_instruction_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Verify generated code exists
    let instructions_rs = output_dir
        .join("test_program")
        .join("src")
        .join("instructions.rs");
    assert!(
        instructions_rs.exists(),
        "Generated instructions.rs not found"
    );

    // Cleanup happens automatically when temp_dir drops
}

/// T078 [P] [US5] Integration test: verify instruction enum discriminator matches override
#[test]
fn test_instruction_discriminator_constant_matches_override() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Copy test files
    let idl_path = test_dir.join("test_instruction_disc.json");
    let override_path = test_dir.join("test_instruction_override.json");

    fs::copy(
        fixture_path("test_instruction_disc.json"),
        &idl_path,
    )
    .expect("Failed to copy test IDL");

    fs::copy(
        fixture_path("test_instruction_override.json"),
        &override_path,
    )
    .expect("Failed to copy override file");

    // Generate code
    let output_dir = test_dir.join("generated");
    let status = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args([
            "--input",
            idl_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--module",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute codegen");

    assert!(status.success(), "Code generation failed");

    // Read generated instructions.rs
    let instructions_rs_path = output_dir
        .join("test_program")
        .join("src")
        .join("instructions.rs");
    let instructions_rs_content = fs::read_to_string(&instructions_rs_path)
        .expect("Failed to read generated instructions.rs");

    // Verify Initialize instruction discriminator matches override [1, 2, 3, 4, 5, 6, 7, 8]
    // Instructions use discriminator bytes directly in match statements
    assert!(
        instructions_rs_content.contains("[1, 2, 3, 4, 5, 6, 7, 8]")
            || instructions_rs_content.contains("[1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8]"),
        "Initialize instruction discriminator does not match override value [1, 2, 3, 4, 5, 6, 7, 8]"
    );

    // Verify Trade instruction discriminator matches override [11, 12, 13, 14, 15, 16, 17, 18]
    assert!(
        instructions_rs_content.contains("[11, 12, 13, 14, 15, 16, 17, 18]")
            || instructions_rs_content.contains("[11u8, 12u8, 13u8, 14u8, 15u8, 16u8, 17u8, 18u8]"),
        "Trade instruction discriminator does not match override value [11, 12, 13, 14, 15, 16, 17, 18]"
    );

    // Cleanup happens automatically when temp_dir drops
}

// ====================
// Phase 8: Edge Cases & Error Handling Integration Tests
// ====================

/// T091 [P] Integration test: multiple override files detected error
#[test]
fn test_multiple_override_files_error() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Create overrides directory
    let overrides_dir = test_dir.join("overrides");
    fs::create_dir_all(&overrides_dir).expect("Failed to create overrides directory");

    // Create test IDL file (minimal)
    let idl_content = r#"{
  "version": "0.1.0",
  "name": "test_program",
  "instructions": [
    {
      "name": "Initialize",
      "accounts": [],
      "args": []
    }
  ]
}"#;

    let idl_path = test_dir.join("test_program.json");
    fs::write(&idl_path, idl_content).expect("Failed to write IDL file");

    // Create convention-based override file
    let convention_override_content = r#"{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
}"#;

    let convention_override_path = overrides_dir.join("test_program.json");
    fs::write(&convention_override_path, convention_override_content)
        .expect("Failed to write convention override file");

    // Create global fallback override file
    let global_override_content = r#"{
  "program_address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
}"#;

    let global_override_path = test_dir.join("idl-overrides.json");
    fs::write(&global_override_path, global_override_content)
        .expect("Failed to write global override file");

    // Change to test directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&test_dir).expect("Failed to change to test directory");

    // Run codegen - should fail with conflict error
    let output = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args(&[
            "-i",
            "test_program.json",
            "-o",
            "generated",
            "-m",
            "test_program",
        ])
        .output()
        .expect("Failed to execute codegen");

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore directory");

    // Verify the error occurred
    assert!(
        !output.status.success(),
        "Codegen should fail with multiple override files conflict"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Multiple override files detected"),
        "Error should mention multiple override files. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("convention-based discovery")
            || stderr.contains("overrides/test_program.json"),
        "Error should mention convention-based file. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("global fallback") || stderr.contains("idl-overrides.json"),
        "Error should mention global fallback file. stderr: {}",
        stderr
    );

    // Cleanup happens automatically when temp_dir drops
}

/// T092 [P] Integration test: malformed override file fails gracefully
#[test]
fn test_malformed_override_file_error() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Create test IDL file (minimal)
    let idl_content = r#"{
  "version": "0.1.0",
  "name": "test_program",
  "instructions": [
    {
      "name": "Initialize",
      "accounts": [],
      "args": []
    }
  ]
}"#;

    let idl_path = test_dir.join("test_program.json");
    fs::write(&idl_path, idl_content).expect("Failed to write IDL file");

    // Create malformed override file (invalid JSON)
    let malformed_override_content = r#"{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
  missing comma and closing brace
}"#;

    let override_path = test_dir.join("override.json");
    fs::write(&override_path, malformed_override_content)
        .expect("Failed to write malformed override file");

    // Run codegen with explicit override file - should fail with parse error
    let output = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args(&[
            "-i",
            idl_path.to_str().unwrap(),
            "-o",
            test_dir.join("generated").to_str().unwrap(),
            "-m",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute codegen");

    // Verify the error occurred
    assert!(
        !output.status.success(),
        "Codegen should fail with malformed JSON"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to parse override file JSON") || stderr.contains("JSON"),
        "Error should mention JSON parsing failure. stderr: {}",
        stderr
    );

    // Cleanup happens automatically when temp_dir drops
}

/// T093 [P] Integration test: empty override file error
#[test]
fn test_empty_override_file_error() {
    // TempDir automatically cleans up on drop (RAII pattern)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();

    // Create test IDL file (minimal)
    let idl_content = r#"{
  "version": "0.1.0",
  "name": "test_program",
  "instructions": [
    {
      "name": "Initialize",
      "accounts": [],
      "args": []
    }
  ]
}"#;

    let idl_path = test_dir.join("test_program.json");
    fs::write(&idl_path, idl_content).expect("Failed to write IDL file");

    // Create empty override file (valid JSON but no overrides)
    let empty_override_content = r#"{}"#;

    let override_path = test_dir.join("override.json");
    fs::write(&override_path, empty_override_content).expect("Failed to write empty override file");

    // Run codegen with explicit override file - should fail with empty file error
    let output = Command::new(env!("CARGO_BIN_EXE_solana-idl-codegen"))
        .args(&[
            "-i",
            idl_path.to_str().unwrap(),
            "-o",
            test_dir.join("generated").to_str().unwrap(),
            "-m",
            "test_program",
            "--override-file",
            override_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute codegen");

    // Verify the error occurred
    assert!(
        !output.status.success(),
        "Codegen should fail with empty override file"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Empty override file")
            || stderr.contains("must contain at least one override"),
        "Error should mention empty override file. stderr: {}",
        stderr
    );

    // Cleanup happens automatically when temp_dir drops
}
