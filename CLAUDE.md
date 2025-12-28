# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust CLI tool that generates strongly-typed Rust code bindings from Solana IDL (Interface Description Language) files. Parses JSON IDL files and produces idiomatic Rust code with proper naming conventions, Borsh/Bytemuck serialization support, and type-safe Pubkey handling.

**Usage Context**: This tool generates Solana program interfaces for use in blockchain data ingestion systems, trading bots, and other applications that need to decode on-chain transactions and events.

**IMPORTANT**: The `generated/` directory is gitignored and rebuilt on-demand. Never manually edit generated code - modify the IDL files or the codegen tool itself.

## Quick Reference

```bash
# Most common commands
just generate              # Generate all IDL bindings
just test                  # Fast unit tests
just check-all            # Full quality gate before commit

# Update IDL sources
git submodule update --remote

# Quality checks (CI enforced)
just fmt-check && just clippy

# Verify generated code
just check-generated       # Check compilation
just lint-generated        # Check formatting + clippy
```

## IDL Source Files (Git Submodules)

IDL files are managed as git submodules in `idl/`:
- `idl/raydium-idl/` - Raydium AMM/CLMM/CPMM IDLs
- `idl/pump-public-docs/` - PumpFun and PumpFun AMM IDLs

**Update submodules** to pull latest IDL changes:
```bash
git submodule update --init --recursive    # First time clone
git submodule update --remote              # Update to latest upstream
```

After updating submodules, regenerate bindings with `just generate`.

## Typical Workflow

```bash
# 1. Update IDL submodules (if needed)
git submodule update --remote

# 2. Generate bindings from IDLs
just generate

# 3. Make changes to codegen tool (src/*.rs)
# 4. Run quality checks
just fmt-check && just clippy

# 5. Test changes
just test                    # Fast unit tests
just check-generated         # Verify generated code compiles
just test-all               # Full test suite before commit

# 6. Commit changes (generated/ not included)
```

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
# Recommended: Use justfile commands (handles sequential execution automatically)
just test              # Fast unit tests (115 tests)
just test-all          # All tests (157 total)
just test-integration  # Integration tests only (30 tests)
just test-perf         # Performance benchmarks (5 tests)
just test-with-timing  # All tests with timing details

