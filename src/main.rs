use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

use solana_idl_codegen::{codegen, idl};

#[derive(Parser)]
#[command(name = "solana-idl-codegen")]
#[command(about = "Generate Rust code bindings from Solana IDL files", long_about = None)]
struct Cli {
    /// Path to the IDL JSON file
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// Output directory for generated code
    #[arg(short, long, value_name = "DIR", default_value = "generated")]
    output: PathBuf,

    /// Module name for generated code
    #[arg(short, long, default_value = "program")]
    module: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Read and parse IDL file
    let idl_content = fs::read_to_string(&cli.input)
        .context(format!("Failed to read IDL file: {:?}", cli.input))?;

    let idl: idl::Idl = serde_json::from_str(&idl_content).context("Failed to parse IDL JSON")?;

    println!("Successfully parsed IDL for program: {}", idl.get_name());
    println!("Version: {}", idl.get_version());
    println!("Instructions: {}", idl.instructions.len());
    println!(
        "Accounts: {}",
        idl.accounts.as_ref().map(|a| a.len()).unwrap_or(0)
    );
    println!(
        "Types: {}",
        idl.types.as_ref().map(|t| t.len()).unwrap_or(0)
    );

    // Generate code
    let generated_code = codegen::generate(&idl, &cli.module)?;

    // Create crate structure
    let crate_dir = cli.output.join(&cli.module);
    let src_dir = crate_dir.join("src");

    fs::create_dir_all(&src_dir).context(format!(
        "Failed to create crate source directory: {:?}",
        src_dir
    ))?;

    // Write lib.rs
    let lib_file = src_dir.join("lib.rs");
    fs::write(&lib_file, &generated_code.lib)
        .context(format!("Failed to write lib.rs: {:?}", lib_file))?;

    // Write types.rs (may be empty)
    if !generated_code.types.is_empty() {
        let types_file = src_dir.join("types.rs");
        fs::write(&types_file, &generated_code.types)
            .context(format!("Failed to write types.rs: {:?}", types_file))?;
    } else {
        // Write empty types module
        let types_file = src_dir.join("types.rs");
        fs::write(&types_file, "// No custom types defined\n")
            .context(format!("Failed to write types.rs: {:?}", types_file))?;
    }

    // Write accounts.rs (may be empty)
    if !generated_code.accounts.is_empty() {
        let accounts_file = src_dir.join("accounts.rs");
        fs::write(&accounts_file, &generated_code.accounts)
            .context(format!("Failed to write accounts.rs: {:?}", accounts_file))?;
    } else {
        // Write empty accounts module
        let accounts_file = src_dir.join("accounts.rs");
        fs::write(&accounts_file, "// No accounts defined\n")
            .context(format!("Failed to write accounts.rs: {:?}", accounts_file))?;
    }

    // Write instructions.rs
    let instructions_file = src_dir.join("instructions.rs");
    fs::write(&instructions_file, &generated_code.instructions).context(format!(
        "Failed to write instructions.rs: {:?}",
        instructions_file
    ))?;

    // Write errors.rs (may be empty)
    if !generated_code.errors.is_empty() {
        let errors_file = src_dir.join("errors.rs");
        fs::write(&errors_file, &generated_code.errors)
            .context(format!("Failed to write errors.rs: {:?}", errors_file))?;
    } else {
        // Write empty errors module
        let errors_file = src_dir.join("errors.rs");
        fs::write(&errors_file, "// No errors defined\n")
            .context(format!("Failed to write errors.rs: {:?}", errors_file))?;
    }

    // Write events.rs (may be empty)
    if !generated_code.events.is_empty() {
        let events_file = src_dir.join("events.rs");
        fs::write(&events_file, &generated_code.events)
            .context(format!("Failed to write events.rs: {:?}", events_file))?;
    } else {
        // Write empty events module
        let events_file = src_dir.join("events.rs");
        fs::write(&events_file, "// No events defined\n")
            .context(format!("Failed to write events.rs: {:?}", events_file))?;
    }

    // Generate Cargo.toml
    let cargo_toml = generate_cargo_toml(&cli.module, &idl);
    let cargo_toml_file = crate_dir.join("Cargo.toml");
    fs::write(&cargo_toml_file, cargo_toml)
        .context(format!("Failed to write Cargo.toml: {:?}", cargo_toml_file))?;

    // Generate README.md
    let readme = generate_readme(&cli.module, &idl);
    let readme_file = crate_dir.join("README.md");
    fs::write(&readme_file, readme)
        .context(format!("Failed to write README.md: {:?}", readme_file))?;

