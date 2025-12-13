use anyhow::{Context, Result};
use clap::Parser;
use heck::{ToPascalCase, ToSnakeCase};
use std::fs;
use std::path::{Path, PathBuf};

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

fn generate_examples(examples_dir: &Path, module_name: &str, idl: &idl::Idl) -> Result<()> {
    // Example 1: Building an instruction
    let build_instruction_example = if !idl.instructions.is_empty() {
        let first_ix = &idl.instructions[0];
        let ix_name_snake = first_ix.name.to_snake_case();
        let ix_name_pascal = first_ix.name.to_pascal_case();

        // Generate keys struct initialization with all fields (commented out)
        let mut keys_fields = String::new();
        for account in &first_ix.accounts {
            let field_name = account.name.to_snake_case();
            keys_fields.push_str(&format!(
                "    //     {}: solana_program::pubkey::Pubkey::default(), // TODO: Fill in actual pubkey\n",
                field_name
            ));
        }

        // Generate args struct initialization if needed (commented out)
        let args_init = if !first_ix.args.is_empty() {
            let mut args_fields = String::new();
            for arg in &first_ix.args {
                let field_name = arg.name.to_snake_case();
                args_fields.push_str(&format!(
                    "    //     {}: todo!(), // TODO: Fill in actual value\n",
                    field_name
                ));
            }
            format!(
                "    // let args = {}IxArgs {{\n{}    // }};\n",
                ix_name_pascal, args_fields
            )
        } else {
            String::new()
        };

        let ix_name = &first_ix.name;
        format!(
            r#"//! Example: Building an instruction
//!
//! This example shows how to build a transaction instruction using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Build {1} instruction
    // In a real application, you would fill in the actual pubkeys
    // use {0}::*;
    // let keys = {2}Keys {{
{3}    // }};
{4}    // let instruction = {5}_ix(keys{6})?;
    // println!("Built instruction: {{:?}}", instruction);

    println!("Instruction building example for {1} - replace with actual values");

    Ok(())
}}
"#,
            module_name,
            ix_name,
            ix_name_pascal,
            keys_fields,
            args_init,
            ix_name_snake,
            if !first_ix.args.is_empty() {
                ", args"
            } else {
                ""
            }
        )
    } else {
        r#"//! Example: Building an instruction
//!
//! This example shows how to build a transaction instruction using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No instructions defined in IDL
    Ok(())
}
"#
        .to_string()
    };

    let build_ix_file = examples_dir.join("build_instruction.rs");
    fs::write(&build_ix_file, build_instruction_example).context(format!(
        "Failed to write build_instruction.rs: {:?}",
        build_ix_file
    ))?;

    // Example 2: Parsing an account
    let parse_account_example = if let Some(accounts) = &idl.accounts {
        if !accounts.is_empty() {
            let first_account = &accounts[0];
            let account_name = &first_account.name;
            format!(
                r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Example: Parse and validate {1} account
    // In a real application, you would get account_info from a transaction or RPC call
    // use {0}::*;
    // use solana_program::account_info::AccountInfo;
    // let account_info: &AccountInfo = /* ... */;
    // let account = {1}::try_from_account_info(account_info)?;
    // println!("Parsed account: {{:?}}", account);

    println!("Account parsing example for {1} - replace with actual AccountInfo");

    Ok(())
}}
"#,
                module_name, account_name
            )
        } else {
            r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No accounts defined in IDL
    Ok(())
}
"#
            .to_string()
        }
    } else {
        r#"//! Example: Parsing and validating an account
//!
//! This example shows how to parse and validate account data using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No accounts defined in IDL
    Ok(())
}
"#
        .to_string()
    };

    let parse_account_file = examples_dir.join("parse_account.rs");
    fs::write(&parse_account_file, parse_account_example).context(format!(
        "Failed to write parse_account.rs: {:?}",
        parse_account_file
    ))?;

    // Example 3: Parsing events
    let parse_events_example = if let Some(events) = &idl.events {
        if !events.is_empty() {
            let mut match_arms = String::new();
            for event in events.iter().take(3) {
                let variant_name = event.name.to_pascal_case();
                match_arms.push_str(&format!(
                    "    //     Ok(ParsedEvent::{}(e)) => {{\n    //         println!(\"Parsed {} event: {{:?}}\", e);\n    //     }}\n",
                    variant_name,
                    event.name
                ));
            }
            // Add catch-all for unhandled Ok variants (before Err arm)
            match_arms.push_str("    //     Ok(_) => println!(\"Parsed other event variant\"),\n");
            format!(
                r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Example: Parse a single event
    // In a real application, you would get event_data from transaction logs
    // use {0}::events::{{parse_event, ParsedEvent}};
    // let event_data: &[u8] = /* ... */;
    // match parse_event(event_data) {{
{1}    //     Err(e) => eprintln!("Failed to parse event: {{}}", e),
    // }}

    println!("Event parsing example - replace with actual event data");

    Ok(())
}}
"#,
                module_name, match_arms
            )
        } else {
            r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No events defined in IDL
    Ok(())
}
"#
            .to_string()
        }
    } else {
        r#"//! Example: Parsing events from transaction logs
//!
//! This example shows how to parse events from transaction data using the generated bindings.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No events defined in IDL
    Ok(())
}
"#
        .to_string()
    };

    let parse_events_file = examples_dir.join("parse_events.rs");
    fs::write(&parse_events_file, parse_events_example).context(format!(
        "Failed to write parse_events.rs: {:?}",
        parse_events_file
    ))?;

    Ok(())
}
