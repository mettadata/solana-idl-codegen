# Solana IDL Code Generator

A Rust CLI tool that generates strongly-typed Rust code bindings from Solana IDL (Interface Description Language) files.

## Features

- 📝 Parse Solana IDL JSON files
- 🦀 Generate idiomatic Rust code with proper naming conventions
- 🔧 Support for all IDL types: instructions, accounts, types, errors, and events
- 📦 Borsh serialization/deserialization support
- 🎯 Type-safe Pubkey handling
- 📚 Preserves documentation from IDL

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/solana-idl-codegen`

## Usage

### Basic Usage

```bash
solana-idl-codegen -i path/to/idl.json -o ./generated
```

### CLI Options

```
Options:
  -i, --input <FILE>      Path to the IDL JSON file
  -o, --output <DIR>      Output directory for generated code [default: generated]
  -m, --module <MODULE>   Module name for generated code [default: program]
  -h, --help             Print help
```

### Example

```bash
# Generate bindings from the example IDL
solana-idl-codegen -i example_counter.json -o src -m counter

# This will create src/counter.rs with all the type definitions
```

## Generated Code Structure

The tool generates:

### 1. **Type Definitions**
- Structs and enums from the `types` section
- Proper Rust naming conventions (snake_case for fields, PascalCase for types)
- Full Borsh serialization support

### 2. **Account Structs**
- Account data structures from the `accounts` section
- Includes all fields with their proper types

### 3. **Instructions**
- Main `Instruction` enum with all program instructions
- Separate args structs for each instruction (e.g., `InitializeArgs`)
- Account structs for each instruction (e.g., `InitializeAccounts`)

### 4. **Errors**
- `ProgramError` enum with all error codes
- Error messages preserved from IDL

### 5. **Events**
- Event structs with all fields
- Ready for event logging/parsing

## Example Generated Code

For an IDL with an `initialize` instruction:

```rust
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum Instruction {
    Initialize(InitializeArgs),
    Increment,
    // ...
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct InitializeArgs {
    pub initial_value: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitializeAccounts {
    /// The counter account to initialize
    pub counter: Pubkey,
    /// The user initializing the counter
    pub user: Pubkey,
    pub system_program: Pubkey,
}
```

## Supported IDL Types

### Primitive Types
- `bool`, `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`
- `f32`, `f64`
- `string` → `String`
- `publicKey` → `Pubkey`
- `bytes` → `Vec<u8>`

### Composite Types
- `Vec<T>` → `Vec<T>`
- `Option<T>` → `Option<T>`
- `[T; N]` → `[T; N]` (fixed-size arrays)
- Custom defined types

## Dependencies in Generated Code

The generated code assumes you have these dependencies in your `Cargo.toml`:

```toml
[dependencies]
borsh = "0.10"
solana-program = "1.18"
```

## Project Structure

```
solana-idl-codegen/
├── src/
│   ├── main.rs       # CLI entry point
│   ├── idl.rs        # IDL data structures
│   └── codegen.rs    # Code generation logic
├── Cargo.toml
└── example_counter.json  # Example IDL file
```

## Development

### Running Tests

The project includes comprehensive unit and integration tests:

```bash
# Run unit tests
cargo test --lib

# Generate code and run integration tests
just test-integration

# Run all tests (unit + integration)
just test-all
```

The integration tests verify:
- ✅ All generated crates compile successfully
- ✅ Event wrapper pattern with discriminators
- ✅ Account discriminators for state parsing
- ✅ Instruction enum serialization
- ✅ Error enum with proper codes
- ✅ Pubkey serde serialization as strings
- ✅ Proper Cargo.toml dependencies
- ✅ Module structure and re-exports

See [TEST_RESULTS.md](TEST_RESULTS.md) for detailed test results and [INTEGRATION_TESTING.md](INTEGRATION_TESTING.md) for how to write additional tests.

### Running with Example

```bash
cargo run -- -i example_counter.json -o generated -m counter
```

### Using the Justfile

The project includes a `justfile` for common operations:

```bash
# Clean generated code and build artifacts
just clean

# Generate all configured IDL bindings
just generate

# Check all generated crates compile
just check

# Build all generated crates  
just build

# Run integration tests
just test-integration

# Run all tests
just test-all
```

## Type Mapping

| IDL Type | Rust Type |
|----------|-----------|
| `bool` | `bool` |
| `u8` - `u128` | `u8` - `u128` |
| `i8` - `i128` | `i8` - `i128` |
| `string` | `String` |
| `publicKey` | `Pubkey` |
| `bytes` | `Vec<u8>` |
| `{vec: T}` | `Vec<T>` |
| `{option: T}` | `Option<T>` |
| `{array: [T, N]}` | `[T; N]` |
| `{defined: "Custom"}` | `Custom` |

## Contributing

Contributions welcome! Areas for improvement:
- Additional type validations
- Support for more Anchor IDL features
- Integration tests with real Solana programs
- Pretty-printing options
- Documentation generation

## License

MIT License

## Author

Created for efficient Solana program development with strong typing guarantees.
