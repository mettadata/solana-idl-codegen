# Data Model: IDL Override System

**Feature**: IDL Override System | **Date**: 2025-12-26 | **Plan**: [plan.md](./plan.md)

## Overview

This document defines the data structures and validation rules for the IDL override system. All structures are Rust types that will be implemented in `src/override.rs`.

## Core Entities

### 1. OverrideFile

**Purpose**: Root structure representing a complete override file for a single IDL

**Rust Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideFile {
    /// Optional program address override (base58-encoded Pubkey)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_address: Option<String>,

    /// Account discriminator overrides (account name → discriminator)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub accounts: HashMap<String, DiscriminatorOverride>,

    /// Event discriminator overrides (event name → discriminator)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub events: HashMap<String, DiscriminatorOverride>,

    /// Instruction discriminator overrides (instruction name → discriminator)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub instructions: HashMap<String, DiscriminatorOverride>,
}
```

**Validation Rules**:
- At least one field must be non-empty (program_address or one of the maps)
- Program address if present must be valid base58 Pubkey
- All discriminator overrides must pass DiscriminatorOverride validation
- Entity names (map keys) are case-sensitive, must match IDL exactly

**JSON Example**:
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
  }
}
```

---

### 2. DiscriminatorOverride

**Purpose**: Represents an 8-byte discriminator override for an account, event, or instruction

**Rust Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscriminatorOverride {
    /// 8-byte discriminator array
    pub discriminator: [u8; 8],
}
```

**Validation Rules**:
- Must be exactly 8 bytes
- Must not be all zeros: [0, 0, 0, 0, 0, 0, 0, 0]
- Each byte must be in range 0-255 (enforced by u8 type)

**JSON Example**:
```json
{
  "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
}
```

---

### 3. OverrideDiscovery

**Purpose**: Result of override file discovery process

**Rust Enum**:
```rust
#[derive(Debug, Clone)]
pub enum OverrideDiscovery {
    /// Override file found at path
    Found(PathBuf),

    /// No override file found (not an error)
    NotFound,

    /// Multiple override files detected (error)
    Conflict {
        files: Vec<PathBuf>,
        sources: Vec<String>, // e.g., "convention-based", "explicit CLI"
    },
}
```

**State Transitions**:
- Start → Search conventional locations → Found | NotFound
- Start → Check explicit CLI arg → Found | Conflict (if convention file also exists)
- Found → Load and parse override file
- NotFound → Continue without overrides
- Conflict → Fail with error listing all conflicting files

---

### 4. ValidationError

**Purpose**: Represents validation errors for override files

**Rust Enum**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid program address: {address}. Must be valid base58-encoded Pubkey.")]
    InvalidProgramAddress { address: String },

    #[error("Invalid program address: {address}. Cannot be system default pubkey.")]
    SystemDefaultPubkey { address: String },

    #[error("Invalid discriminator for {entity_type} '{entity_name}': must be exactly 8 bytes")]
    InvalidDiscriminatorLength {
        entity_type: String,
        entity_name: String,
    },

    #[error("Invalid discriminator for {entity_type} '{entity_name}': cannot be all zeros")]
    AllZeroDiscriminator {
        entity_type: String,
        entity_name: String,
    },

    #[error("Empty override file: must contain at least one override")]
    EmptyOverrideFile,

    #[error("Unknown {entity_type} '{entity_name}' in override file. Available: {available}")]
    UnknownEntity {
        entity_type: String,
        entity_name: String,
        available: String,
    },
}
```

**Error Handling Strategy**:
- Validation errors are fatal (fail fast with clear message)
- Unknown entity warnings (log and continue, ignore override)
- Malformed JSON errors propagate from serde_json with file path context

---

### 5. AppliedOverride (Internal Tracking)

**Purpose**: Tracks which overrides were successfully applied (for logging/debugging)

**Rust Structure**:
```rust
#[derive(Debug, Clone)]
pub struct AppliedOverride {
    pub override_type: OverrideType,
    pub entity_name: Option<String>, // None for program_address
    pub original_value: Option<String>,
    pub override_value: String,
}

#[derive(Debug, Clone)]
pub enum OverrideType {
    ProgramAddress,
    AccountDiscriminator,
    EventDiscriminator,
    InstructionDiscriminator,
}
```

**Purpose**: Used for generating warning messages showing what was overridden

---

## Data Relationships

```
OverrideFile (1)
    ├─ program_address: Option<String> (0..1)
    ├─ accounts: HashMap<String, DiscriminatorOverride> (0..*)
    ├─ events: HashMap<String, DiscriminatorOverride> (0..*)
    └─ instructions: HashMap<String, DiscriminatorOverride> (0..*)

DiscriminatorOverride (many)
    └─ discriminator: [u8; 8] (1)

OverrideDiscovery (result type)
    ├─ Found(PathBuf) (1)
    ├─ NotFound (1)
    └─ Conflict { files: Vec<PathBuf>, sources: Vec<String> } (1)
```

