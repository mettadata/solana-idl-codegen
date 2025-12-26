# Override System API Contract

**Feature**: IDL Override System | **Date**: 2025-12-26 | **Plan**: [../plan.md](../plan.md)

## Overview

This document defines the public API contract for the override system. The override system is a Rust library module, not a REST/GraphQL API, so this contract specifies Rust function signatures and behaviors.

## Public Functions

### 1. discover_override_file

**Purpose**: Discover override file location using convention-based search or explicit path

**Signature**:
```rust
pub fn discover_override_file(
    idl_path: &Path,
    idl_name: &str,
    explicit_override: Option<&Path>,
) -> Result<OverrideDiscovery>
```

**Parameters**:
- `idl_path`: Path to the IDL file (used to resolve relative paths)
- `idl_name`: Name of the IDL (e.g., "raydium_amm")
- `explicit_override`: Optional explicit override file path from CLI `--override-file`

**Returns**:
- `OverrideDiscovery::Found(PathBuf)`: Override file found at path
- `OverrideDiscovery::NotFound`: No override file found (not an error)
- `OverrideDiscovery::Conflict { files, sources }`: Multiple override files detected (error)

**Errors**:
- Returns `Err` if file system errors occur (permission denied, I/O errors)
- Returns `Ok(Conflict)` if multiple override files detected

**Behavior**:
1. If `explicit_override` provided:
   - Check if file exists at explicit path
   - If conventional override also exists → return `Conflict`
   - Otherwise → return `Found(explicit_path)` or `NotFound`

2. If `explicit_override` is None:
   - Check `./overrides/{idl_name}.json`
   - If not found, check `./idl-overrides.json`
   - Return first found or `NotFound` if neither exists

**Example Usage**:
```rust
let discovery = discover_override_file(
    Path::new("idl/raydium_amm/idl.json"),
    "raydium_amm",
    cli.override_file.as_deref(),
)?;

match discovery {
    OverrideDiscovery::Found(path) => {
        // Load and apply overrides
    },
    OverrideDiscovery::NotFound => {
        // Continue without overrides
    },
    OverrideDiscovery::Conflict { files, sources } => {
        return Err(anyhow!(
            "Multiple override files detected: {:?} from {:?}",
            files, sources
        ));
    },
}
```

---

### 2. load_override_file

**Purpose**: Load and parse override file from disk

**Signature**:
```rust
pub fn load_override_file(path: &Path) -> Result<OverrideFile>
```

**Parameters**:
- `path`: Path to override JSON file

**Returns**:
- `Ok(OverrideFile)`: Successfully parsed override file
- `Err`: File I/O error or JSON parsing error

**Errors**:
- File not found: `anyhow!("Failed to read override file: {:?}", path)`
- Invalid JSON: `anyhow!("Failed to parse override file: {:?}", path)`
- Deserialization errors from serde_json propagated with context

**Behavior**:
1. Read file contents as string
2. Parse JSON using `serde_json::from_str`
3. Return parsed `OverrideFile` struct
4. All errors include file path in context

**Example Usage**:
```rust
let override_file = load_override_file(&override_path)
    .context("Failed to load override file")?;
```

---

### 3. validate_override_file

**Purpose**: Validate override file structure and values

**Signature**:
```rust
pub fn validate_override_file(
    override_file: &OverrideFile,
    idl: &Idl,
) -> Result<Vec<String>>
```

**Parameters**:
- `override_file`: Parsed override file to validate
- `idl`: IDL structure to validate against

**Returns**:
- `Ok(warnings)`: Validation passed, returns list of warning messages (e.g., unknown entities)
- `Err(ValidationError)`: Validation failed with specific error

**Validation Checks**:
1. **Structure validation**:
   - At least one field must be non-empty
   - Error: `ValidationError::EmptyOverrideFile`

2. **Program address validation**:
   - Must be valid base58 Pubkey
   - Error: `ValidationError::InvalidProgramAddress`
   - Must not be system default (11111...1111)
   - Error: `ValidationError::SystemDefaultPubkey`

3. **Discriminator validation**:
   - Must be exactly 8 bytes
   - Error: `ValidationError::InvalidDiscriminatorLength`
   - Must not be all zeros
   - Error: `ValidationError::AllZeroDiscriminator`