# Or run directly with cargo (sequential execution required)
cargo test -- --test-threads=1                    # Unit tests
cargo test --all -- --test-threads=1              # All tests
cargo test --test override_tests -- --test-threads=1  # Override integration tests
```

**Test Categories**:
- **Unit tests** (115 tests): Fast validation of codegen logic, type mappings, and override system
- **Integration tests** (30 tests): End-to-end verification
  - 16 pattern/structure tests (file structure, module organization, trait implementation)
  - 7 serialization round-trip tests (runtime behavior, discriminator validation)
  - 13 override system tests (file discovery, validation, application)
- **Generated code test** (1 test): Verifies generated code compiles
- **Performance tests** (5 tests): Benchmarking code generation speed
- **Serialization tests** (7 tests): Runtime behavior validation

**Important**: Tests must run sequentially (`--test-threads=1`) because integration tests copy fixture files and experience race conditions in parallel execution. The justfile commands handle this automatically.

### Generated Code Quality Gates

After code generation, verify generated crates compile and pass linting:

```bash
just check-generated            # Verify all generated crates compile
just build-generated            # Build all generated crates
just lint-generated             # Run fmt + clippy on generated code
just check-all                  # Complete quality gate (codegen + generated)
```


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

### Discriminators (Critical for Solana)

**Why discriminators matter**: Solana accounts and program logs are untyped byte arrays. Discriminators are 8-byte prefixes that identify the data structure type, enabling safe deserialization and preventing data misinterpretation.

**Account discriminators**: 8-byte arrays for account type identification
- Generated as `const DISCRIMINATOR: [u8; 8]`
- Provides `try_from_slice_with_discriminator()` validates discriminator before deserializing
- Provides `serialize_with_discriminator()` prepends discriminator when writing
- Handles both Borsh and Bytemuck types with appropriate deserialization

**Event discriminators**: Same pattern for parsing events from program logs
- Critical for blockchain data ingestion systems that decode transaction logs
- Enables matching raw log data to specific event types

## IDL Override System

**Purpose**: Fix missing or incorrect program addresses and discriminators without modifying upstream IDL files (git submodules, external repositories).

**src/override.rs**: Override system implementation
- `OverrideFile` struct: JSON-serializable override data structure
- `DiscriminatorOverride` struct: 8-byte discriminator for accounts/events/instructions
- `AppliedOverride` struct: Tracking applied overrides for logging
- `OverrideDiscovery` enum: Found/NotFound/Conflict states for file discovery
- `ValidationError` enum: Comprehensive validation error types
- `discover_override_file()`: Convention-based file discovery (3 priority levels)
- `load_override_file()`: JSON deserialization with error context
- `validate_override_file()`: Strict fail-fast validation
- `apply_overrides()`: Applies overrides to parsed IDL before code generation

### Override File Discovery (Convention-Based)

The system uses three priority levels for discovering override files:

**Priority 1: Per-IDL file** - `./overrides/{idl_name}.json`
- Example: `./overrides/raydium_amm.json` for `raydium_amm` module
- Most common pattern for multi-IDL projects
- Automatic discovery based on module name

**Priority 2: Global fallback** - `./idl-overrides.json`
- Used when per-IDL file not found
- Suitable for simple single-IDL projects

**Priority 3: Explicit path** - `--override-file <path>` CLI argument
- Highest priority, bypasses convention-based discovery
- Required for custom project structures

**Conflict Detection**: If multiple override sources exist for the same IDL, the system fails with `OverrideDiscovery::Conflict` listing all conflicting files.

### Override File Format

JSON structure with optional fields (at least one required):

```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "accounts": {
    "PoolState": { "discriminator": [1, 2, 3, 4, 5, 6, 7, 8] }
  },
  "events": {
    "SwapEvent": { "discriminator": [17, 18, 19, 20, 21, 22, 23, 24] }
  },
  "instructions": {
    "initialize": { "discriminator": [33, 34, 35, 36, 37, 38, 39, 40] }
  }
}
```

**Validation Rules** (fail-fast):
- Program address: Valid base58 Pubkey (32 bytes), cannot be system default `11111111111111111111111111111111`
- Discriminators: Exactly 8 bytes, cannot be all zeros `[0, 0, 0, 0, 0, 0, 0, 0]`
- Entity names: Must match IDL exactly (case-sensitive)
- Unknown entity names: Return `ValidationError::UnknownEntity` with available names
- Empty files: Return `ValidationError::EmptyOverrideFile`
- Malformed JSON: anyhow::Error with context

### Integration with Code Generation

Override application happens **before** code generation:

```
IDL JSON → Parse → Discover Override → Load → Validate → Apply → Generate Code
```

**Applied Overrides Logging**:
- System logs which overrides were applied (address, accounts, events, instructions)
- Helps verify correct override file was used
- Critical for debugging discriminator issues

### Common Use Cases

**1. Missing Program Address**:
```json
{ "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" }
```

**2. Incorrect Account Discriminators** (on-chain data decoding):
```json
{
  "accounts": {
    "PoolState": { "discriminator": [1, 2, 3, 4, 5, 6, 7, 8] },
    "UserPosition": { "discriminator": [9, 10, 11, 12, 13, 14, 15, 16] }
  }
}
```

**3. Event Log Parsing** (blockchain data ingestion):
```json
{
  "events": {
    "SwapEvent": { "discriminator": [17, 18, 19, 20, 21, 22, 23, 24] },
    "AddLiquidityEvent": { "discriminator": [25, 26, 27, 28, 29, 30, 31, 32] }
  }
}
```

**4. Transaction Construction**:
```json
{
  "instructions": {
    "initialize": { "discriminator": [33, 34, 35, 36, 37, 38, 39, 40] },
    "swap": { "discriminator": [41, 42, 43, 44, 45, 46, 47, 48] }
  }
}
```

### Multiple IDL Projects

For projects with multiple IDLs, create one override file per IDL:

```
./overrides/
├── raydium_amm.json
├── raydium_clmm.json
├── raydium_cpmm.json
├── pumpfun.json
└── pumpfun_amm.json
```

Each file is automatically matched to its corresponding IDL by module name. No conflicts occur because each IDL has its own dedicated override file.

### Example Override Files

See `overrides/` directory for examples:
- `example_program_address.json` - Minimal address-only override
- `example_account_discriminators.json` - Account discriminator fixes
- `example_event_discriminators.json` - Event log parsing fixes
- `example_instruction_discriminators.json` - Transaction construction fixes
- `example_complete.json` - Complete example with all override types

**Note**: These are examples only - copy and modify for your specific program's discriminators.

### Testing Override System

**Unit tests** (src/override.rs): 24 tests covering:
- File discovery (per-IDL, global, explicit, conflicts)
- JSON loading (valid, malformed, empty, missing files)
- Validation (addresses, discriminators, entity names, empty overrides)
- Override application (accounts, events, instructions, program address)
- Error messages with available entity names

**Integration tests** (tests/override_tests.rs): 13 tests covering:
- End-to-end override application in generated code
- Account/event/instruction discriminator overrides
- Program address overrides
- Validation error propagation
- Multiple override file conflicts
- Empty and malformed override files

Run override system tests:
```bash
# Recommended: Use justfile commands (handles sequential execution)
just test              # Fast unit tests (115 tests)
just test-all          # All tests (157 total)

