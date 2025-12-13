//! Post-Generation Integration Tests
//!
//! These tests verify that all generated code compiles and functions correctly.
//! Run `just generate` before running these tests.
//!
//! See INTEGRATION_TESTING.md for more information on writing tests.

use std::path::Path;
use std::time::Instant;

// ============================================================================
// File Structure Tests
// ============================================================================

#[test]
fn test_all_crates_generated() {
    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    let mut missing = Vec::new();
    let mut present = Vec::new();

    for crate_name in &crates {
        let path = format!("generated/{}", crate_name);
        if Path::new(&path).exists() {
            present.push(crate_name);
        } else {
            missing.push(crate_name);
        }
    }

    if !missing.is_empty() {
        eprintln!("⚠️  Missing generated crates: {:?}", missing);
        eprintln!("   Run `just generate` to create them.");
        eprintln!("   Present crates: {:?}", present);
    } else {
        println!("✓ All {} generated crates are present", crates.len());
    }

    assert!(
        !present.is_empty(),
        "At least some crates should be generated. Run `just generate`."
    );
}

#[test]
fn test_all_crates_have_required_files() {
    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];
    let required_files = [
        "Cargo.toml",
        "src/lib.rs",
        "src/accounts.rs",
        "src/instructions.rs",
        "src/events.rs",
        "src/errors.rs",
        "src/types.rs",
    ];

    let mut tested_crates = 0;

    for crate_name in &crates {
        let crate_path = format!("generated/{}", crate_name);
        if !Path::new(&crate_path).exists() {
            continue; // Skip if crate doesn't exist
        }

        tested_crates += 1;

        for file in &required_files {
            let file_path = format!("{}/{}", crate_path, file);
            assert!(
                Path::new(&file_path).exists(),
                "Missing required file: {}",
                file_path
            );
        }

        println!("✓ {} has all required files", crate_name);
    }

    assert!(
        tested_crates > 0,
        "No generated crates found to test. Run `just generate`."
    );
}

// ============================================================================
// Compilation Tests
// ============================================================================

#[test]
fn test_generated_crates_compile() {
    use std::process::Command;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    let mut tested = 0;
    let mut passed = 0;
    let mut failed = Vec::new();
    let mut timings = Vec::new();

    for crate_name in &crates {
        let crate_path = format!("generated/{}", crate_name);
        if !Path::new(&crate_path).exists() {
            continue;
        }

        tested += 1;

        println!("Checking {}...", crate_name);
        let start = Instant::now();
        let output = Command::new("cargo")
            .args([
                "check",
                "--manifest-path",
                &format!("{}/Cargo.toml", crate_path),
            ])
            .output()
            .expect("Failed to run cargo check");
        let duration = start.elapsed();

        if output.status.success() {
            passed += 1;
            println!(
                "✓ {} compiles successfully in {:.2}s",
                crate_name,
                duration.as_secs_f64()
            );
            timings.push((crate_name, duration));
        } else {
            failed.push(crate_name);
            eprintln!("✗ {} failed to compile:", crate_name);
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }

    println!("\n=== Compilation Performance ===");
    println!(
        "Compilation test summary: {}/{} crates passed",
        passed, tested
    );
    println!("\nIndividual crate compilation times:");
    for (name, duration) in &timings {
        println!("  {} - {:.2}s", name, duration.as_secs_f64());
    }
    let total_time: std::time::Duration = timings.iter().map(|(_, d)| *d).sum();
    println!("Total compilation time: {:.2}s", total_time.as_secs_f64());
    println!("===============================\n");

    assert!(tested > 0, "No crates found to test. Run `just generate`.");
    assert!(
        failed.is_empty(),
        "Some crates failed to compile: {:?}",
        failed
    );
}

// ============================================================================
// Pattern Tests - These test code generation patterns without depending
// on specific IDL field structures
// ============================================================================

#[test]
fn test_lib_rs_structure() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let lib_path = format!("generated/{}/src/lib.rs", crate_name);
        if !Path::new(&lib_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&lib_path).expect("Failed to read lib.rs");

        // Check for required declarations
        assert!(
            content.contains("declare_id!"),
            "{} lib.rs should have declare_id! macro",
            crate_name
        );
        assert!(
            content.contains("pub mod types"),
            "{} lib.rs should declare types module",
            crate_name
        );
        assert!(
            content.contains("pub mod accounts"),
            "{} lib.rs should declare accounts module",
            crate_name
        );
        assert!(
            content.contains("pub mod instructions"),
            "{} lib.rs should declare instructions module",
            crate_name
        );
        assert!(
            content.contains("pub mod errors"),
            "{} lib.rs should declare errors module",
            crate_name
        );
        assert!(
            content.contains("pub mod events"),
            "{} lib.rs should declare events module",
            crate_name
        );

        // Check for re-exports
        assert!(
            content.contains("pub use types::*"),
            "{} lib.rs should re-export types",
            crate_name
        );
        assert!(
            content.contains("pub use accounts::*"),
            "{} lib.rs should re-export accounts",
            crate_name
        );

        // Check for serde helper if feature is present
        if content.contains("#[cfg(feature = \"serde\")]") {
            assert!(
                content.contains("serialize_pubkey_as_string"),
                "{} lib.rs should have Pubkey serde helper",
                crate_name
            );
        }

        println!("✓ {} lib.rs has correct structure", crate_name);
    }
}

