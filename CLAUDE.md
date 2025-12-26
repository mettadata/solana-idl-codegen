# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust CLI tool that generates strongly-typed Rust code bindings from Solana IDL (Interface Description Language) files. Parses JSON IDL files and produces idiomatic Rust code with proper naming conventions, Borsh/Bytemuck serialization support, and type-safe Pubkey handling.

## Development Commands

### Build and Run

```bash
cargo build --release           # Build production binary (target/release/solana-idl-codegen)
cargo run -- -i idl.json -o generated -m module_name  # Run directly

# Use justfile commands instead (preferred):
just generate                   # Generate all configured IDL bindings
just clean                      # Remove generated/ directory and cargo artifacts
```

### Code Quality

**CRITICAL**: All code must pass formatting and linting checks:

```bash
just fmt-check                  # Verify formatting (read-only)
just fmt                        # Apply auto-formatting
just clippy                     # Run linting with warnings-as-errors
```

These are enforced in CI and must pass before merging.

### Testing

```bash
# Unit tests (84 tests, ~0.01s)
cargo test
just test

# Integration tests (11 tests, ~61s - compiles all generated crates)
just test-integration

# All tests including integration
just test-all

# Performance tests (5 tests, ~1s)
just test-perf

# Run with timing details
just test-with-timing
```

**Note**: Integration tests compile all generated crates to verify they build successfully. This takes ~61 seconds.

### Generated Code Quality Gates

After code generation, verify generated crates compile and pass linting:

```bash
just check-generated            # Verify all generated crates compile
just build-generated            # Build all generated crates
just lint-generated             # Run fmt + clippy on generated code
just check-all                  # Complete quality gate (codegen + generated)
```

### Benchmarking

```bash
just bench                      # Run Criterion benchmarks with HTML reports
```

Performance targets:
- Simple programs: ~42ms generation time
- Complex programs: ~114ms generation time
- Average: ~84ms per program

## Architecture

### Core Components

**src/main.rs**: CLI entry point using clap
- Parses IDL JSON files
- Orchestrates code generation
- Creates crate structure with Cargo.toml and organized modules

**src/idl.rs**: IDL data structures
- Supports both old and new IDL formats via optional fields
- Handles metadata, instructions, accounts, types, errors, events, constants
- Provides `get_name()`, `get_version()`, `get_address()` helpers for format compatibility

**src/codegen.rs**: Code generation engine
- Generates 6 modules: lib.rs, types.rs, accounts.rs, instructions.rs, errors.rs, events.rs
- Uses `proc-macro2`, `quote`, and `prettyplease` for token-based code generation
- Handles discriminators, Borsh/Bytemuck serialization, serde support
- Implements type mapping (IDL primitives → Rust types)

### Code Generation Flow

```
IDL JSON → Parse (idl.rs) → Generate TokenStreams (codegen.rs) → Format (prettyplease) → Write Files
```

Generated crate structure:
```
generated/<module_name>/
├── Cargo.toml              # Auto-generated with dependencies
├── src/
│   ├── lib.rs              # Module re-exports
│   ├── types.rs            # Custom type definitions
│   ├── accounts.rs         # Account structs with discriminators
│   ├── instructions.rs     # Instruction enum + args/accounts
│   ├── errors.rs           # Error enum with codes
│   └── events.rs           # Event structs
```

### IDL Format Compatibility

The codegen handles two IDL formats:

**Old format**: Top-level `name` and `version` fields, inline account type definitions
**New format**: `metadata` object, accounts reference types separately, discriminators

Key compatibility patterns in `idl.rs`:
- `#[serde(default)]` on all optional fields
- Helper methods (`get_name()`, `get_version()`, `get_address()`) check both locations
- Account discriminators stored separately and applied to matching types

### Type Mappings

| IDL Type | Rust Type |
|----------|-----------|
| `bool`, `u8`-`u128`, `i8`-`i128` | Same primitive |
| `string` | `String` |
| `publicKey` | `Pubkey` |
| `bytes` | `Vec<u8>` |
| `{vec: T}` | `Vec<T>` |
| `{option: T}` | `Option<T>` |
| `{array: [T, N]}` | `[T; N]` |
| `{defined: "Custom"}` | `Custom` |