# Or run directly with cargo (sequential execution required)
cargo test --all -- --test-threads=1              # All tests
cargo test --lib -- --test-threads=1              # Unit tests only
cargo test --test override_tests -- --test-threads=1  # Override integration tests (13 tests)
```

**Note**: Integration tests copy fixture files and must run sequentially to avoid race conditions. The justfile commands handle this automatically.

### Documentation

- `docs/override-format.md` - Complete format specification with validation rules
- `overrides/README.md` - Example file usage guide
- `README.md` - User-facing override system guide
- `specs/001-idl-override/` - Feature specification, plan, and tasks

## Project Structure

**src/**: Codegen tool source
- `main.rs` - CLI entry point
- `idl.rs` - IDL data structures with dual format support
- `codegen.rs` - Token-based code generation engine
- `override.rs` - IDL override system implementation

**idl/**: Git submodules containing IDL JSON files (raydium-idl, pump-public-docs)

**overrides/**: Example override files (gitignored in production, examples committed)
- `example_program_address.json` - Program address override example
- `example_account_discriminators.json` - Account discriminator example
- `example_event_discriminators.json` - Event discriminator example
- `example_instruction_discriminators.json` - Instruction discriminator example
- `example_complete.json` - Complete override example
- `README.md` - Override examples documentation

**docs/**: Documentation
- `override-format.md` - Override file format specification

**generated/**: Auto-generated Rust crates (gitignored, not committed)

**imported/**: Manually maintained reference implementations
- `pump_interface/` - Hand-written Pump interface for comparison
- Used as reference when improving codegen output

**tests/**: Integration and performance tests

**benches/**: Criterion benchmarks

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

## Error Handling

**Tool code** (`src/`): Uses `anyhow::Result` with `.context()` for error chains
```rust
let idl_content = fs::read_to_string(&cli.input)
    .context(format!("Failed to read IDL file: {:?}", cli.input))?;
