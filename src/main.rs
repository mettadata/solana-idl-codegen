use anyhow::{Context, Result};
use clap::Parser;
use heck::{ToPascalCase, ToSnakeCase};
use std::fs;
use std::path::{Path, PathBuf};

use solana_idl_codegen::{codegen, idl, manifest, r#override, registry, workspace};

#[derive(Parser)]
#[command(name = "solana-idl-codegen")]
#[command(about = "Generate Rust code bindings from Solana IDL files", long_about = None)]
struct Cli {
    /// Path to the IDL JSON file (single-program mode)
    #[arg(short, long, value_name = "FILE", required_unless_present = "manifest")]
    input: Option<PathBuf>,

    /// Output directory for generated code (single-program mode)
    #[arg(short, long, value_name = "DIR", default_value = "generated")]
    output: PathBuf,

    /// Module name for generated code (single-program mode)
    #[arg(short, long, default_value = "program")]
    module: String,

    /// Path to override file (optional, single-program mode)
    #[arg(long, value_name = "FILE")]
    override_file: Option<PathBuf>,

    /// Path to manifest file for batch mode (processes all programs)
    #[arg(long, value_name = "FILE", conflicts_with_all = ["input", "override_file"])]
    manifest: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Dispatch: manifest batch mode vs single-program mode
    if let Some(manifest_path) = &cli.manifest {
        return run_manifest_mode(manifest_path);
    }

    // Single-program mode — input is required (enforced by clap)
    let input = cli
        .input
        .as_ref()
        .expect("input required in single-program mode");

    // Read and parse IDL file
    let idl_content =
        fs::read_to_string(input).context(format!("Failed to read IDL file: {:?}", input))?;

    let mut idl: idl::Idl =
        serde_json::from_str(&idl_content).context("Failed to parse IDL JSON")?;

    // T027: Discover and apply override file if present
    // Use module name for override discovery (more reliable than IDL filename)
    let override_discovery =
        r#override::discover_override_file(input, &cli.module, cli.override_file.as_deref())
            .context("Failed to discover override file")?;

    match override_discovery {
        r#override::OverrideDiscovery::Found(override_path) => {
            println!("Found override file: {}", override_path.display());

            // T024: Load override file
            let override_file = r#override::load_override_file(&override_path)
                .context("Failed to load override file")?;

            // T025: Validate override file
            r#override::validate_override_file(&override_file, &idl)
                .context("Override file validation failed")?;

            // T026: Apply overrides to IDL
            let (modified_idl, applied_overrides) =
                r#override::apply_overrides(idl, &override_file)
                    .context("Failed to apply overrides to IDL")?;
            idl = modified_idl;

            // T028: Log applied overrides
            if !applied_overrides.is_empty() {
                println!("Applied {} override(s):", applied_overrides.len());
                for applied in &applied_overrides {
                    match applied.override_type {
                        r#override::OverrideType::ProgramAddress => {
                            println!(
                                "  ⚠ Program address: {} → {}",
                                applied.original_value.as_deref().unwrap_or("(none)"),
                                applied.override_value
                            );
                        }
                        r#override::OverrideType::AccountDiscriminator => {
                            println!(
                                "  Account '{}': discriminator overridden",
                                applied.entity_name.as_deref().unwrap_or("unknown")
                            );
                        }
                        r#override::OverrideType::EventDiscriminator => {
                            println!(
                                "  Event '{}': discriminator overridden",
                                applied.entity_name.as_deref().unwrap_or("unknown")
                            );
                        }
                        r#override::OverrideType::InstructionDiscriminator => {
                            println!(
                                "  Instruction '{}': discriminator overridden",
                                applied.entity_name.as_deref().unwrap_or("unknown")
                            );
                        }
                    }
                }
            }
        }
        r#override::OverrideDiscovery::NotFound => {
            // No override file - continue with original IDL
        }
        r#override::OverrideDiscovery::Conflict { files, sources } => {
            eprintln!("ERROR: Multiple override files detected:");
            for (file, source) in files.iter().zip(sources.iter()) {
                eprintln!("  - {} ({})", file.display(), source);
            }
            eprintln!("\nOverride file priority order:");
            eprintln!("  1. Explicit --override-file flag (highest priority)");
            eprintln!("  2. Convention-based: ./overrides/{{idl_name}}.json");
            eprintln!("  3. Global fallback: ./idl-overrides.json (lowest priority)");
            eprintln!("\nTo resolve this conflict:");
            eprintln!("  - Remove one of the conflicting files, OR");
            eprintln!("  - Use --override-file <path> to explicitly choose which to apply");
            anyhow::bail!("Multiple override files detected");
        }
    }

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

    // Write serializable.rs (serializable event types with String pubkeys)
    if !generated_code.serializable.is_empty() {
        let serializable_file = src_dir.join("serializable.rs");
        fs::write(&serializable_file, &generated_code.serializable).context(format!(
            "Failed to write serializable.rs: {:?}",
            serializable_file
        ))?;
    } else {
        let serializable_file = src_dir.join("serializable.rs");
        fs::write(
            &serializable_file,
            "// No serializable event types needed\n",
        )
        .context(format!(
            "Failed to write serializable.rs: {:?}",
            serializable_file
        ))?;
    }

    // Write decoder.rs (discriminator-based event decoder)
    if !generated_code.decoder.is_empty() {
        let decoder_file = src_dir.join("decoder.rs");
        fs::write(&decoder_file, &generated_code.decoder)
            .context(format!("Failed to write decoder.rs: {:?}", decoder_file))?;
    }

    // Write deref_impls (appended to events.rs or as separate module — for now included in decoder.rs)

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
    if !generated_code.serializable.is_empty() {
        rustfmt_files.push(src_dir.join("serializable.rs"));
    }
    if !generated_code.decoder.is_empty() {
        rustfmt_files.push(src_dir.join("decoder.rs"));
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
    println!("      ├── events.rs");
    println!("      └── serializable.rs");

    Ok(())
}