#[test]
fn test_accounts_have_discriminators() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let accounts_path = format!("generated/{}/src/accounts.rs", crate_name);
        if !Path::new(&accounts_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&accounts_path).expect("Failed to read accounts.rs");

        // Look for DISCRIMINATOR constants
        if content.contains("pub const") && content.contains("DISCRIMINATOR") {
            println!("✓ {} has account discriminators", crate_name);

            // Check that discriminators are the right size
            assert!(
                content.contains(": [u8; 8]"),
                "{} discriminators should be [u8; 8]",
                crate_name
            );

            // Check for serialize_with_discriminator method
            assert!(
                content.contains("serialize_with_discriminator")
                    || content.contains("fn serialize<W: std::io::Write>"),
                "{} should have discriminator serialization methods",
                crate_name
            );
        }
    }
}

#[test]
fn test_events_have_wrapper_pattern() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let events_path = format!("generated/{}/src/events.rs", crate_name);
        if !Path::new(&events_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&events_path).expect("Failed to read events.rs");

        // Skip if no events
        if !content.contains("pub struct") {
            continue;
        }

        // Look for event discriminator constants
        if content.contains("_EVENT_DISCM") {
            println!("✓ {} has event discriminators", crate_name);

            // Check for wrapper pattern (EventNameEvent structs)
            assert!(
                content.contains("Event(pub") || content.contains("pub struct"),
                "{} should have event wrapper structs",
                crate_name
            );

            // Check for custom deserialize method
            assert!(
                content.contains("fn deserialize(buf: &mut &[u8])"),
                "{} events should have deserialize method",
                crate_name
            );
        }
    }
}

#[test]
fn test_instructions_have_enum() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let instructions_path = format!("generated/{}/src/instructions.rs", crate_name);
        if !Path::new(&instructions_path).exists() {
            continue;
        }

        let content =
            fs::read_to_string(&instructions_path).expect("Failed to read instructions.rs");

        // Check for Instruction enum
        assert!(
            content.contains("pub enum Instruction"),
            "{} should have Instruction enum",
            crate_name
        );

        // Check for Borsh traits
        assert!(
            content.contains("BorshSerialize") && content.contains("BorshDeserialize"),
            "{} instructions should derive Borsh traits",
            crate_name
        );

        // Check for Debug trait
        assert!(
            content.contains("Debug"),
            "{} instructions should derive Debug",
            crate_name
        );

        println!("✓ {} has Instruction enum with correct traits", crate_name);
    }
}

