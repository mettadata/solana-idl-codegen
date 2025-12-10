use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

mod codegen;
mod idl;

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

    println!("Successfully parsed IDL for program: {}", idl.name);
    println!("Version: {}", idl.version);
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

    // Create output directory
    fs::create_dir_all(&cli.output).context(format!(
        "Failed to create output directory: {:?}",
        cli.output
    ))?;

    // Write generated code
    let output_file = cli.output.join(format!("{}.rs", cli.module));
    fs::write(&output_file, generated_code)
        .context(format!("Failed to write output file: {:?}", output_file))?;

    println!("\n✓ Generated code written to: {:?}", output_file);

    Ok(())
}
