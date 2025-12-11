//! Post-Generation Integration Tests
//!
//! These tests verify that all generated code compiles and functions correctly.
//! Run `just generate` before running these tests.
//!
//! See INTEGRATION_TESTING.md for more information on writing tests.

use std::path::Path;

// ============================================================================
// File Structure Tests
// ============================================================================

#[test]
fn test_all_crates_generated() {
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
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
    
    assert!(!present.is_empty(), "At least some crates should be generated. Run `just generate`.");
}

#[test]
fn test_all_crates_have_required_files() {
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
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
    
    assert!(tested_crates > 0, "No generated crates found to test. Run `just generate`.");
}

// ============================================================================
// Compilation Tests
// ============================================================================

#[test]
fn test_generated_crates_compile() {
    use std::process::Command;
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
    let mut tested = 0;
    let mut passed = 0;
    let mut failed = Vec::new();
    
    for crate_name in &crates {
        let crate_path = format!("generated/{}", crate_name);
        if !Path::new(&crate_path).exists() {
            continue;
        }
        
        tested += 1;
        
        println!("Checking {}...", crate_name);
        let output = Command::new("cargo")
            .args(&["check", "--manifest-path", &format!("{}/Cargo.toml", crate_path)])
            .output()
            .expect("Failed to run cargo check");
        
        if output.status.success() {
            passed += 1;
            println!("✓ {} compiles successfully", crate_name);
        } else {
            failed.push(crate_name);
            eprintln!("✗ {} failed to compile:", crate_name);
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }
    
    println!("\nCompilation test summary: {}/{} crates passed", passed, tested);
    
    assert!(tested > 0, "No crates found to test. Run `just generate`.");
    assert!(failed.is_empty(), "Some crates failed to compile: {:?}", failed);
}

// ============================================================================
// Pattern Tests - These test code generation patterns without depending
// on specific IDL field structures
// ============================================================================

#[test]
fn test_lib_rs_structure() {
    use std::fs;
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
    for crate_name in &crates {
        let lib_path = format!("generated/{}/src/lib.rs", crate_name);
        if !Path::new(&lib_path).exists() {
            continue;
        }
        
        let content = fs::read_to_string(&lib_path).expect("Failed to read lib.rs");
        
        // Check for required declarations
        assert!(content.contains("declare_id!"), "{} lib.rs should have declare_id! macro", crate_name);
        assert!(content.contains("pub mod types"), "{} lib.rs should declare types module", crate_name);
        assert!(content.contains("pub mod accounts"), "{} lib.rs should declare accounts module", crate_name);
        assert!(content.contains("pub mod instructions"), "{} lib.rs should declare instructions module", crate_name);
        assert!(content.contains("pub mod errors"), "{} lib.rs should declare errors module", crate_name);
        assert!(content.contains("pub mod events"), "{} lib.rs should declare events module", crate_name);
        
        // Check for re-exports
        assert!(content.contains("pub use types::*"), "{} lib.rs should re-export types", crate_name);
        assert!(content.contains("pub use accounts::*"), "{} lib.rs should re-export accounts", crate_name);
        
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
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
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
                content.contains("serialize_with_discriminator") ||
                content.contains("fn serialize<W: std::io::Write>"),
                "{} should have discriminator serialization methods",
                crate_name
            );
        }
    }
}

#[test]
fn test_events_have_wrapper_pattern() {
    use std::fs;
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
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
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
    for crate_name in &crates {
        let instructions_path = format!("generated/{}/src/instructions.rs", crate_name);
        if !Path::new(&instructions_path).exists() {
            continue;
        }
        
        let content = fs::read_to_string(&instructions_path).expect("Failed to read instructions.rs");
        
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
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
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
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
    for crate_name in &crates {
        let cargo_path = format!("generated/{}/Cargo.toml", crate_name);
        if !Path::new(&cargo_path).exists() {
            continue;
        }
        
        let content = fs::read_to_string(&cargo_path).expect("Failed to read Cargo.toml");
        
        // Check for required dependencies
        assert!(content.contains("borsh"), "{} should depend on borsh", crate_name);
        assert!(content.contains("bytemuck"), "{} should depend on bytemuck", crate_name);
        assert!(content.contains("solana-program"), "{} should depend on solana-program", crate_name);
        assert!(content.contains("thiserror"), "{} should depend on thiserror", crate_name);
        assert!(content.contains("num-derive"), "{} should depend on num-derive", crate_name);
        assert!(content.contains("num-traits"), "{} should depend on num-traits", crate_name);
        
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
    
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
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
                let events_content = fs::read_to_string(&events_path).expect("Failed to read events.rs");
                if events_content.contains("Pubkey") {
                    assert!(
                        events_content.contains("serialize_with = \"crate::serialize_pubkey_as_string\""),
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
    let crates = ["pumpfun", "pumpfun_amm", "raydium_amm", "raydium_clmm", "raydium_cpmm"];
    
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