#[test]
fn test_errors_have_enum() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let errors_path = format!("generated/{}/src/errors.rs", crate_name);
        if !Path::new(&errors_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&errors_path).expect("Failed to read errors.rs");

        // Skip if no errors defined
        if !content.contains("pub enum") {
            continue;
        }

        // Look for error enum (name varies by program)
        assert!(
            content.contains("Error"),
            "{} should have an Error enum",
            crate_name
        );

        // Check for required traits
        assert!(
            content.contains("Debug"),
            "{} errors should derive Debug",
            crate_name
        );

        assert!(
            content.contains("thiserror::Error") || content.contains("Display"),
            "{} errors should implement Display",
            crate_name
        );

        // Check for num-derive traits for error code conversion
        assert!(
            content.contains("FromPrimitive") || content.contains("ToPrimitive"),
            "{} errors should derive FromPrimitive/ToPrimitive",
            crate_name
        );

        println!("✓ {} has Error enum with correct traits", crate_name);
    }
}

#[test]
fn test_cargo_toml_structure() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let cargo_path = format!("generated/{}/Cargo.toml", crate_name);
        if !Path::new(&cargo_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&cargo_path).expect("Failed to read Cargo.toml");

        // Check for required dependencies
        assert!(
            content.contains("borsh"),
            "{} should depend on borsh",
            crate_name
        );
        assert!(
            content.contains("bytemuck"),
            "{} should depend on bytemuck",
            crate_name
        );
        assert!(
            content.contains("solana-program"),
            "{} should depend on solana-program",
            crate_name
        );
        assert!(
            content.contains("thiserror"),
            "{} should depend on thiserror",
            crate_name
        );
        assert!(
            content.contains("num-derive"),
            "{} should depend on num-derive",
            crate_name
        );
        assert!(
            content.contains("num-traits"),
            "{} should depend on num-traits",
            crate_name
        );

        // Check for serde feature
        assert!(
            content.contains("[features]") && content.contains("serde"),
            "{} should have serde feature",
            crate_name
        );

        println!("✓ {} Cargo.toml has correct dependencies", crate_name);
    }
}

#[test]
fn test_pubkey_serde_serialization() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let lib_path = format!("generated/{}/src/lib.rs", crate_name);
        if !Path::new(&lib_path).exists() {
            continue;
        }

        let lib_content = fs::read_to_string(&lib_path).expect("Failed to read lib.rs");

        // Check for Pubkey serialization helper
        if lib_content.contains("serialize_pubkey_as_string") {
            // Check that it's properly gated behind serde feature
            assert!(
                lib_content.contains("#[cfg(feature = \"serde\")]"),
                "{} Pubkey helper should be behind serde feature",
                crate_name
            );

            // Check that events/accounts use this helper
            let events_path = format!("generated/{}/src/events.rs", crate_name);
            if Path::new(&events_path).exists() {
                let events_content =
                    fs::read_to_string(&events_path).expect("Failed to read events.rs");
                if events_content.contains("Pubkey") {
                    assert!(
                        events_content
                            .contains("serialize_with = \"crate::serialize_pubkey_as_string\""),
                        "{} events should use Pubkey serialization helper",
                        crate_name
                    );
                }
            }

            println!("✓ {} has Pubkey serde serialization", crate_name);
        }
    }
}

// ============================================================================
// Summary Test
// ============================================================================

#[test]
fn test_summary() {
    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    let mut summary = Vec::new();

    for crate_name in &crates {
        let crate_path = format!("generated/{}", crate_name);
        if !Path::new(&crate_path).exists() {
            summary.push(format!("⚠️  {} - not generated", crate_name));
        } else {
            summary.push(format!("✓ {} - present and tested", crate_name));
        }
    }

    println!("\n=== Integration Test Summary ===");
    for line in &summary {
        println!("{}", line);
    }
    println!("================================\n");
}

// ============================================================================
// Event Parsing Helpers Tests
// ============================================================================

