# Event Wrapper Pattern Implementation

## Overview
Implemented a dual-struct wrapper pattern for Solana program events, similar to the instruction data wrapper pattern.

## Pattern Details

### 1. Module-Level Discriminator Constants
When an event has a discriminator in the IDL, a module-level constant is generated:

```rust
pub const CREATE_EVENT_EVENT_DISCM: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
```

### 2. Data Struct
The core event data structure with all fields:

```rust
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_pubkey_as_string"))]
    pub mint: Pubkey,
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_pubkey_as_string"))]
    pub bonding_curve: Pubkey,
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_pubkey_as_string"))]
    pub user: Pubkey,
}
```

### 3. Wrapper Struct with Discriminator Handling
A wrapper struct that handles discriminator serialization:

```rust
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateEventEvent(pub CreateEvent);

impl borsh::BorshSerialize for CreateEventEvent {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        CREATE_EVENT_EVENT_DISCM.serialize(writer)?;
        self.0.serialize(writer)
    }
}

impl CreateEventEvent {
    pub fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let maybe_discm = <[u8; 8]>::deserialize(buf)?;
        if maybe_discm != CREATE_EVENT_EVENT_DISCM {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "discm does not match. Expected: {:?}. Received: {:?}",
                    CREATE_EVENT_EVENT_DISCM, maybe_discm
                )
            ));
        }
        Ok(Self(CreateEvent::deserialize(buf)?))
    }
}
```

## Features

### ✅ Pubkey Custom Serialization
Pubkey fields are automatically detected and configured for custom serde serialization:

```rust
#[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_pubkey_as_string"))]
pub mint: Pubkey,
```

This serializes Pubkeys as strings in JSON instead of byte arrays, making them human-readable.

### ✅ Helper Function in lib.rs
A helper function is added to `lib.rs` for Pubkey serialization:

```rust
#[cfg(feature = "serde")]
pub fn serialize_pubkey_as_string<S>(
    pubkey: &solana_program::pubkey::Pubkey,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&pubkey.to_string())
}
```

### ✅ Conditional Generation
The wrapper pattern is only generated when:
- The event has a discriminator in the IDL
- This keeps the generated code clean for events without discriminators

## Benefits for Off-Chain Use

1. **Event Parsing**: Easy to deserialize events from transaction logs
2. **Discriminator Validation**: Ensures you're parsing the correct event type
3. **JSON Serialization**: Pubkeys serialize as readable strings
4. **Type Safety**: Separate wrapper type for discriminator-aware operations

## Usage Example

```rust
use pumpfun::*;

// Deserialize an event from transaction log data
let event = CreateEventEvent::deserialize(&mut log_data)?;

// Access the event data
println!("Mint: {}", event.0.mint);
println!("Name: {}", event.0.name);

// Serialize to JSON (with serde feature)
let json = serde_json::to_string(&event)?;
```

## Comparison with Imported Code

Our implementation matches the imported code pattern:
- ✅ Module-level discriminator constants
- ✅ Dual-struct pattern (data + wrapper)
- ✅ Custom `BorshSerialize` on wrapper
- ✅ Custom `deserialize` method with validation
- ✅ Custom serde for Pubkey fields
- ✅ Conditional `#[cfg_attr(feature = "serde")]` for optional serde support

## Test Coverage

The implementation includes comprehensive tests:
- Event generation with discriminators
- Event generation without discriminators
- Pubkey field detection and custom serde attributes
- Wrapper struct generation
- Discriminator constant generation

All tests pass ✅
