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

    println!("\n✓ Generated crate at: {:?}", crate_dir);
    println!("  ├── Cargo.toml");
    println!("  ├── README.md");
    println!("  ├── .gitignore");
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