#[test]
fn test_events_have_parsing_helpers() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let events_path = format!("generated/{}/src/events.rs", crate_name);
        if !Path::new(&events_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&events_path).expect("Failed to read events.rs");

        // Skip if no events
        if !content.contains("pub struct") && !content.contains("_EVENT_DISCM") {
            continue;
        }

        // Check for event parsing helpers
        if content.contains("_EVENT_DISCM") {
            // Should have ParsedEvent enum
            assert!(
                content.contains("enum ParsedEvent") || content.contains("pub enum ParsedEvent"),
                "{} should have ParsedEvent enum for event parsing",
                crate_name
            );

            // Should have EventParseError
            assert!(
                content.contains("enum EventParseError")
                    || content.contains("pub enum EventParseError"),
                "{} should have EventParseError enum",
                crate_name
            );

            // Should have parse_event function
            assert!(
                content.contains("fn parse_event") || content.contains("pub fn parse_event"),
                "{} should have parse_event function",
                crate_name
            );

            // Should have parse_events_from_data function
            assert!(
                content.contains("fn parse_events_from_data")
                    || content.contains("pub fn parse_events_from_data"),
                "{} should have parse_events_from_data function",
                crate_name
            );

            println!("✓ {} has event parsing helpers", crate_name);
        }
    }
}

// ============================================================================
// Account Validation Helpers Tests
// ============================================================================

#[test]
fn test_accounts_have_validation_helpers() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let accounts_path = format!("generated/{}/src/accounts.rs", crate_name);
        if !Path::new(&accounts_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&accounts_path).expect("Failed to read accounts.rs");

        // Skip if no accounts
        if !content.contains("pub struct") && !content.contains("DISCRIMINATOR") {
            continue;
        }

        // Check for validation helpers if accounts have discriminators
        if content.contains("DISCRIMINATOR") {
            // Should have ValidationError enum
            assert!(
                content.contains("enum ValidationError")
                    || content.contains("pub enum ValidationError"),
                "{} should have ValidationError enum for account validation",
                crate_name
            );

            // Should have validate_account_info method
            assert!(
                content.contains("fn validate_account_info")
                    || content.contains("pub fn validate_account_info"),
                "{} should have validate_account_info method",
                crate_name
            );

            // Should have try_from_account_info method
            assert!(
                content.contains("fn try_from_account_info")
                    || content.contains("pub fn try_from_account_info"),
                "{} should have try_from_account_info method",
                crate_name
            );

            println!("✓ {} has account validation helpers", crate_name);
        }
    }
}

// ============================================================================
// Example Files Tests
// ============================================================================

#[test]
fn test_example_files_generated() {
    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    let mut tested = 0;
    let mut passed = 0;

    for crate_name in &crates {
        let crate_path = format!("generated/{}", crate_name);
        if !Path::new(&crate_path).exists() {
            continue;
        }

        tested += 1;

        let examples_dir = format!("{}/examples", crate_path);
        let build_ix_example = format!("{}/build_instruction.rs", examples_dir);
        let parse_account_example = format!("{}/parse_account.rs", examples_dir);
        let parse_events_example = format!("{}/parse_events.rs", examples_dir);

        let mut all_present = true;
        let mut missing = Vec::new();

        if !Path::new(&build_ix_example).exists() {
            all_present = false;
            missing.push("build_instruction.rs");
        }
        if !Path::new(&parse_account_example).exists() {
            all_present = false;
            missing.push("parse_account.rs");
        }
        if !Path::new(&parse_events_example).exists() {
            all_present = false;
            missing.push("parse_events.rs");
        }

        if all_present {
            passed += 1;
            println!("✓ {} has all example files", crate_name);
        } else {
            eprintln!("✗ {} missing example files: {:?}", crate_name, missing);
        }
    }

    assert!(
        tested > 0,
        "No generated crates found to test. Run `just generate`."
    );

    if passed == tested {
        println!("✓ All {} tested crates have example files", tested);
    } else {
        eprintln!(
            "⚠️  Only {}/{} crates have all example files",
            passed, tested
        );
    }
}