### Serialization Support

**Borsh** (default): `BorshSerialize`, `BorshDeserialize` derives
**Bytemuck** (when `serialization: "bytemuckunsafe"` or `"bytemuck"`):
- `Pod`, `Zeroable`, `Copy`, `Clone` derives
- `#[repr(C)]` for memory layout
- Zero-copy deserialization methods

**Serde** (for Pubkey): Custom serializer/deserializer for base58 string representation

### Discriminators

**Account discriminators**: 8-byte arrays for account type identification
- Generated as `const DISCRIMINATOR: [u8; 8]`
- Provides `try_from_slice_with_discriminator()` and `serialize_with_discriminator()`
- Handles both Borsh and Bytemuck types differently

**Event discriminators**: Same pattern for event parsing from logs

## Justfile Configuration

The `justfile` defines module→IDL mappings:

```just
# List of generated crates
projects := "raydium_amm raydium_clmm raydium_cpmm pumpfun pumpfun_amm"

# IDL configurations: module_name:idl_path
idls := "raydium_amm:idl/raydium-idl/raydium_amm/idl.json ..."
```

Add new IDL bindings by:
1. Adding to `idls` variable
2. Adding module name to `projects` variable
3. Running `just generate`

## Dependencies

**Build dependencies** (Cargo.toml):
- `serde`, `serde_json` - IDL parsing
- `clap` - CLI argument parsing
- `anyhow` - Error handling
- `quote`, `proc-macro2`, `syn` - Token-based code generation
- `heck` - Case conversion (snake_case ↔ PascalCase)
- `prettyplease` - Rust code formatting

**Generated code dependencies** (added to each generated Cargo.toml):
- `borsh 1.5` - Serialization
- `bytemuck 1.14` - Zero-copy types
- `solana-program 1.18` - Pubkey and Solana types
- `serde 1.0` - Serde for Pubkey serialization

## Testing Strategy

**Unit tests** (src/): Core logic testing (IDL parsing, type mapping, code generation)

**Integration tests** (tests/integration_tests.rs):
- Generates all configured IDL bindings
- Compiles each generated crate to verify syntax correctness
- Tests specific patterns: event wrappers, discriminators, serialization

**Performance tests** (tests/performance_tests.rs):
- Measures code generation speed for each program
- Validates performance targets (avg ~84ms per program)

**Benchmarks** (benches/): Criterion-based performance benchmarks with HTML reports

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
1. Check formatting (`cargo fmt --check`)
2. Run clippy (`cargo clippy -- -D warnings`)
3. Run unit tests
4. Generate all IDL bindings
5. Verify all generated crates compile
6. Run integration tests

All checks must pass before merging PRs.

## Common Patterns

### Adding a New IDL Source

1. Add IDL file to `idl/` directory (or use git submodule)
2. Update `justfile`:
   ```just
   projects := "... new_module"
   idls := "... new_module:path/to/idl.json"
   ```
3. Run `just generate` to generate bindings
4. Run `just check-generated` to verify compilation

### Testing Generated Code

Generated code should be tested by:
1. Compilation (automatic via integration tests)
2. Serialization round-trip tests (in consuming projects)
3. Discriminator validation against on-chain data

### Handling New IDL Features

When IDL spec adds new features:
1. Update `idl.rs` structs with `#[serde(default)]` for backward compatibility
2. Update `codegen.rs` to handle new feature
3. Add unit tests for the new pattern
4. Add integration test verifying generated code compiles
5. Update type mapping documentation

## Performance Considerations

- Code generation is fast (~84ms avg) - no need for incremental generation
- Integration tests are slow (~61s) because they compile all generated crates
- Use `just test` for fast iteration, `just test-all` before committing
- Benchmark with `just bench` if modifying codegen performance-critical paths

## Documentation

Key documentation files:
- `README.md` - User-facing usage guide
- `INTEGRATION_TESTING.md` - How to write integration tests
- `TEST_RESULTS.md` - Detailed test results
- `PERFORMANCE_ANALYSIS.md` - Performance metrics and analysis
- `CODEGEN_FEATURES.md` - Supported IDL features
- `EVENT_WRAPPER_PATTERN.md` - Event discriminator pattern
- `BENCHMARKING.md` - Benchmarking guide