    // Generate .gitignore
    let gitignore = "/target\n/Cargo.lock\n";
    let gitignore_file = crate_dir.join(".gitignore");
    fs::write(&gitignore_file, gitignore)
        .context(format!("Failed to write .gitignore: {:?}", gitignore_file))?;

    // Generate example files
    let examples_dir = crate_dir.join("examples");
    fs::create_dir_all(&examples_dir).context(format!(
        "Failed to create examples directory: {:?}",
        examples_dir
    ))?;

    generate_examples(&examples_dir, &cli.module, &idl)?;

    // Format generated code with rustfmt
    let mut rustfmt_files = Vec::new();
    rustfmt_files.push(src_dir.join("lib.rs"));
    rustfmt_files.push(src_dir.join("instructions.rs"));
    if !generated_code.types.is_empty() {
        rustfmt_files.push(src_dir.join("types.rs"));
    }
    if !generated_code.accounts.is_empty() {
        rustfmt_files.push(src_dir.join("accounts.rs"));
    }
    if !generated_code.errors.is_empty() {
        rustfmt_files.push(src_dir.join("errors.rs"));
    }
    if !generated_code.events.is_empty() {
        rustfmt_files.push(src_dir.join("events.rs"));
    }

    let rustfmt_args: Vec<&str> = rustfmt_files.iter().filter_map(|p| p.to_str()).collect();

    if !rustfmt_args.is_empty() {
        let rustfmt_result = std::process::Command::new("rustfmt")
            .arg("--edition")
            .arg("2021")
            .args(&rustfmt_args)
            .output();

        if let Err(e) = rustfmt_result {
            eprintln!("Warning: Failed to run rustfmt: {}. Generated code may not be formatted correctly.", e);
        } else if let Ok(output) = rustfmt_result {
            if !output.status.success() {
                eprintln!("Warning: rustfmt exited with non-zero status. Generated code may not be formatted correctly.");
            }
        }
    }

    println!("\n✓ Generated crate at: {:?}", crate_dir);
    println!("  ├── Cargo.toml");
    println!("  ├── README.md");
    println!("  ├── .gitignore");
    println!("  ├── examples/");
    println!("  │   ├── build_instruction.rs");
    println!("  │   ├── parse_account.rs");
    println!("  │   └── parse_events.rs");
    println!("  └── src/");
    println!("      ├── lib.rs");
    println!("      ├── types.rs");
    println!("      ├── accounts.rs");
    println!("      ├── instructions.rs");
    println!("      ├── errors.rs");
    println!("      └── events.rs");

    Ok(())
}

fn generate_cargo_toml(module_name: &str, idl: &idl::Idl) -> String {
    format!(
        r#"[package]
name = "{}"
version = "{}"
edition = "2021"
description = "Rust bindings for {} Solana program"
license = "MIT OR Apache-2.0"

[dependencies]
borsh = {{ version = "^1.5", features = ["derive"] }}
bytemuck = {{ version = "^1.14", features = ["derive"] }}
solana-program = "^2.2"
thiserror = "^2.0"
num-derive = "^0.4"
num-traits = "^0.2"

[dependencies.serde]
version = "^1.0"
features = ["derive"]
optional = true

[features]
default = ["serde"]
serde = ["dep:serde"]

[lib]
crate-type = ["lib"]
"#,
        module_name,
        idl.get_version(),
        idl.get_name()
    )
}