```

**Generated code**: Uses `std::io::Result` for discriminator validation and deserialization

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

## Key Implementation Details

### Naming Conventions (via heck crate)
- IDL names → Rust: `snake_case` for fields, `PascalCase` for types/enums
- Example: IDL `publicKey` → Rust `pub_key`, IDL `TradeEvent` → `TradeEvent`

### Code Formatting
- Uses `prettyplease` for consistent formatting of generated code
- All generated code passes `cargo fmt --check` and `cargo clippy`

### Module Organization
Generated crates split code across 6 modules to avoid monolithic files:
- `lib.rs` - Public API with re-exports
- `types.rs` - Custom type definitions
- `accounts.rs` - Account structs with discriminators
- `instructions.rs` - Instruction enum, args, and account structs
- `errors.rs` - Error enum with codes and messages
- `events.rs` - Event structs with discriminators

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
1. Check formatting (`cargo fmt --check`)
2. Run clippy (`cargo clippy -- -D warnings`)
3. Run unit tests
4. Generate all IDL bindings
5. Verify all generated crates compile
6. Run integration tests

All checks must pass before merging PRs.

## Common Workflows

### Adding a New Solana Program Interface

**Example**: Adding a new Orca Whirlpool interface

1. **Add IDL source** (git submodule or local file):
   ```bash
   git submodule add git@github.com:orca-so/whirlpools-idl.git idl/whirlpools-idl
   ```

2. **Update justfile**:
   ```just
   projects := "raydium_amm raydium_clmm ... orca_whirlpool"
   idls := "... orca_whirlpool:idl/whirlpools-idl/whirlpool.json"
   ```

3. **Generate and verify**:
   ```bash
   just generate
   just check-generated
   ```

4. **Use in your application**:
   - Generated code is available at `generated/orca_whirlpool/`
   - Copy to your project's interface directory
   - Import in your decoder: `use orca_whirlpool::events::*;`

### Improving Codegen for Missing Features

See `codegen-improvements.md` for systematic improvements needed:
- Instruction→Event mapping generation
- Pubkey→String conversion helpers
- Event wrapper patterns
- Account context helpers

When adding new codegen features:
1. Update `idl.rs` for new IDL fields (`#[serde(default)]` for compatibility)
2. Implement in `codegen.rs` using `quote!` macros
3. Add unit test in `src/codegen.rs` tests
4. Add integration test verifying generated code compiles
5. Update type mapping table if applicable

### Debugging Generated Code Issues

If generated code doesn't compile:
1. Check `just check-generated` output for specific errors
2. Inspect `generated/<module>/src/` files manually
3. Compare with `imported/pump_interface/` reference implementation
4. Use `RUST_LOG=debug` for verbose codegen output (if implemented)
5. Add minimal reproduction case to integration tests

## Performance Notes

- **Code generation**: Fast (<100ms per program) - always regenerate from scratch
- **Unit tests**: Very fast (<1s) - use for rapid iteration
- **Integration tests**: Slow (~60s) - compile all generated crates to verify correctness
- **Workflow**: Use `just test` during development, `just test-all` before commit
- **Benchmarking**: Run `just bench` when optimizing codegen performance (Criterion with HTML reports)

## Documentation

Key documentation files:
- `README.md` - User-facing usage guide, CLI reference, and override system guide
- `docs/override-format.md` - Override file format specification and validation rules
- `overrides/README.md` - Example override files usage guide
- `specs/001-idl-override/` - IDL Override System feature specification
  - `spec.md` - Requirements and design rationale
  - `plan.md` - Implementation plan
  - `tasks.md` - Task breakdown with completion tracking
  - `quickstart.md` - Quick reference guide
  - `data-model.md` - Data structures and validation rules
- `codegen-improvements.md` - **Active development roadmap** for reducing boilerplate in blockchain data ingestion systems
- `CODEGEN_FEATURES.md` - Supported IDL features and patterns
- `INTEGRATION_TESTING.md` - How to write integration tests
- `EVENT_WRAPPER_PATTERN.md` - Event discriminator pattern details
- `PERFORMANCE_ANALYSIS.md` - Performance metrics and benchmarking
- `TEST_RESULTS.md` - Detailed test execution results

## Active Technologies
- Rust 2021 edition, MSRV 1.70.0 (stable toolchain) (001-idl-override)
- File system (JSON override files), no database required (001-idl-override)

## Recent Changes
- 001-idl-override: Added Rust 2021 edition, MSRV 1.70.0 (stable toolchain)
