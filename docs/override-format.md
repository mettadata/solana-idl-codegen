# IDL Override File Format Specification

**Version**: 1.0.0
**Last Updated**: 2025-12-26

## Overview

Override files allow developers to correct missing or incorrect data in Solana IDL files without modifying the upstream IDL sources. This is particularly useful for IDL files maintained in git submodules or when discriminators don't match deployed mainnet programs.

## File Format

Override files are JSON documents with the following structure:

```json
{
  "program_address": "string (optional)",
  "accounts": { "AccountName": { "discriminator": [u8; 8] } },
  "events": { "EventName": { "discriminator": [u8; 8] } },
  "instructions": { "InstructionName": { "discriminator": [u8; 8] } }
}
```

All fields are optional, but at least one must be present.

## Field Specifications

### program_address (optional)

**Type**: String (base58-encoded Solana Pubkey)
**Purpose**: Override or add the program's on-chain address

**Example**:
```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
}
```

**Validation Rules**:
- Must be valid base58-encoded Solana Pubkey (32 bytes when decoded)
- Cannot be the system default pubkey (`11111111111111111111111111111111`)
- Used for both missing and incorrect program addresses

### accounts (optional)

**Type**: Object mapping account names to discriminator overrides
**Purpose**: Correct account discriminators that don't match on-chain data

**Example**:
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

**Validation Rules**:
- Account names must match IDL exactly (case-sensitive)
- Discriminators must be exactly 8 bytes (u8 array)
- Discriminators cannot be all zeros: `[0, 0, 0, 0, 0, 0, 0, 0]`
- Unknown account names generate warnings but don't fail validation

### events (optional)

**Type**: Object mapping event names to discriminator overrides
**Purpose**: Correct event discriminators for parsing transaction logs

**Example**:
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

**Validation Rules**: Same as accounts

### instructions (optional)

**Type**: Object mapping instruction names to discriminator overrides
**Purpose**: Correct instruction discriminators for transaction construction

**Example**:
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

**Validation Rules**: Same as accounts

## Complete Example

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

## File Discovery

Override files are discovered using convention-based search:

1. **Per-IDL file**: `./overrides/{idl_name}.json`
   - Example: `./overrides/raydium_amm.json` for `raydium_amm` IDL
   - Most common pattern for projects with multiple IDLs

2. **Global fallback**: `./idl-overrides.json`
   - Used when per-IDL file not found
   - Suitable for simple projects with single IDL

3. **Explicit path**: `--override-file <path>` CLI argument
   - Highest priority, bypasses convention-based discovery
   - Useful for custom project structures

### Conflict Detection

If multiple override files are detected (e.g., both convention-based and explicit CLI argument), the tool will fail with an error message listing all conflicting files and their sources.

## Validation

Override files are validated in two stages:

### Stage 1: Format Validation

- At least one field must be non-empty
- Program address must be valid base58 Pubkey (if present)
- Discriminators must be exactly 8 bytes
- Discriminators cannot be all zeros
- JSON must be well-formed

### Stage 2: IDL Validation

- Account/event/instruction names are checked against IDL
- Unknown names generate warnings (logged, not fatal)
- Warnings include available names for debugging

## Error Messages

### Invalid Program Address
```
Error: Invalid program address: "not-a-valid-pubkey". Must be valid base58-encoded Pubkey.
```

### Invalid Discriminator Length
```
Error: Invalid discriminator for account 'PoolState': must be exactly 8 bytes
```

### All-Zero Discriminator
```
Error: Invalid discriminator for event 'SwapEvent': cannot be all zeros
```

### Unknown Entity Warning
```
Warning: Override file contains discriminator for account 'UnknownAccount' which does not exist in IDL 'raydium_amm'
  Override entry: accounts.UnknownAccount
  Available accounts: PoolState, AmmConfig, UserPosition
  Action: Remove unknown entry or verify IDL version matches
```

### Multiple Override Files
```
Error: Multiple override files detected for 'raydium_amm' IDL:
  - ./overrides/raydium_amm.json (convention-based discovery)
  - /custom/path/overrides.json (--override-file argument)

Please remove one of the conflicting override files or use --override-file exclusively.
```

## Best Practices

1. **Version Control**: Commit override files to git (they are source code, not generated artifacts)

2. **Documentation**: Add comments explaining why overrides are needed (JSON standard doesn't support comments, but you can add a README or use `_comment` fields that tools will ignore)

3. **Validation**: Always verify generated code compiles after adding overrides

4. **Minimal Overrides**: Only override what's necessary - avoid duplicating correct IDL data

5. **Maintenance**: Review overrides when updating IDL submodules - upstream fixes may make overrides obsolete

## Integration with Code Generation

Override files are applied during the IDL parsing phase, before code generation:

```
IDL JSON → Parse → Apply Overrides → Validate → Generate Code
```

Generated code will include the override values as if they were in the original IDL file.

## Performance

- Override file loading + validation: <10ms per file (target)
- Total codegen overhead from overrides: <5% (target)
- Convention-based discovery: <50ms (max 2 file checks)

## See Also

- `specs/001-idl-override/quickstart.md` - User-facing quick start guide
- `specs/001-idl-override/data-model.md` - Rust data structures
- `specs/001-idl-override/contracts/override-api.md` - API contract
