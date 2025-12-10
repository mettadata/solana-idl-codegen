# Solana IDL Codegen - Feature Summary

## Overview
This codegen tool generates complete Rust bindings from Solana IDL (Interface Description Language) files, supporting both old and new IDL formats.

## Supported IDL Files
All IDL files from the `justfile` are fully supported:
- ✅ `idl/raydium-idl/raydium_amm/idl.json` - Raydium AMM v0.3.0 (old format)
- ✅ `idl/raydium-idl/raydium_clmm/amm_v3.json` - Raydium CLMM v0.1.0 (new format)
- ✅ `idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json` - Raydium CPMM v0.2.0 (new format)
- ✅ `idl/pumpfun/pump-fun-idl.json` - Pump.fun v0.1.0 (old format with events)

## Generated Code Features

### 1. Type Definitions
- **Structs**: Full struct generation with all fields
- **Enums**: Support for unit, tuple, and named variants
- **Nested Types**: Proper handling of nested and defined types
- **Documentation**: All doc comments from IDL are preserved
- **Serialization**: Automatic `BorshSerialize` and `BorshDeserialize` derives

### 2. Account Structures
- **Old Format**: Accounts with inline type definitions
- **New Format**: Accounts that reference types from the types array
- **Discriminators**: Full support with serialization/deserialization helpers
- **Bytemuck Support**: `Pod` and `Zeroable` traits for C-compatible types

#### Account Discriminator Methods
```rust
impl AccountType {
    pub const DISCRIMINATOR: [u8; 8] = [...];
    
    pub fn try_from_slice_with_discriminator(data: &[u8]) -> std::io::Result<Self>;
    pub fn serialize_with_discriminator<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>;
}
```

### 3. Instructions
- **Instruction Enum**: All instructions in a single enum
- **Args Structs**: Separate structs for instruction arguments
- **Accounts Structs**: Type-safe account structures per instruction
- **Discriminator Support**: Both explicit (new format) and index-based (old format)

#### Instruction Discriminator Methods
```rust
impl Instruction {
    pub fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>;
    pub fn try_from_slice(data: &[u8]) -> std::io::Result<Self>;
}
```

The `serialize` method automatically prepends the 8-byte discriminator, and `try_from_slice` validates and strips it.

### 4. Events
- **Event Structures**: Full struct generation from event fields
- **Old Format**: Events with inline field definitions
- **New Format**: Events that reference type definitions
- **Discriminator Support**: Constant discriminator arrays

#### Event Features
```rust
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct EventType {
    pub field1: Type1,
    pub field2: Type2,
}
```

### 5. Error Enums
- **Error Types**: Complete error enum generation
- **Error Messages**: Doc comments with error descriptions
- **Error Codes**: Preserved from IDL

### 6. Discriminator Logic

#### What are Discriminators?
Discriminators are 8-byte identifiers used by Solana programs to:
- Identify instruction types
- Identify account types
- Identify event types

They are typically the first 8 bytes of the serialized data.

#### Discriminator Support
- **Instructions**: 
  - New format: Uses explicit discriminators from IDL
  - Old format: Uses instruction index as u64 little-endian discriminator
- **Accounts**: Uses 8-byte discriminators from account definitions
- **Events**: Uses discriminators for event identification

#### Serialization with Discriminators
```rust
// Account serialization
let mut data = Vec::new();
account.serialize_with_discriminator(&mut data)?;
// data now contains: [8-byte discriminator][borsh-serialized account]

// Instruction serialization
let mut data = Vec::new();
instruction.serialize(&mut data)?;
// data now contains: [8-byte discriminator][borsh-serialized args]
```

#### Deserialization with Discriminators
```rust
// Account deserialization
let account = AccountType::try_from_slice_with_discriminator(&data)?;
// Validates discriminator and deserializes remaining bytes

// Instruction deserialization
let instruction = Instruction::try_from_slice(&data)?;
// Validates discriminator and deserializes correct variant
```

## IDL Format Support

### Old Format (Anchor v0.x)
- Fields: `isMut`, `isSigner`
- No explicit discriminators (uses indices)
- Accounts with inline type definitions
- Events as simple references

### New Format (Anchor v1.x)
- Fields: `writable`, `signer`
- Explicit 8-byte discriminators
- Accounts reference types array
- Events with inline field definitions
- Account addresses and PDAs

### Mixed Format Handling
The codegen seamlessly handles both formats and can parse IDLs with:
- Optional metadata fields
- Mixed account field formats
- Various serialization types (`borsh`, `bytemuck`, `bytemuckunsafe`)

## Code Quality Features

### Type Safety
- Strong typing for all fields
- Proper Option and Vec handling
- Array types with compile-time sizes

### Documentation
- All IDL doc comments preserved
- Generated doc comments for errors
- Clear type annotations

### Serialization Formats
- **Borsh**: Default serialization for most types
- **Bytemuck**: For C-compatible packed structures
  - Automatic `#[repr(C)]` or `#[repr(C, packed)]`
  - Unsafe `Pod` and `Zeroable` implementations

## Generated Code Statistics

| File | Lines | Accounts | Types | Instructions |
|------|-------|----------|-------|--------------|
| pumpfun.rs | 258 | 2 | 0 | 6 |
| raydium_amm.rs | 776 | 3 | 9 | 16 |
| raydium_clmm.rs | 1,670 | 9 | 25 | 25 |
| raydium_cpmm.rs | 644 | 3 | 6 | 10 |
| **Total** | **3,348** | **17** | **40** | **57** |

## Usage Example

```rust
use borsh::{BorshDeserialize, BorshSerialize};

// Deserialize an instruction
let instruction = Instruction::try_from_slice(&instruction_data)?;

// Match on instruction type
match instruction {
    Instruction::Swap(args) => {
        // Handle swap with args.amount, args.is_base_input, etc.
    }
    Instruction::Initialize => {
        // Handle initialize
    }
    _ => {}
}

// Serialize an instruction
let instruction = Instruction::Swap(SwapArgs {
    amount: 1000,
    other_amount_threshold: 900,
    sqrt_price_limit_x64: 0,
    is_base_input: true,
});
let mut data = Vec::new();
instruction.serialize(&mut data)?;

// Deserialize an account
let account_data = &mut &account_info.data.borrow()[..];
let pool = PoolState::try_from_slice_with_discriminator(account_data)?;
```

## Build and Test

```bash
# Generate all IDL bindings
just generate

# Format generated code
just format
```

## Implementation Details

### Key Files
- `src/idl.rs`: IDL structure definitions with serde support
- `src/codegen.rs`: Code generation logic
- `src/main.rs`: CLI interface

### Design Decisions
1. **Separate Args Structs**: Each instruction with arguments gets its own Args struct
2. **Separate Accounts Structs**: Each instruction gets a typed Accounts struct
3. **No Duplicate Types**: In new format, accounts reference types instead of duplicating
4. **Discriminator as Constant**: Easy to reference and compare
5. **Helper Methods**: Convenience methods for serialization/deserialization with discriminators