4. **Entity existence checks** (warnings, not errors):
   - Unknown account names → warning
   - Unknown event names → warning
   - Unknown instruction names → warning

**Example Usage**:
```rust
let warnings = validate_override_file(&override_file, &idl)
    .context("Override file validation failed")?;

for warning in warnings {
    eprintln!("Warning: {}", warning);
}
```

---

### 4. apply_overrides

**Purpose**: Apply validated overrides to IDL structure

**Signature**:
```rust
pub fn apply_overrides(
    mut idl: Idl,
    override_file: &OverrideFile,
) -> Result<(Idl, Vec<AppliedOverride>)>
```

**Parameters**:
- `idl`: IDL structure to modify (consumed and returned)
- `override_file`: Validated override file

**Returns**:
- `Ok((modified_idl, applied_overrides))`: Successfully applied overrides
  - `modified_idl`: IDL with overrides applied
  - `applied_overrides`: List of overrides that were applied (for logging)
- `Err`: Internal error during override application (should be rare if validation passed)

**Behavior**:
1. Apply program address override (if present)
   - Update `idl.metadata.address` or create metadata if missing

2. Apply account discriminator overrides
   - For each override, find matching account by name
   - Update `account.discriminator` field
   - Skip if account not found (already warned in validation)

3. Apply event discriminator overrides
   - For each override, find matching event by name
   - Update `event.discriminator` field
   - Skip if event not found

4. Apply instruction discriminator overrides
   - For each override, find matching instruction by name
   - Update `instruction.discriminator` field
   - Skip if instruction not found

5. Track all applied overrides for logging

**Example Usage**:
```rust
let (modified_idl, applied) = apply_overrides(idl, &override_file)?;

for override_info in applied {
    eprintln!(
        "Warning: Overriding {:?} '{}': {} → {}",
        override_info.override_type,
        override_info.entity_name.unwrap_or_default(),
        override_info.original_value.unwrap_or("(none)".to_string()),
        override_info.override_value,
    );
}

// Continue with modified_idl
generate_code(&output_dir, &module_name, &modified_idl)?;
```

---

## Complete Workflow Example

```rust
use solana_idl_codegen::override_system::*;

// 1. Discover override file
let discovery = discover_override_file(
    &cli.input,
    &idl_name,
    cli.override_file.as_deref(),
)?;

// 2. Load override file if found
let override_file = match discovery {
    OverrideDiscovery::Found(path) => {
        Some(load_override_file(&path)?)
    },
    OverrideDiscovery::NotFound => None,
    OverrideDiscovery::Conflict { files, sources } => {
        return Err(anyhow!(
            "Multiple override files detected for '{}' IDL:\n{}\n\nPlease remove conflicting files or use --override-file exclusively.",
            idl_name,
            files.iter().zip(sources.iter())
                .map(|(f, s)| format!("  - {} ({})", f.display(), s))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    },
};

// 3. Parse IDL
let mut idl = parse_idl(&cli.input)?;

// 4. Apply overrides if present
if let Some(override_file) = override_file {
    // Validate
    let warnings = validate_override_file(&override_file, &idl)?;
    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }

    // Apply
    let (modified_idl, applied) = apply_overrides(idl, &override_file)?;
    idl = modified_idl;

    // Log applied overrides
    for applied_override in applied {
        eprintln!(
            "Warning: Overriding {:?}{}",
            applied_override.override_type,
            if let Some(name) = &applied_override.entity_name {
                format!(" '{}'", name)
            } else {
                String::new()
            }
        );
        if let Some(original) = &applied_override.original_value {
            eprintln!("  IDL value: {}", original);
        }
        eprintln!("  Override value: {}", applied_override.override_value);
    }
}

// 5. Generate code with potentially modified IDL
generate_code(&cli.output, &cli.module_name, &idl)?;
```

---

## CLI Integration

### New CLI Flag

```rust
#[derive(Parser)]
struct Cli {
    // ... existing fields ...

    /// Path to override file (optional)
    #[arg(long)]
    override_file: Option<PathBuf>,
}
```