fn generate_readme(module_name: &str, idl: &idl::Idl) -> String {
    format!(
        r#"# {}

Rust bindings for the {} Solana program.

## Overview

- **Program**: {}
- **Version**: {}
- **Instructions**: {}
- **Accounts**: {}
- **Types**: {}

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
{} = {{ path = "path/to/{}" }}
```

Then import in your Rust code:

```rust
use {}::*;
```

## Features

- Type-safe instruction builders
- Borsh serialization/deserialization
- Account type definitions
- Custom type definitions

## Generated Code

This crate was automatically generated from the Solana IDL using `solana-idl-codegen`.

## License

MIT OR Apache-2.0
"#,
        module_name,
        idl.get_name(),
        idl.get_name(),
        idl.get_version(),
        idl.instructions.len(),
        idl.accounts.as_ref().map(|a| a.len()).unwrap_or(0),
        idl.types.as_ref().map(|t| t.len()).unwrap_or(0),
        module_name,
        module_name,
        module_name
    )
}

fn generate_examples(examples_dir: &PathBuf, module_name: &str, idl: &idl::Idl) -> Result<()> {
    // Example 1: Building an instruction
    let build_instruction_example = if !idl.instructions.is_empty() {
        let first_ix = &idl.instructions[0];
        let ix_name_snake = first_ix.name.to_snake_case();
        let ix_name_pascal = first_ix.name.to_pascal_case();
        format!(
            r#"//! Example: Building an instruction
//!
//! This example shows how to build a transaction instruction using the generated bindings.

use {}::*;
use solana_program::pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Build {} instruction
    let keys = {}Keys {{
        // TODO: Fill in account pubkeys based on your IDL
    }};
    {}
    let instruction = {}(keys{})?;
    println!("Built instruction: {{:?}}", instruction);
    
    Ok(())
}}
"#,
            module_name,
            first_ix.name,
            ix_name_pascal,
            if !first_ix.args.is_empty() {
                format!(
                    "    let args = {}IxArgs {{\n        // TODO: Fill in instruction arguments\n    }};\n    ",
                    ix_name_pascal
                )
            } else {
                String::new()
            },
            ix_name_snake,
            if !first_ix.args.is_empty() {
                ", args"
            } else {
                ""
            }
        )
    } else {
        format!(
            r#"//! Example: Building an instruction
//!
//! This example shows how to build a transaction instruction using the generated bindings.

use {}::*;
use solana_program::pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // No instructions defined in IDL
    Ok(())
}}
"#,
            module_name
        )
    };

    let build_ix_file = examples_dir.join("build_instruction.rs");
    fs::write(&build_ix_file, build_instruction_example)
        .context(format!("Failed to write build_instruction.rs: {:?}", build_ix_file))?;

    // Example 2: Parsing an account
    let parse_account_example = if let Some(accounts) = &idl.accounts {
        if !accounts.is_empty() {
            let first_account = &accounts[0];
            format!(
                r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

use {}::*;
use solana_program::account_info::AccountInfo;

fn parse_account_example(account_info: &AccountInfo) -> Result<(), Box<dyn std::error::Error>> {{
    // Parse and validate {} account
    let account = {}::try_from_account_info(account_info)?;
    println!("Parsed account: {{:?}}", account);
    
    Ok(())
}}
"#,
                module_name,
                first_account.name,
                first_account.name
            )
        } else {
            format!(
                r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

use {}::*;
use solana_program::account_info::AccountInfo;

fn parse_account_example(_account_info: &AccountInfo) -> Result<(), Box<dyn std::error::Error>> {{
    // No accounts defined in IDL
    Ok(())
}}
"#,
                module_name
            )
        }
    } else {
        format!(
            r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

use {}::*;
use solana_program::account_info::AccountInfo;

fn parse_account_example(_account_info: &AccountInfo) -> Result<(), Box<dyn std::error::Error>> {{
    // No accounts defined in IDL
    Ok(())
}}
"#,
            module_name
        )
    };

    let parse_account_file = examples_dir.join("parse_account.rs");
    fs::write(&parse_account_file, parse_account_example)
        .context(format!("Failed to write parse_account.rs: {:?}", parse_account_file))?;

    // Example 3: Parsing events
    let parse_events_example = if let Some(events) = &idl.events {
        if !events.is_empty() {
            let mut match_arms = String::new();
            for event in events.iter().take(3) {
                let variant_name = event.name.to_pascal_case();
                match_arms.push_str(&format!(
                    "        Ok(ParsedEvent::{}(e)) => println!(\"Parsed {} event: {{:?}}\", e),\n        ",
                    variant_name,
                    event.name
                ));
            }
            match_arms.push_str("_ => {}");
            format!(
                r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

use {}::*;

fn parse_events_example(event_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {{
    // Parse a single event
    match parse_event(event_data) {{
        {}
        Err(e) => eprintln!("Failed to parse event: {{}}", e),
    }}
    
    Ok(())
}}
"#,
                module_name,
                match_arms
            )
        } else {
            format!(
                r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

use {}::*;

fn parse_events_example(_event_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {{
    // No events defined in IDL
    Ok(())
}}
"#,
                module_name
            )
        }
    } else {
        format!(
            r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

use {}::*;

fn parse_events_example(_event_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {{
    // No events defined in IDL
    Ok(())
}}
"#,
            module_name
        )
    };

    let parse_events_file = examples_dir.join("parse_events.rs");
    fs::write(&parse_events_file, parse_events_example)
        .context(format!("Failed to write parse_events.rs: {:?}", parse_events_file))?;

    Ok(())
}

fn generate_examples(examples_dir: &PathBuf, module_name: &str, idl: &idl::Idl) -> Result<()> {
    // Example 1: Building an instruction
    let build_instruction_example = if !idl.instructions.is_empty() {
        let first_ix = &idl.instructions[0];
        let ix_name_snake = first_ix.name.to_snake_case();
        let ix_name_pascal = first_ix.name.to_pascal_case();
        format!(
            r#"//! Example: Building an instruction
//!
//! This example shows how to build a transaction instruction using the generated bindings.

use {}::*;
use solana_program::pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Build {} instruction
    let keys = {}Keys {{
        // TODO: Fill in account pubkeys based on your IDL
    }};
    {}
    let instruction = {}(keys{})?;
    println!("Built instruction: {{:?}}", instruction);
    
    Ok(())
}}
"#,
            module_name,
            first_ix.name,
            ix_name_pascal,
            if !first_ix.args.is_empty() {
                format!(
                    "    let args = {}IxArgs {{\n        // TODO: Fill in instruction arguments\n    }};\n    ",
                    ix_name_pascal
                )
            } else {
                String::new()
            },
            ix_name_snake,
            if !first_ix.args.is_empty() {
                ", args"
            } else {
                ""
            }
        )
    } else {
        format!(
            r#"//! Example: Building an instruction
//!
//! This example shows how to build a transaction instruction using the generated bindings.

use {}::*;
use solana_program::pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // No instructions defined in IDL
    Ok(())
}}
"#,
            module_name
        )
    };

    let build_ix_file = examples_dir.join("build_instruction.rs");
    fs::write(&build_ix_file, build_instruction_example)
        .context(format!("Failed to write build_instruction.rs: {:?}", build_ix_file))?;

    // Example 2: Parsing an account
    let parse_account_example = if let Some(accounts) = &idl.accounts {
        if !accounts.is_empty() {
            let first_account = &accounts[0];
            format!(
                r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

use {}::*;
use solana_program::account_info::AccountInfo;

fn parse_account_example(account_info: &AccountInfo) -> Result<(), Box<dyn std::error::Error>> {{
    // Parse and validate {} account
    let account = {}::try_from_account_info(account_info)?;
    println!("Parsed account: {{:?}}", account);
    
    Ok(())
}}
"#,
                module_name,
                first_account.name,
                first_account.name
            )
        } else {
            format!(
                r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

use {}::*;
use solana_program::account_info::AccountInfo;

fn parse_account_example(_account_info: &AccountInfo) -> Result<(), Box<dyn std::error::Error>> {{
    // No accounts defined in IDL
    Ok(())
}}
"#,
                module_name
            )
        }
    } else {
        format!(
            r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

use {}::*;
use solana_program::account_info::AccountInfo;

fn parse_account_example(_account_info: &AccountInfo) -> Result<(), Box<dyn std::error::Error>> {{
    // No accounts defined in IDL
    Ok(())
}}
"#,
            module_name
        )
    };

    let parse_account_file = examples_dir.join("parse_account.rs");
    fs::write(&parse_account_file, parse_account_example)
        .context(format!("Failed to write parse_account.rs: {:?}", parse_account_file))?;

    // Example 3: Parsing events
    let parse_events_example = if let Some(events) = &idl.events {
        if !events.is_empty() {
            let mut match_arms = String::new();
            for event in events.iter().take(3) {
                let variant_name = event.name.to_pascal_case();
                match_arms.push_str(&format!(
                    "        Ok(ParsedEvent::{}(e)) => println!(\"Parsed {} event: {{:?}}\", e),\n        ",
                    variant_name,
                    event.name
                ));
            }
            match_arms.push_str("_ => {}");
            format!(
                r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

use {}::*;

fn parse_events_example(event_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {{
    // Parse a single event
    match parse_event(event_data) {{
        {}
        Err(e) => eprintln!("Failed to parse event: {{}}", e),
    }}
    
    Ok(())
}}
"#,
                module_name,
                match_arms
            )
        } else {
            format!(
                r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

use {}::*;

fn parse_events_example(_event_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {{
    // No events defined in IDL
    Ok(())
}}
"#,
                module_name
            )
        }
    } else {
        format!(
            r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

use {}::*;

fn parse_events_example(_event_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {{
    // No events defined in IDL
    Ok(())
}}
"#,
            module_name
        )
    };

    let parse_events_file = examples_dir.join("parse_events.rs");
    fs::write(&parse_events_file, parse_events_example)
        .context(format!("Failed to write parse_events.rs: {:?}", parse_events_file))?;

    Ok(())
}
