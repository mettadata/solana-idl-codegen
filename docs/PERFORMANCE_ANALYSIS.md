# Performance Analysis & Benchmarking

This document provides a comprehensive analysis of test execution times and code generation performance.

## Test Execution Summary

### Overview
- **Total test suite execution**: ~85-90 seconds (clean build)
- **Unit tests (84 tests)**: 0.01s ⚡
- **Integration tests (11 tests)**: ~61s 
- **Performance tests (5 tests)**: ~1s

### Detailed Breakdown

#### 1. Build Times
- **Clean build**: 6-7 seconds (main binary)
- **Test dependencies build**: 19-20 seconds (first time)
- **Incremental builds**: < 1 second

#### 2. Code Generation Performance
Individual IDL processing times:

| Program | Parse Time | Generation Time | Total | Lines Generated | Size |
|---------|-----------|----------------|-------|----------------|------|
| pumpfun | 2.90ms | 99.11ms | 102.21ms | 4,503 lines | 152 KB |
| pumpfun_amm | 2.74ms | 103.51ms | 106.29ms | 4,724 lines | 160 KB |
| raydium_amm | 1.15ms | 56.56ms | 57.74ms | 2,951 lines | 95 KB |
| raydium_clmm | 2.83ms | 110.90ms | 113.83ms | 5,172 lines | 180 KB |
| raydium_cpmm | 1.15ms | 41.34ms | 42.54ms | 2,032 lines | 67 KB |

**Total code generation time**: ~422ms for all 5 programs

**Key Insights**:
- IDL parsing is very fast (< 3ms per file)
- Code generation scales with complexity:
  - Simple programs (raydium_cpmm): ~42ms
  - Complex programs (raydium_clmm): ~114ms
- Average: ~84ms per program

#### 3. Generated Crate Compilation Times

This is the main bottleneck in the test suite:

| Crate | Compilation Time |
|-------|-----------------|
| pumpfun | 13.26s |
| pumpfun_amm | 13.47s |
| raydium_amm | 10.21s |
| raydium_clmm | 12.99s |
| raydium_cpmm | 11.49s |

**Total**: 61.42s

**Analysis**:
- Each crate takes 10-13 seconds to compile (cargo check)
- This is expected for Solana programs with heavy dependencies
- Crates are compiled sequentially in tests
- Most time spent in dependency resolution and compilation

#### 4. IDL Parsing Performance (100 iterations avg)

| Program | Avg Parse Time | File Size |
|---------|---------------|-----------|
| pumpfun | 3.43ms | 119 KB |
| pumpfun_amm | 2.71ms | 108 KB |
| raydium_amm | 0.97ms | 46 KB |
| raydium_clmm | 2.52ms | 131 KB |
| raydium_cpmm | 0.92ms | 47 KB |

**Total avg**: 10.54ms

#### 5. IDL Complexity Metrics

| Program | Complexity Score | Instructions | Accounts | Types | Errors | Events |
|---------|-----------------|--------------|----------|-------|--------|--------|
| pumpfun | 121 | 23 | 5 | 26 | 49 | 18 |
| pumpfun_amm | 121 | 21 | 6 | 29 | 45 | 20 |
| raydium_amm | 85 | 16 | 3 | 9 | 57 | 0 |
| raydium_clmm | 115 | 25 | 9 | 25 | 45 | 11 |
| raydium_cpmm | 32 | 10 | 3 | 6 | 11 | 2 |

*Complexity score = instructions + accounts + types + errors + events*

## Performance Optimization Opportunities

### 1. Test Parallelization
**Current**: Crates compile sequentially  
**Potential**: Parallel compilation could reduce from 61s to ~15s  
**Status**: Not implemented (trade-off: resource usage)

### 2. Incremental Testing
**Current**: Full clean + rebuild on every test run  
**Potential**: Keep generated crates cached between runs  
**Savings**: ~20s per test run after first execution

### 3. Code Generation Optimization
**Current**: ~400ms for all 5 programs  
**Status**: Already quite fast, not a bottleneck

### 4. Selective Testing
**Current**: Always test all 5 generated crates  
**Potential**: Test only changed crates  
**Use case**: Development workflow

## Benchmarking

Run comprehensive benchmarks with:

```bash
just bench
```

This uses [Criterion.rs](https://github.com/bheisler/criterion.rs) to provide:
- Statistical analysis of performance
- Comparison with previous runs
- HTML reports with graphs
- Regression detection

### Benchmark Groups

1. **IDL Parsing** - Measures JSON deserialization performance
2. **Code Generation** - Measures TokenStream generation performance

## Running Performance Tests

```bash
# Run all performance tests
just test-perf

# Run with timing information
just test-with-timing

# Run benchmarks
just bench
```

## CI/CD Considerations

For continuous integration:

1. **Cache Strategy**:
   - Cache Cargo build directory
   - Cache generated crates between test phases
   - Expected speedup: 50-70%

2. **Parallel Jobs**:
   - Run unit tests and integration tests in parallel
   - Generate all crates before testing
   - Expected speedup: 30-40%

3. **Selective Testing**:
   - Skip compilation tests on docs-only changes
   - Use cargo nextest for faster test execution

## Performance Regression Detection

The benchmark suite (`cargo bench`) maintains historical data to detect:
- Code generation slowdowns
- IDL parsing regressions
- Memory usage increases

Reports are saved in `target/criterion/` with HTML visualizations.

## Summary

### Fast ✅
- Unit tests (0.01s)
- Code generation (0.4s)
- IDL parsing (0.01s)
- Performance tests (1s)

### Acceptable ⚠️
- Build times (6-20s depending on cache)
- Integration tests (61s)

### Optimization Available 🔧
- Parallel crate compilation could reduce integration tests to ~15s
- Caching could reduce repeated test runs significantly

---

*Last updated: Based on test run with 5 generated programs*
*Hardware: Results may vary based on CPU, disk speed, and available memory*