### Usage Examples

```bash
# Convention-based discovery (check ./overrides/raydium_amm.json)
solana-idl-codegen -i idl/raydium_amm/idl.json -o generated -m raydium_amm

# Explicit override file
solana-idl-codegen -i idl/raydium_amm/idl.json -o generated -m raydium_amm \
    --override-file custom/path/overrides.json

# No override file (existing behavior unchanged)
solana-idl-codegen -i idl/raydium_amm/idl.json -o generated -m raydium_amm
```

---

## Error Messages Contract

### Validation Errors

**Invalid Program Address**:
```
Error: Invalid program address: "not-a-valid-pubkey". Must be valid base58-encoded Pubkey.
```

**System Default Pubkey**:
```
Error: Invalid program address: "11111111111111111111111111111111". Cannot be system default pubkey.
```

**Invalid Discriminator Length**:
```
Error: Invalid discriminator for account 'PoolState': must be exactly 8 bytes
```

**All-Zero Discriminator**:
```
Error: Invalid discriminator for event 'SwapEvent': cannot be all zeros
```

**Empty Override File**:
```
Error: Empty override file: must contain at least one override
```

### Warning Messages

**Unknown Entity**:
```
Warning: Override file contains discriminator for account 'UnknownAccount' which does not exist in IDL 'raydium_amm'
  Override entry: accounts.UnknownAccount
  Available accounts: PoolState, AmmConfig, UserPosition
  Action: Remove unknown entry or verify IDL version matches
```

**Override Applied**:
```
Warning: Overriding program address
  IDL value: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
  Override value: HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8
```

### Conflict Error

```
Error: Multiple override files detected for 'raydium_amm' IDL:
  - ./overrides/raydium_amm.json (convention-based discovery)
  - /custom/path/overrides.json (--override-file argument)

Please remove one of the conflicting override files or use --override-file exclusively.
```

---

## Performance Contract

**Targets** (from Technical Context):
- Override file loading + validation: <10ms per file
- Total codegen overhead from overrides: <5% (<100ms → <105ms)
- Convention-based discovery: <50ms (max 2 file checks)

**Optimization Strategies**:
- Lazy discovery: only discover if needed
- Early exit: stop searching after first file found
- Zero-copy parsing where possible
- Minimal validation overhead (simple checks, no network calls)

---

## Backward Compatibility Contract

**Guarantees**:
1. Existing workflow unchanged (overrides are optional)
2. CLI preserves existing flags (only adds `--override-file`)
3. IDL parsing unchanged (overrides applied afterward)
4. Generated code format unchanged
5. No breaking changes to existing functions

**Version Policy**:
- Adding new override types (e.g., type overrides): MINOR version bump
- Changing override file schema: MAJOR version bump
- Adding optional fields with defaults: PATCH version bump

---

## Testing Contract

### Unit Test Coverage

**Required Tests** (in `src/override.rs`):
- `test_discover_override_file_convention_based()`
- `test_discover_override_file_explicit()`
- `test_discover_override_file_conflict()`
- `test_load_override_file_valid()`
- `test_load_override_file_invalid_json()`
- `test_validate_program_address_valid()`
- `test_validate_program_address_invalid()`
- `test_validate_discriminator_valid()`
- `test_validate_discriminator_all_zeros()`
- `test_apply_overrides_program_address()`
- `test_apply_overrides_account_discriminators()`
- `test_apply_overrides_unknown_entities()`

### Integration Test Coverage

**Required Tests** (in `tests/integration/override_tests.rs`):
- `test_override_missing_program_address()`
- `test_override_incorrect_discriminators()`
- `test_override_workflow_compiles()`
- `test_multiple_override_files_error()`
- `test_malformed_override_file_error()`

---

## Summary

This contract defines:

1. **Public API**: 4 core functions with clear signatures and behaviors
2. **Error handling**: Actionable error messages with file paths and remediation guidance
3. **CLI integration**: Single new flag `--override-file` with backward compatibility
4. **Performance**: Specific targets for overhead and discovery time
5. **Testing**: Comprehensive unit and integration test requirements

All contracts are implementable using existing Rust patterns and dependencies in the codebase.
