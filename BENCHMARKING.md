# Benchmarking Guide

This project uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for statistical benchmarking of code generation performance.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Or use the justfile command
just bench

# Run specific benchmark group
cargo bench --bench codegen_bench -- idl_parsing
cargo bench --bench codegen_bench -- code_generation
```

## Benchmark Groups

### 1. IDL Parsing
Measures the performance of deserializing IDL JSON files into Rust structs.

**Test Cases**:
- pumpfun (119 KB)
- pumpfun_amm (108 KB)
- raydium_amm (46 KB)
- raydium_clmm (131 KB)
- raydium_cpmm (47 KB)

### 2. Code Generation
Measures the performance of generating Rust code from parsed IDL structures.

**Test Cases**: Same as above

## Benchmark Output

Criterion provides:
- **Statistical Analysis**: Mean, median, standard deviation
- **Outlier Detection**: Identifies abnormal measurements
- **Regression Detection**: Compares with previous runs
- **HTML Reports**: Visual graphs and detailed analysis

### Example Output

```
idl_parsing/pumpfun    time:   [2.80 ms 2.85 ms 2.90 ms]
                       change: [-2.5% +0.1% +2.7%] (p = 0.07 > 0.05)
                       No change in performance detected.

code_generation/pumpfun time:   [98.5 ms 101.2 ms 104.1 ms]
                        change: [-1.2% +1.5% +4.3%] (p = 0.15 > 0.05)
                        No change in performance detected.
```

## Reports Location

After running benchmarks, reports are saved in:
```
target/criterion/
├── idl_parsing/
│   ├── pumpfun/
│   │   ├── report/
│   │   │   └── index.html  ← Open this in browser
│   │   └── ...
│   └── ...
└── code_generation/
    └── ...
```

## Viewing HTML Reports

```bash
# Open the main report index
open target/criterion/report/index.html

# Or for a specific benchmark
open target/criterion/idl_parsing/pumpfun/report/index.html
```

## Baseline Comparison

Criterion can save and compare against baselines:

```bash
# Save current results as baseline
cargo bench -- --save-baseline my-baseline

# Compare against baseline
cargo bench -- --baseline my-baseline
```

This is useful for:
- Testing optimizations
- Detecting performance regressions
- A/B testing different implementations

## Performance Goals

### IDL Parsing
- **Target**: < 5ms per file
- **Current**: 0.9-3.4ms ✅
- **Status**: Exceeds target

### Code Generation
- **Target**: < 150ms per program
- **Current**: 42-114ms ✅
- **Status**: Exceeds target

## Interpreting Results

### What to Look For

1. **Mean Time**: Average execution time
2. **Standard Deviation**: Consistency of measurements
3. **Outliers**: Unusual measurements (investigate if many)
4. **Throughput**: Iterations per second
5. **Change**: Comparison with previous run

### Red Flags

- ⚠️ High standard deviation (> 10%)
- ⚠️ Many outliers (> 5%)
- 🚨 Significant regression (> 10% slower)
- 🚨 Increasing trend over time

### Good Signs

- ✅ Low standard deviation (< 5%)
- ✅ Few outliers
- ✅ Stable or improving performance
- ✅ Consistent results across runs

## Advanced Usage

### Profiling with Flamegraph

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bench codegen_bench

# Output: flamegraph.svg
```

### Custom Sample Size

```rust
// In benches/codegen_bench.rs
group.sample_size(100);  // Default: 100
group.measurement_time(Duration::from_secs(10));  // Default: 5s
```

### Warming Up

Criterion automatically warms up by running the benchmark multiple times before measuring.

```rust
group.warm_up_time(Duration::from_secs(3));  // Default: 3s
```

## CI/CD Integration

For continuous integration:

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench --no-fail-fast

- name: Store benchmark results
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: target/criterion/baseline-comparison.json
```

## Comparing Different Approaches

To compare two implementations:

1. Save current as baseline:
   ```bash
   cargo bench -- --save-baseline before
   ```

2. Make your changes

3. Compare:
   ```bash
   cargo bench -- --baseline before
   ```

4. Review the change percentages in the output

## Troubleshooting

### Inconsistent Results

If you get high variance:
- Close other applications
- Disable CPU frequency scaling
- Run multiple times and average
- Increase sample size

### Benchmarks Take Too Long

```rust
// Reduce samples for slower benchmarks
group.sample_size(10);  // Default: 100
group.measurement_time(Duration::from_secs(5));  // Default: 5s
```

### Out of Disk Space

Criterion stores historical data. Clean up with:
```bash
rm -rf target/criterion
```

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Benchmarking Best Practices](https://easyperf.net/blog/2018/08/26/Benchmarking-tips)

---

*Remember: Benchmarking shows you where you are, not where you need to be. Optimize based on real-world usage, not just benchmark numbers.*
