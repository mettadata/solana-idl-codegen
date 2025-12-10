# Quick Start Guide - Solana IDL Code Generator

## What This Tool Does

This Rust CLI tool automatically generates strongly-typed Rust code bindings from Solana program IDL (Interface Description Language) files. It's perfect for:

- 🚀 Rapid Solana client development
- 🔒 Type-safe program interactions
- 📦 Automated code generation from IDL
- 🎯 Integration with Anchor and custom Solana programs

## Installation

1. **Prerequisites**: Install Rust
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Build the tool**:
   ```bash
   cd solana-idl-codegen
   cargo build --release
   ```

3. **The binary will be at**: `target/release/solana-idl-codegen`

## Quick Example

```bash
# Generate bindings from the example counter program
./target/release/solana-idl-codegen \
    --input example_counter.json \
    --output src \
    --module counter

# Output: src/counter.rs
```

## Generated Code Features

### ✅ Instruction Enum
```rust
pub enum Instruction {
    Initialize(InitializeArgs),
    Increment,
    Decrement(DecrementArgs),
}
```

### ✅ Typed Arguments
```rust
pub struct InitializeArgs {
    pub initial_value: u64,
}
```

### ✅ Account Structures
```rust
pub struct InitializeAccounts {
    /// The counter account to initialize
    pub counter: Pubkey,
    /// The user initializing the counter
    pub user: Pubkey,
    pub system_program: Pubkey,
}
```

### ✅ Program Accounts
```rust
pub struct Counter {
    /// The authority that can modify this counter
    pub authority: Pubkey,
    /// The current count value
    pub count: u64,
    pub bump: u8,
}
```

### ✅ Custom Types
```rust
pub enum Operation {
    Add,
    Subtract,
    Reset { new_value: u64 },
}
```

### ✅ Errors
```rust
pub enum ProgramError {
    /// Error: Counter overflow occurred
    Overflow,
    /// Error: Counter underflow occurred
    Underflow,
}
```

## CLI Usage

```bash
solana-idl-codegen [OPTIONS]

Options:
  -i, --input <FILE>      Path to IDL JSON file (required)
  -o, --output <DIR>      Output directory [default: generated]
  -m, --module <NAME>     Module name [default: program]
  -h, --help             Show help
```

## Integration with Your Project

1. **Add dependencies to your Cargo.toml**:
   ```toml
   [dependencies]
   borsh = "0.10"
   solana-program = "1.18"
   ```

2. **Import the generated code**:
   ```rust
   mod counter; // The generated module
   use counter::*;
   ```

3. **Use the types**:
   ```rust
   // Create instruction data
   let ix = Instruction::Initialize(InitializeArgs {
       initial_value: 42,
   });
   
   // Serialize with Borsh
   let data = ix.try_to_vec()?;
   
   // Create account metas
   let accounts = InitializeAccounts {
       counter: counter_pubkey,
       user: user_pubkey,
       system_program: system_program::ID,
   };
   ```

## Supported IDL Features

| Feature | Support | Notes |
|---------|---------|-------|
| Instructions | ✅ | Full support with args and accounts |
| Accounts | ✅ | Struct generation with all fields |
| Types (Struct) | ✅ | Custom struct types |
| Types (Enum) | ✅ | Named and tuple variants |
| Errors | ✅ | Error enum with descriptions |
| Events | ✅ | Event struct generation |
| Primitive Types | ✅ | All Rust primitives |
| Vec/Option/Array | ✅ | Nested collections |
| Defined Types | ✅ | References to custom types |

## File Structure

```
solana-idl-codegen/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── idl.rs          # IDL deserialization
│   └── codegen.rs      # Code generation logic
├── Cargo.toml          # Dependencies
├── README.md           # Full documentation
├── QUICKSTART.md       # This file
└── example_counter.json # Example IDL
```

## Tips & Best Practices

1. **Always regenerate** after IDL changes
2. **Version control** the generated code for reproducibility
3. **Use meaningful module names** that match your program
4. **Check generated code** into git for easier debugging
5. **Keep IDL files** in a central location (e.g., `idl/` directory)

## Common Workflows

### Anchor Program Development
```bash
# After building your Anchor program
anchor build

# IDL is at target/idl/your_program.json
solana-idl-codegen \
    -i target/idl/your_program.json \
    -o client/src \
    -m your_program
```

### Client Application
```bash
# Place IDL in your client project
solana-idl-codegen \
    -i ./idl/program.json \
    -o ./src/generated \
    -m program_bindings
```

### Multiple Programs
```bash
# Generate bindings for multiple programs
for idl in idl/*.json; do
    name=$(basename "$idl" .json)
    solana-idl-codegen -i "$idl" -o "src/programs" -m "$name"
done
```

## Troubleshooting

### Issue: "Failed to parse IDL JSON"
- Verify your IDL file is valid JSON
- Check that it follows the Anchor IDL format

### Issue: Generated code doesn't compile
- Ensure you have the required dependencies
- Check that custom types are defined before use
- Verify Pubkey types are imported

### Issue: Type mismatches
- Regenerate after IDL changes
- Clear and rebuild your project
- Check IDL type definitions

## Next Steps

1. Read the full [README.md](README.md) for detailed documentation
2. Explore the [example_counter.json](example_counter.json) IDL
3. Try generating code from your own Solana program IDL
4. Integrate into your build process with `build.rs`

## Support

For issues or questions:
- Check the README.md for detailed documentation
- Review the example IDL and generated code
- Inspect the generated Rust code for insights

Happy coding! 🦀⚡
