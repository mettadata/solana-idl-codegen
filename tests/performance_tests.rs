//! Performance Tests for Code Generation
//!
//! These tests measure the performance of various code generation operations.

use solana_idl_codegen::{codegen, idl::Idl};
use std::fs;
use std::path::Path;
use std::time::Instant;

#[test]
fn test_idl_parsing_performance() {
    let test_cases = vec![
        ("pumpfun", "idl/pump-public-docs/idl/pump.json"),
        ("pumpfun_amm", "idl/pump-public-docs/idl/pump_amm.json"),
        ("raydium_amm", "idl/raydium-idl/raydium_amm/idl.json"),
        ("raydium_clmm", "idl/raydium-idl/raydium_clmm/amm_v3.json"),
        (
            "raydium_cpmm",
            "idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json",
        ),
    ];

    println!("\n=== IDL Parsing Performance ===");
    let mut total_time = std::time::Duration::ZERO;
    let mut timings = Vec::new();

    for (name, path) in &test_cases {
        if !Path::new(path).exists() {
            println!("⚠️  Skipping {} - file not found", name);
            continue;
        }

        let content = fs::read_to_string(path).unwrap();
        let file_size = content.len();

        let start = Instant::now();
        let iterations = 100;
        for _ in 0..iterations {
            let _idl: Idl = serde_json::from_str(&content).unwrap();
        }
        let duration = start.elapsed();
        let avg_time = duration / iterations;

        total_time += avg_time;
        timings.push((*name, avg_time, file_size));
        println!(
            "  {} - {:.2}ms avg (file size: {} KB)",
            name,
            avg_time.as_micros() as f64 / 1000.0,
            file_size / 1024
        );
    }

    println!(
        "Total avg parsing time: {:.2}ms",
        total_time.as_micros() as f64 / 1000.0
    );
    println!("===============================\n");
}

#[test]
fn test_code_generation_performance() {
    let test_cases = vec![
        ("pumpfun", "idl/pump-public-docs/idl/pump.json"),
        ("pumpfun_amm", "idl/pump-public-docs/idl/pump_amm.json"),
        ("raydium_amm", "idl/raydium-idl/raydium_amm/idl.json"),
        ("raydium_clmm", "idl/raydium-idl/raydium_clmm/amm_v3.json"),
        (
            "raydium_cpmm",
            "idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json",
        ),
    ];

    println!("\n=== Code Generation Performance ===");
    let mut total_time = std::time::Duration::ZERO;

    for (name, path) in &test_cases {
        if !Path::new(path).exists() {
            println!("⚠️  Skipping {} - file not found", name);
            continue;
        }

        let content = fs::read_to_string(path).unwrap();
        let idl: Idl = serde_json::from_str(&content).unwrap();

        // Measure full code generation (this is what the public API provides)
        let start = Instant::now();
        let _generated = codegen::generate(&idl, name).unwrap();
        let gen_time = start.elapsed();

        total_time += gen_time;

        println!("  {} - {:.2}ms", name, gen_time.as_micros() as f64 / 1000.0);
    }

    println!(
        "Total code generation time: {:.2}ms",
        total_time.as_micros() as f64 / 1000.0
    );
    println!("====================================\n");
}

#[test]
fn test_full_pipeline_performance() {
    let test_cases = vec![
        ("pumpfun", "idl/pump-public-docs/idl/pump.json"),
        ("pumpfun_amm", "idl/pump-public-docs/idl/pump_amm.json"),
        ("raydium_amm", "idl/raydium-idl/raydium_amm/idl.json"),
        ("raydium_clmm", "idl/raydium-idl/raydium_clmm/amm_v3.json"),
        (
            "raydium_cpmm",
            "idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json",
        ),
    ];

    println!("\n=== Full Pipeline Performance ===");
    let mut total_time = std::time::Duration::ZERO;

    for (name, path) in &test_cases {
        if !Path::new(path).exists() {
            println!("⚠️  Skipping {} - file not found", name);
            continue;
        }

        let start = Instant::now();

        // Read file
        let content = fs::read_to_string(path).unwrap();
        let read_time = start.elapsed();

        // Parse IDL
        let parse_start = Instant::now();
        let idl: Idl = serde_json::from_str(&content).unwrap();
        let parse_time = parse_start.elapsed();

        // Generate code
        let gen_start = Instant::now();
        let _generated = codegen::generate(&idl, name).unwrap();
        let gen_time = gen_start.elapsed();

        let pipeline_time = start.elapsed();
        total_time += pipeline_time;

        println!(
            "  {} - {:.2}ms total",
            name,
            pipeline_time.as_micros() as f64 / 1000.0
        );
        println!(
            "    read:       {:.2}ms",
            read_time.as_micros() as f64 / 1000.0
        );
        println!(
            "    parse:      {:.2}ms",
            parse_time.as_micros() as f64 / 1000.0
        );
        println!(
            "    generate:   {:.2}ms",
            gen_time.as_micros() as f64 / 1000.0
        );
    }

    println!(
        "Total pipeline time: {:.2}ms",
        total_time.as_micros() as f64 / 1000.0
    );
    println!("==================================\n");
}

