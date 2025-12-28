# Solana IDL Code Generator

A Rust CLI tool that generates strongly-typed Rust code bindings from Solana IDL (Interface Description Language) files. Parses JSON IDL files and produces idiomatic Rust code with proper naming conventions, Borsh/Bytemuck serialization support, and type-safe Pubkey handling.

## Features

- **Type-Safe Code Generation**: Converts IDL JSON to idiomatic Rust with proper type mappings
- **Serialization Support**: Borsh (default) and Bytemuck (zero-copy) derives
- **Discriminator Handling**: Automatic discriminator validation for accounts, events, and instructions
- **IDL Override System**: Fix missing or incorrect program addresses and discriminators without modifying upstream IDL files
- **Convention-Based Discovery**: Automatic override file detection using `./overrides/{idl_name}.json`
- **Fail-Fast Validation**: Strict validation catches configuration errors immediately

## Quick Start

### Installation

```bash
cargo install --path .
```

### Basic Usage

Generate Rust bindings from an IDL file:

```bash
solana-idl-codegen \
  --input idl/raydium-idl/raydium_amm/idl.json \
  --output generated \
  --module raydium_amm
```

This creates `generated/raydium_amm/` with:
- `Cargo.toml` - Auto-generated with dependencies
- `src/lib.rs` - Module re-exports
- `src/types.rs` - Custom type definitions
- `src/accounts.rs` - Account structs with discriminators
- `src/instructions.rs` - Instruction enum + args/accounts
- `src/errors.rs` - Error enum with codes
- `src/events.rs` - Event structs with discriminators

### Using Generated Code

```rust
use raydium_amm::events::*;
use raydium_amm::accounts::*;

// Deserialize account with discriminator validation
let pool_state = PoolState::try_from_slice_with_discriminator(&account_data)?;

// Match event by discriminator
if data[0..8] == SwapEvent::DISCRIMINATOR {
    let event = SwapEvent::try_from_slice(&data[8..])?;
    println!("Swap: {} for {}", event.amount_in, event.amount_out);
}
```

## IDL Override System

### Why Override Files?

Solana IDL files maintained in git submodules or external repositories sometimes have:
- Missing program addresses
- Incorrect discriminators that don't match deployed mainnet programs
- Outdated metadata that needs correction without forking

Override files let you fix these issues without modifying upstream sources.

### Quick Example

Create `./overrides/raydium_amm.json`:

```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "accounts": {
    "PoolState": {
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
    }
  },
  "events": {
    "SwapEvent": {
      "discriminator": [17, 18, 19, 20, 21, 22, 23, 24]
    }
  },
  "instructions": {
    "initialize": {
      "discriminator": [33, 34, 35, 36, 37, 38, 39, 40]
    }
  }
}
```

Generate with automatic override detection:

```bash
solana-idl-codegen \
  --input idl/raydium-idl/raydium_amm/idl.json \
  --output generated \
  --module raydium_amm
```

The tool automatically finds `./overrides/raydium_amm.json` and applies overrides.

### Override File Discovery

The tool uses convention-based discovery with three priority levels:

1. **Per-IDL file**: `./overrides/{idl_name}.json`
   - Example: `./overrides/raydium_amm.json` for `raydium_amm` module
   - Most common pattern for projects with multiple IDLs

2. **Global fallback**: `./idl-overrides.json`
   - Used when per-IDL file not found
   - Suitable for simple projects with single IDL

3. **Explicit path**: `--override-file <path>` CLI argument
   - Highest priority, bypasses convention-based discovery
   - Useful for custom project structures

**Conflict Detection**: If multiple override sources are detected for the same IDL, the tool fails with an error listing all conflicting files.

### Override File Format

See [docs/override-format.md](docs/override-format.md) for complete specification.

**Minimal Example** (program address only):
```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
}
```

**Validation Rules**:
- At least one field must be non-empty
- Program address must be valid base58 Pubkey (32 bytes)
- Discriminators must be exactly 8 bytes
- Discriminators cannot be all zeros `[0, 0, 0, 0, 0, 0, 0, 0]`
- Entity names (accounts/events/instructions) must match IDL exactly (case-sensitive)
- Unknown entity names cause validation errors (fail-fast to catch typos)

### Multiple IDL Files

For projects with multiple IDL files, create one override file per IDL:

```
./overrides/
├── raydium_amm.json
├── raydium_clmm.json
├── raydium_cpmm.json
└── pumpfun.json
```

Each override file is automatically matched to its corresponding IDL by module name.

### Explicit Override Path

For custom project structures:

```bash
solana-idl-codegen \
  --input idl/custom/program.json \
  --output generated \
  --module my_program \
  --override-file /custom/path/overrides.json
```

### Common Use Cases

**1. Missing Program Address**

IDL file doesn't specify on-chain program address:

```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
}
```

**2. Incorrect Account Discriminators**

IDL discriminators don't match deployed program:

```json
{
  "accounts": {
    "PoolState": {
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
    },
    "UserPosition": {
      "discriminator": [9, 10, 11, 12, 13, 14, 15, 16]
    }
  }
}
```

**3. Event Log Parsing**

Fix event discriminators for transaction log parsing:

```json
{
  "events": {
    "SwapEvent": {
      "discriminator": [17, 18, 19, 20, 21, 22, 23, 24]
    },
    "AddLiquidityEvent": {
      "discriminator": [25, 26, 27, 28, 29, 30, 31, 32]
    }
  }
}
```

**4. Instruction Construction**

Correct instruction discriminators for transaction building:

```json
{
  "instructions": {
    "initialize": {
      "discriminator": [33, 34, 35, 36, 37, 38, 39, 40]
    },
    "swap": {
      "discriminator": [41, 42, 43, 44, 45, 46, 47, 48]
    }
  }
}
```

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
# Recommended: Use justfile commands
just test              # Fast unit tests
just test-all          # All tests (158 total)
just test-integration  # Integration tests only

# Or run directly with cargo
cargo test             # Unit tests
cargo test --all       # All tests
cargo test --test override_tests  # Override integration tests
```

### Code Quality

```bash
cargo fmt --check    # Formatting
cargo clippy         # Linting
```

## Type Mappings

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

## License

This project is part of the Pandora blockchain data ingestion system.

## Documentation

- [Override File Format Specification](docs/override-format.md) - Complete override file schema and validation rules
- [CLAUDE.md](CLAUDE.md) - Claude Code integration guide
