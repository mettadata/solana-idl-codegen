# Test Performance Quick Reference

## Quick Commands

```bash
# Run all tests with timing
just test-with-timing

# Run only performance tests
just test-perf

# Run benchmarks (requires criterion)
just bench

# Run integration tests only
just test-integration
```

## Test Timing Breakdown (Latest Run)

### Unit Tests
- **Count**: 84 tests
- **Time**: 0.01s
- **Status**: ⚡ Extremely fast

### Integration Tests
- **Count**: 11 tests
- **Time**: ~61s
- **Bottleneck**: `test_generated_crates_compile` (~61s)
  - pumpfun: 13.26s
  - pumpfun_amm: 13.47s
  - raydium_amm: 10.21s
  - raydium_clmm: 12.99s
  - raydium_cpmm: 11.49s

### Performance Tests
- **Count**: 5 tests
- **Time**: ~1s
- **Coverage**:
  - IDL Parsing Performance
  - Code Generation Performance
  - Full Pipeline Performance
  - Code Size Metrics
  - IDL Complexity Metrics

### Benchmarks (with Criterion)
- **Groups**: 2 benchmark groups
- **Time**: ~5-10 minutes (statistical analysis)
- **Output**: `target/criterion/` (HTML reports)

## Performance Metrics

### Code Generation Speed
- **Average**: ~84ms per program
- **Range**: 42ms (simple) to 114ms (complex)
- **Total**: ~422ms for all 5 programs

### IDL Parsing Speed
- **Average**: 2.1ms per file
- **Total**: ~10.5ms for all 5 files (100 iterations avg)

### Generated Code Size
| Program | Lines | Size |
|---------|-------|------|
| pumpfun | 4,503 | 152 KB |
| pumpfun_amm | 4,724 | 160 KB |
| raydium_amm | 2,951 | 95 KB |
| raydium_clmm | 5,172 | 180 KB |
| raydium_cpmm | 2,032 | 67 KB |

## Optimization Status

✅ **Already Optimized**:
- Unit tests (0.01s)
- Code generation (< 0.5s)
- IDL parsing (< 0.02s)

⚠️ **Could Be Improved**:
- Integration test compilation (61s → could be ~15s with parallelization)
- Clean build times (varies based on cache)

## CI/CD Recommendations

1. **Cache Strategy**:
   ```yaml
   cache:
     - target/
     - ~/.cargo/
     - generated/
   ```

2. **Parallel Testing**:
   - Run unit tests + performance tests in parallel
   - Compile generated crates in parallel
   - Expected speedup: 40-50%

3. **Selective Testing**:
   - Skip compilation tests on documentation changes
   - Run benchmarks only on performance-related changes

## Profiling

To profile code generation:

```bash
# Install flamegraph
cargo install flamegraph

# Profile code generation
cargo flamegraph --bin solana-idl-codegen -- -i idl/pump-public-docs/idl/pump.json -o /tmp/test -m test

# View flamegraph.svg in browser
```

## Memory Usage

Typical memory usage during code generation:
- **Peak**: ~50-100 MB per program
- **Average**: ~30 MB
- **Total for 5 programs**: < 200 MB

Very memory efficient! ✅

---

For detailed performance analysis, see [PERFORMANCE_ANALYSIS.md](./PERFORMANCE_ANALYSIS.md)