**Cardinality**:
- One OverrideFile per IDL file
- Zero or more discriminator overrides per entity type (accounts/events/instructions)
- Exactly one OverrideDiscovery result per discovery attempt

---

## Validation State Machine

```
[Override File Loaded]
    ↓
[Validate Structure]
    ├─ Empty file? → Error
    ├─ Program address invalid? → Error
    └─ Any discriminator invalid? → Error
    ↓
[Validate Against IDL]
    ├─ Unknown account? → Warning (ignore override)
    ├─ Unknown event? → Warning (ignore override)
    └─ Unknown instruction? → Warning (ignore override)
    ↓
[Apply Valid Overrides]
    ├─ Program address → Update IDL metadata
    ├─ Account discriminators → Update account discriminators in IDL
    ├─ Event discriminators → Update event discriminators in IDL
    └─ Instruction discriminators → Update instruction discriminators in IDL
    ↓
[Log Applied Overrides]
    └─ Generate warning for each override showing original vs new value
    ↓
[Return Modified IDL]
```

---

## Integration with Existing IDL Structures

### IDL Structure (from src/idl.rs)

The override system integrates with existing IDL structures:

```rust
// Existing IDL structure (simplified)
pub struct Idl {
    pub metadata: Option<Metadata>,
    pub accounts: Option<Vec<IdlAccountItem>>,
    pub events: Option<Vec<IdlEvent>>,
    pub instructions: Vec<IdlInstruction>,
    // ... other fields
}

pub struct Metadata {
    pub name: String,
    pub version: String,
    pub address: Option<String>, // <-- Override target
    // ... other fields
}

pub struct IdlAccountItem {
    pub name: String,
    pub discriminator: Option<Vec<u8>>, // <-- Override target
    // ... other fields
}

pub struct IdlEvent {
    pub name: String,
    pub discriminator: Option<Vec<u8>>, // <-- Override target
    // ... other fields
}

pub struct IdlInstruction {
    pub name: String,
    pub discriminator: Option<Vec<u8>>, // <-- Override target
    // ... other fields
}
```

### Override Application Logic

```rust
pub fn apply_overrides(mut idl: Idl, override_file: OverrideFile) -> Result<Idl> {
    // 1. Apply program address override
    if let Some(address) = override_file.program_address {
        if let Some(metadata) = &mut idl.metadata {
            metadata.address = Some(address);
        } else {
            // Create metadata if missing
            idl.metadata = Some(Metadata {
                name: idl.name.clone(),
                version: idl.version.clone(),
                address: Some(address),
            });
        }
    }

    // 2. Apply account discriminator overrides
    if let Some(accounts) = &mut idl.accounts {
        for account in accounts {
            if let Some(override_disc) = override_file.accounts.get(&account.name) {
                account.discriminator = Some(override_disc.discriminator.to_vec());
            }
        }
    }

    // 3. Apply event discriminator overrides
    if let Some(events) = &mut idl.events {
        for event in events {
            if let Some(override_disc) = override_file.events.get(&event.name) {
                event.discriminator = Some(override_disc.discriminator.to_vec());
            }
        }
    }

    // 4. Apply instruction discriminator overrides
    for instruction in &mut idl.instructions {
        if let Some(override_disc) = override_file.instructions.get(&instruction.name) {
            instruction.discriminator = Some(override_disc.discriminator.to_vec());
        }
    }

    Ok(idl)
}
```

---

## File System Locations

### Conventional Override File Paths

**Per-IDL Convention**:
- Path pattern: `./overrides/{idl_name}.json`
- Example: `./overrides/raydium_amm.json` for `raydium_amm` IDL
- Scoped to single IDL file

**Global Fallback**:
- Path: `./idl-overrides.json`
- Used when per-IDL file not found
- Useful for simple projects with single IDL

**Explicit Override**:
- CLI flag: `--override-file <path>`
- Absolute or relative path
- Highest priority, bypasses convention-based discovery

---

## Example Override Files

### Example 1: Missing Program Address

```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
}
```

### Example 2: Incorrect Account Discriminators

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

### Example 3: Comprehensive Override

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
    },
    "AddLiquidityEvent": {
      "discriminator": [25, 26, 27, 28, 29, 30, 31, 32]
    }
  },
  "instructions": {
    "initialize": {
      "discriminator": [33, 34, 35, 36, 37, 38, 39, 40]
    }
  }
}
```

---

## Summary

The data model provides:

1. **Type-safe structures** using Rust's type system and serde
2. **Clear validation rules** with actionable error messages
3. **Simple JSON schema** matching IDL structure
4. **Minimal overhead** - small structs, simple validation
5. **Backward compatible** - all fields optional with `#[serde(default)]`
6. **Testable** - pure data structures with clear contracts

Ready to proceed to contract generation and quickstart documentation.