/// Run codegen in batch mode using a manifest file.
///
/// Processes all programs listed in the manifest:
/// 1. Load and validate the manifest
/// 2. For each program: load IDL, apply overrides, generate code, write output
fn run_manifest_mode(manifest_path: &Path) -> Result<()> {
    let manifest_dir = manifest_path
        .parent()
        .context("Manifest path has no parent directory")?;

    // Load and validate manifest
    let mf = manifest::load_manifest(manifest_path)?;
    manifest::validate_manifest(&mf, manifest_dir)?;

    println!(
        "Processing {} program(s) from manifest: {}",
        mf.programs.len(),
        manifest_path.display()
    );

    let output_dir = manifest_dir.join(&mf.output_dir);

    // First pass: generate per-program crates and collect event info for registry
    let mut program_infos = Vec::new();

    for entry in &mf.programs {
        println!("\n--- Generating: {} ---", entry.name);

        // Resolve paths
        let idl_path = manifest::resolve_idl_path(entry, manifest_dir);
        let override_path = manifest::resolve_override_path(entry, manifest_dir);

        // Read and parse IDL
        let idl_content = fs::read_to_string(&idl_path)
            .with_context(|| format!("Failed to read IDL file: {}", idl_path.display()))?;
        let mut idl: idl::Idl =
            serde_json::from_str(&idl_content).context("Failed to parse IDL JSON")?;

        // Apply overrides if present
        if let Some(override_path) = &override_path {
            let override_file = r#override::load_override_file(override_path)
                .context("Failed to load override file")?;
            r#override::validate_override_file(&override_file, &idl)
                .context("Override file validation failed")?;
            let (modified_idl, applied) = r#override::apply_overrides(idl, &override_file)
                .context("Failed to apply overrides")?;
            idl = modified_idl;
            if !applied.is_empty() {
                println!("  Applied {} override(s)", applied.len());
            }
        }

        println!(
            "  Program: {} | Version: {} | Instructions: {}",
            idl.get_name(),
            idl.get_version(),
            idl.instructions.len()
        );

        // Collect event info for registry (before generating)
        let event_info = registry::collect_program_event_info(entry, &idl);
        program_infos.push(event_info);

        // Generate code
        let generated_code = codegen::generate(&idl, &entry.name)?;

        // Write generated crate
        write_generated_crate(&output_dir, &entry.name, &idl, &generated_code)?;

        println!(
            "  ✓ Generated crate at: {}",
            output_dir.join(&entry.name).display()
        );
    }

    // Second pass: generate the cross-program registry crate
    println!("\n--- Generating registry: {} ---", mf.registry_crate);
    registry::generate_registry_crate(&output_dir, &mf.registry_crate, &program_infos)?;
    println!(
        "  ✓ Generated registry at: {}",
        output_dir.join(&mf.registry_crate).display()
    );

    // Third pass: workspace auto-wiring (if configured)
    if let Some(ref ws_toml) = mf.workspace_cargo_toml {
        let root_cargo_toml = manifest_dir.join(ws_toml);
        println!("\n--- Workspace wiring ---");

        // Build (name, relative_path) pairs using output_dir relative to workspace root
        let ws_root = root_cargo_toml
            .parent()
            .context("workspace Cargo.toml has no parent directory")?;
        let abs_output = fs::canonicalize(&output_dir).unwrap_or_else(|_| output_dir.clone());

        let programs: Vec<(String, String)> = mf
            .programs
            .iter()
            .map(|entry| {
                let crate_abs = abs_output.join(&entry.name);
                let rel = pathdiff::diff_paths(&crate_abs, ws_root)
                    .unwrap_or_else(|| crate_abs.clone())
                    .to_string_lossy()
                    .to_string();
                (entry.name.clone(), rel)
            })
            .collect();

        let registry_abs = abs_output.join(&mf.registry_crate);
        let registry_rel = pathdiff::diff_paths(&registry_abs, ws_root)
            .unwrap_or_else(|| registry_abs.clone())
            .to_string_lossy()
            .to_string();

        // Resolve downstream Cargo.toml paths
        let downstream: Vec<PathBuf> = mf
            .downstream_cargo_tomls
            .as_ref()
            .map(|paths| paths.iter().map(|p| manifest_dir.join(p)).collect())
            .unwrap_or_default();

        let downstream_ref: Option<&Path> = downstream.first().map(|p| p.as_path());

        workspace::wire_workspace(
            &root_cargo_toml,
            &programs,
            &mf.registry_crate,
            &registry_rel,
            downstream_ref,
        )?;

        // Wire each program crate as downstream dependency too
        for downstream_path in &downstream {
            for entry in &mf.programs {
                let changed = workspace::ensure_dependency(downstream_path, &entry.name)?;
                if changed {
                    eprintln!(
                        "  + Added {} dependency to {}",
                        entry.name,
                        downstream_path.display()
                    );
                }
            }
        }

        println!("  ✓ Workspace wiring complete");
    }

    println!(
        "\n✓ All {} program(s) + registry generated successfully.",
        mf.programs.len()
    );

    Ok(())
}