#[test]
fn test_code_size_metrics() {
    let test_cases = vec![
        ("pumpfun", "idl/pump-public-docs/idl/pump.json"),
        ("pumpfun_amm", "idl/pump-public-docs/idl/pump_amm.json"),
        ("raydium_amm", "idl/raydium-idl/raydium_amm/idl.json"),
        ("raydium_clmm", "idl/raydium-idl/raydium_clmm/amm_v3.json"),
        (
            "raydium_cpmm",
            "idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json",
        ),
    ];

    println!("\n=== Generated Code Size Metrics ===");

    for (name, path) in &test_cases {
        if !Path::new(path).exists() {
            println!("⚠️  Skipping {} - file not found", name);
            continue;
        }

        let content = fs::read_to_string(path).unwrap();
        let idl: Idl = serde_json::from_str(&content).unwrap();
        let generated = codegen::generate(&idl, name).unwrap();

        let lib_lines = generated.lib.lines().count();
        let types_lines = generated.types.lines().count();
        let accounts_lines = generated.accounts.lines().count();
        let instructions_lines = generated.instructions.lines().count();
        let errors_lines = generated.errors.lines().count();
        let events_lines = generated.events.lines().count();
        let total_lines = lib_lines
            + types_lines
            + accounts_lines
            + instructions_lines
            + errors_lines
            + events_lines;

        let total_bytes = generated.lib.len()
            + generated.types.len()
            + generated.accounts.len()
            + generated.instructions.len()
            + generated.errors.len()
            + generated.events.len();

        println!(
            "  {} - {} lines, {} KB",
            name,
            total_lines,
            total_bytes / 1024
        );
        println!("    lib:          {} lines", lib_lines);
        println!("    types:        {} lines", types_lines);
        println!("    accounts:     {} lines", accounts_lines);
        println!("    instructions: {} lines", instructions_lines);
        println!("    errors:       {} lines", errors_lines);
        println!("    events:       {} lines", events_lines);
    }

    println!("=====================================\n");
}

#[test]
fn test_idl_complexity_metrics() {
    let test_cases = vec![
        ("pumpfun", "idl/pump-public-docs/idl/pump.json"),
        ("pumpfun_amm", "idl/pump-public-docs/idl/pump_amm.json"),
        ("raydium_amm", "idl/raydium-idl/raydium_amm/idl.json"),
        ("raydium_clmm", "idl/raydium-idl/raydium_clmm/amm_v3.json"),
        (
            "raydium_cpmm",
            "idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json",
        ),
    ];

    println!("\n=== IDL Complexity Metrics ===");

    for (name, path) in &test_cases {
        if !Path::new(path).exists() {
            println!("⚠️  Skipping {} - file not found", name);
            continue;
        }

        let content = fs::read_to_string(path).unwrap();
        let idl: Idl = serde_json::from_str(&content).unwrap();

        let num_instructions = idl.instructions.len();
        let num_accounts = idl.accounts.as_ref().map(|a| a.len()).unwrap_or(0);
        let num_types = idl.types.as_ref().map(|t| t.len()).unwrap_or(0);
        let num_errors = idl.errors.as_ref().map(|e| e.len()).unwrap_or(0);
        let num_events = idl.events.as_ref().map(|e| e.len()).unwrap_or(0);

        let total_complexity =
            num_instructions + num_accounts + num_types + num_errors + num_events;

        println!("  {} - complexity score: {}", name, total_complexity);
        println!("    instructions: {}", num_instructions);
        println!("    accounts:     {}", num_accounts);
        println!("    types:        {}", num_types);
        println!("    errors:       {}", num_errors);
        println!("    events:       {}", num_events);
    }

    println!("==============================\n");
}
