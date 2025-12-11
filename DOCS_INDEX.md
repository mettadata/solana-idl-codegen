# Documentation Index

Quick reference to all documentation files in this project.

## 📊 Performance & Testing Documentation

### **Start Here**: PERFORMANCE_REVIEW_SUMMARY.md
**What**: Complete summary of performance analysis and improvements  
**When to read**: First! Get the big picture of what was done  
**Time**: 5 minutes

### PERFORMANCE_ANALYSIS.md
**What**: Detailed performance breakdown and metrics  
**When to read**: When you need detailed performance data  
**Time**: 10 minutes

### TEST_PERFORMANCE.md
**What**: Quick reference for test metrics and commands  
**When to read**: When running tests, need quick stats  
**Time**: 3 minutes

### TESTING_AND_PERFORMANCE_SUMMARY.md
**What**: Overall testing infrastructure summary  
**When to read**: Understanding the complete test suite  
**Time**: 8 minutes

### BENCHMARKING.md
**What**: Complete guide to using Criterion benchmarks  
**When to read**: Setting up or running benchmarks  
**Time**: 15 minutes

## 🏗️ Project Documentation

### README.md
**What**: Main project documentation  
**When to read**: First time using the project  
**Sections**:
- Installation
- Usage
- Generated code structure
- Development
- Performance section (NEW)

### INTEGRATION_TESTING.md
**What**: Guide for writing integration tests  
**When to read**: Adding new integration tests  

### TEST_COVERAGE.md
**What**: Test coverage documentation  
**When to read**: Understanding what's tested  

### TEST_RESULTS.md
**What**: Latest test results  
**When to read**: Checking test status  

### TESTING_SUMMARY.md
**What**: Testing approach summary  
**When to read**: Overview of testing strategy  

## 🔧 Feature Documentation

### CODEGEN_FEATURES.md
**What**: Code generation features  
**When to read**: Understanding what can be generated  

### EVENT_WRAPPER_PATTERN.md
**What**: Event wrapper pattern documentation  
**When to read**: Working with events  

### OFF_CHAIN_FEATURES.md
**What**: Off-chain feature support  
**When to read**: Using generated code off-chain  

### POST_GENERATION_TESTS.md
**What**: Tests run after code generation  
**When to read**: Understanding post-gen validation  

## 📝 New Files Added (Performance Review)

1. **PERFORMANCE_REVIEW_SUMMARY.md** ⭐ Start here!
2. **PERFORMANCE_ANALYSIS.md** - Detailed metrics
3. **TEST_PERFORMANCE.md** - Quick reference
4. **TESTING_AND_PERFORMANCE_SUMMARY.md** - Overall summary
5. **BENCHMARKING.md** - Criterion guide
6. **DOCS_INDEX.md** - This file

## 🚀 Quick Command Reference

### Daily Development
```bash
cargo test --lib          # 0.01s - instant feedback
```

### Before Commit
```bash
just test-all            # ~90s - full validation
```

### Performance Tracking
```bash
just test-perf           # ~25s - performance metrics
```

### Deep Analysis
```bash
just bench               # 5-10min - statistical analysis
```

## 📂 Project Structure

```
solana-idl-codegen/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports (NEW)
│   ├── codegen.rs       # Code generation
│   └── idl.rs           # IDL parsing
├── tests/
│   ├── integration_tests.rs       # Integration tests (ENHANCED)
│   ├── generated_code_test.rs     # Generated code tests
│   └── performance_tests.rs       # Performance tests (NEW)
├── benches/
│   └── codegen_bench.rs           # Benchmarks (NEW)
├── idl/                           # Test IDL files
├── generated/                     # Generated code output
└── Documentation files (this index)
```

## 🎯 Documentation by Task

### "I want to run tests"
1. Start: TEST_PERFORMANCE.md (Quick command reference)
2. Details: INTEGRATION_TESTING.md

### "I want to measure performance"
1. Start: PERFORMANCE_REVIEW_SUMMARY.md (What was done)
2. Run: `just test-perf`
3. Deep dive: PERFORMANCE_ANALYSIS.md

### "I want to run benchmarks"
1. Read: BENCHMARKING.md
2. Run: `just bench`
3. View: `open target/criterion/report/index.html`

### "I want to understand test timing"
1. Start: TEST_PERFORMANCE.md
2. Run: `just test-with-timing`
3. Analyze: PERFORMANCE_ANALYSIS.md

### "I want to optimize something"
1. Read: PERFORMANCE_ANALYSIS.md (Section: Optimization Opportunities)
2. Benchmark: `just bench`
3. Profile: `cargo flamegraph`

### "I want to add a new test"
1. Unit test: Add to `src/codegen.rs` or `src/idl.rs`
2. Integration test: See INTEGRATION_TESTING.md
3. Performance test: See `tests/performance_tests.rs`

### "I want to understand the codebase"
1. Start: README.md
2. Features: CODEGEN_FEATURES.md
3. Testing: TESTING_AND_PERFORMANCE_SUMMARY.md

## 📊 Key Metrics Quick Reference

| Metric | Value | Status |
|--------|-------|--------|
| Unit Tests | 0.01s | ⚡ Lightning Fast |
| Code Generation | 42-114ms | ✅ Excellent |
| IDL Parsing | 0.9-3.4ms | 🚀 Outstanding |
| Integration Tests | ~61s | ⚠️ Acceptable |
| Test Coverage | 100 tests | ✅ Comprehensive |

## 🔗 External Resources

- [Criterion.rs Guide](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Benchmarking Best Practices](https://easyperf.net/blog/2018/08/26/Benchmarking-tips)

## 💡 Pro Tips

1. **Fast feedback loop**: Use `cargo test --lib` during development
2. **Check before commit**: Run `just test-all` to catch issues
3. **Track performance**: Run `just bench` weekly to monitor trends
4. **Profile bottlenecks**: Use `cargo flamegraph` for detailed analysis
5. **Read reports**: Criterion HTML reports are very informative

---

**Last Updated**: December 11, 2025  
**Version**: After performance review and benchmarking additions