/// Write a generated crate's files to the output directory.
fn write_generated_crate(
    output_dir: &Path,
    module_name: &str,
    idl: &idl::Idl,
    generated_code: &codegen::GeneratedCode,
) -> Result<()> {
    let crate_dir = output_dir.join(module_name);
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).context(format!(
        "Failed to create crate source directory: {:?}",
        src_dir
    ))?;

    // Write lib.rs
    fs::write(src_dir.join("lib.rs"), &generated_code.lib).context("Failed to write lib.rs")?;

    // Write types.rs
    let types_content = if generated_code.types.is_empty() {
        "// No custom types defined\n"
    } else {
        &generated_code.types
    };
    fs::write(src_dir.join("types.rs"), types_content).context("Failed to write types.rs")?;

    // Write accounts.rs
    let accounts_content = if generated_code.accounts.is_empty() {
        "// No accounts defined\n"
    } else {
        &generated_code.accounts
    };
    fs::write(src_dir.join("accounts.rs"), accounts_content)
        .context("Failed to write accounts.rs")?;

    // Write instructions.rs
    fs::write(
        src_dir.join("instructions.rs"),
        &generated_code.instructions,
    )
    .context("Failed to write instructions.rs")?;

    // Write errors.rs
    let errors_content = if generated_code.errors.is_empty() {
        "// No errors defined\n"
    } else {
        &generated_code.errors
    };
    fs::write(src_dir.join("errors.rs"), errors_content).context("Failed to write errors.rs")?;

    // Write events.rs
    let events_content = if generated_code.events.is_empty() {
        "// No events defined\n"
    } else {
        &generated_code.events
    };
    fs::write(src_dir.join("events.rs"), events_content).context("Failed to write events.rs")?;

    // Write serializable.rs
    let serializable_content = if generated_code.serializable.is_empty() {
        "// No serializable event types needed\n"
    } else {
        &generated_code.serializable
    };
    fs::write(src_dir.join("serializable.rs"), serializable_content)
        .context("Failed to write serializable.rs")?;

    // Write decoder.rs (discriminator-based event decoder)
    if !generated_code.decoder.is_empty() {
        fs::write(src_dir.join("decoder.rs"), &generated_code.decoder)
            .context("Failed to write decoder.rs")?;
    }

    // Generate Cargo.toml
    let cargo_toml = generate_cargo_toml(module_name, idl);
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml).context("Failed to write Cargo.toml")?;

    // Generate README.md
    let readme = generate_readme(module_name, idl);
    fs::write(crate_dir.join("README.md"), readme).context("Failed to write README.md")?;

    // Generate .gitignore
    fs::write(crate_dir.join(".gitignore"), "/target\n/Cargo.lock\n")
        .context("Failed to write .gitignore")?;

    // Generate examples
    let examples_dir = crate_dir.join("examples");
    fs::create_dir_all(&examples_dir).context("Failed to create examples directory")?;
    generate_examples(&examples_dir, module_name, idl)?;

    // Format generated code with rustfmt
    let mut rustfmt_files = vec![src_dir.join("lib.rs"), src_dir.join("instructions.rs")];
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
    if !generated_code.serializable.is_empty() {
        rustfmt_files.push(src_dir.join("serializable.rs"));
    }
    if !generated_code.decoder.is_empty() {
        rustfmt_files.push(src_dir.join("decoder.rs"));
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
solana-program = "3.0"
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

    println!("Example: building {1} instruction");

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
