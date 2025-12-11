use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs;
use std::path::Path;

// Add the solana-idl-codegen modules
use solana_idl_codegen::idl::Idl;
use solana_idl_codegen::codegen;

fn load_test_idl(name: &str, path: &str) -> Idl {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read IDL: {}", path));
    serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse IDL: {}", name))
}

fn bench_idl_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("idl_parsing");
    
    let test_cases = vec![
        ("pumpfun", "idl/pump-public-docs/idl/pump.json"),
        ("pumpfun_amm", "idl/pump-public-docs/idl/pump_amm.json"),
        ("raydium_amm", "idl/raydium-idl/raydium_amm/idl.json"),
        ("raydium_clmm", "idl/raydium-idl/raydium_clmm/amm_v3.json"),
        ("raydium_cpmm", "idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json"),
    ];
    
    for (name, path) in test_cases.iter() {
        if !Path::new(path).exists() {
            continue;
        }
        
        let content = fs::read_to_string(path).unwrap();
        
        group.bench_with_input(BenchmarkId::from_parameter(name), &content, |b, content| {
            b.iter(|| {
                let idl: Idl = serde_json::from_str(black_box(content)).unwrap();
                black_box(idl)
            });
        });
    }
    
    group.finish();
}

fn bench_code_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("code_generation");
    group.sample_size(10); // Reduce samples since code generation is expensive
    
    let test_cases = vec![
        ("pumpfun", "idl/pump-public-docs/idl/pump.json"),
        ("pumpfun_amm", "idl/pump-public-docs/idl/pump_amm.json"),
        ("raydium_amm", "idl/raydium-idl/raydium_amm/idl.json"),
        ("raydium_clmm", "idl/raydium-idl/raydium_clmm/amm_v3.json"),
        ("raydium_cpmm", "idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json"),
    ];
    
    for (name, path) in test_cases.iter() {
        if !Path::new(path).exists() {
            continue;
        }
        
        let idl = load_test_idl(name, path);
        
        group.bench_with_input(BenchmarkId::from_parameter(name), &idl, |b, idl| {
            b.iter(|| {
                let generated = codegen::generate(black_box(idl), name).unwrap();
                black_box(generated)
            });
        });
    }
    
    group.finish();
}

// Type mapping is an internal function, so we'll skip this benchmark

// Individual component benchmarks skipped as they are internal functions
// The full code_generation benchmark covers the entire pipeline

criterion_group!(
    benches,
    bench_idl_parsing,
    bench_code_generation
);
criterion_main!(benches);