#[test]
fn test_example_files_content() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let examples_dir = format!("generated/{}/examples", crate_name);
        if !Path::new(&examples_dir).exists() {
            continue;
        }

        // Check build_instruction.rs
        let build_ix_path = format!("{}/build_instruction.rs", examples_dir);
        if Path::new(&build_ix_path).exists() {
            let content =
                fs::read_to_string(&build_ix_path).expect("Failed to read build_instruction.rs");
            // Check for imports (either active or commented) or instruction building references
            // If no instructions are defined, the file will be minimal with just a main function
            let has_imports = (content.contains("use") && (content.contains("::*") || content.contains("::{"))) || content.contains("// use");
            let has_instruction_refs = content.contains("Keys") || content.contains("_ix") || content.contains("IxArgs");
            let no_instructions_defined = content.contains("No instructions defined");
            
            assert!(
                has_imports || has_instruction_refs || no_instructions_defined,
                "{} build_instruction.rs should have imports, instruction references, or indicate no instructions",
                crate_name
            );
            assert!(
                content.contains("fn main"),
                "{} build_instruction.rs should have a main function",
                crate_name
            );
        }

        // Check parse_account.rs
        let parse_account_path = format!("{}/parse_account.rs", examples_dir);
        if Path::new(&parse_account_path).exists() {
            let content =
                fs::read_to_string(&parse_account_path).expect("Failed to read parse_account.rs");
            // Check for imports (either active or commented) or account parsing references
            // If no accounts are defined, the file will be minimal with just a main function
            let has_imports = (content.contains("use") && (content.contains("::*") || content.contains("::{"))) || content.contains("// use");
            let has_account_refs = content.contains("AccountInfo") || content.contains("try_from_account_info");
            let no_accounts_defined = content.contains("No accounts defined");
            
            assert!(
                has_imports || has_account_refs || no_accounts_defined,
                "{} parse_account.rs should have imports, account parsing references, or indicate no accounts",
                crate_name
            );
            // Only check for account parsing if accounts are actually defined
            if !no_accounts_defined {
                assert!(
                    has_account_refs,
                    "{} parse_account.rs should reference account parsing when accounts are defined",
                    crate_name
                );
            }
        }

        // Check parse_events.rs
        let parse_events_path = format!("{}/parse_events.rs", examples_dir);
        if Path::new(&parse_events_path).exists() {
            let content =
                fs::read_to_string(&parse_events_path).expect("Failed to read parse_events.rs");
            // Check for imports (either active or commented) or event parsing references
            // If no events are defined, the file will be minimal with just a main function
            let has_events = content.contains("parse_event") || content.contains("ParsedEvent");
            let has_imports = (content.contains("use") && (content.contains("::*") || content.contains("::{"))) || content.contains("// use");
            let no_events_defined = content.contains("No events defined");
            
            assert!(
                has_imports || has_events || no_events_defined,
                "{} parse_events.rs should have imports, event parsing references, or indicate no events",
                crate_name
            );
            // Only check for event parsing if events are actually defined
            if !no_events_defined {
                assert!(
                    has_events,
                    "{} parse_events.rs should reference event parsing when events are defined",
                    crate_name
                );
            }
        }

        println!("✓ {} example files have correct content", crate_name);
    }
}

// ============================================================================
// Enhanced Documentation Tests
// ============================================================================

#[test]
fn test_enhanced_documentation() {
    use std::fs;

    let crates = [
        "pumpfun",
        "pumpfun_amm",
        "raydium_amm",
        "raydium_clmm",
        "raydium_cpmm",
    ];

    for crate_name in &crates {
        let events_path = format!("generated/{}/src/events.rs", crate_name);
        if !Path::new(&events_path).exists() {
            continue;
        }

        let content = fs::read_to_string(&events_path).expect("Failed to read events.rs");

        // Check for enhanced documentation in events
        if content.contains("pub struct") && content.contains("Event:") {
            // Should have usage examples in doc comments
            let has_usage = content.contains("# Usage") || content.contains("/// # Usage");
            let has_example = content.contains("```no_run") || content.contains("```rust");

            if has_usage || has_example {
                println!("✓ {} events have enhanced documentation", crate_name);
            }
        }
    }
}
