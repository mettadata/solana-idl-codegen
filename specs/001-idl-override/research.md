# Research: IDL Override System

**Feature**: IDL Override System | **Date**: 2025-12-26 | **Plan**: [plan.md](./plan.md)

## Overview

This document consolidates research findings for implementing an IDL override system that corrects missing or incorrect data in upstream IDL files without modifying the source files.

## Key Technical Decisions

### 1. Override File Format

**Decision**: JSON format with schema matching IDL structure

**Rationale**:
- Already a project dependency (serde_json for IDL parsing)
- Human-readable and git-friendly
- Native IDL format - familiar to users
- Strong typing via serde deserialization
- No additional dependencies required

**Alternatives Considered**:
- **TOML**: More human-friendly for config but adds new dependency (toml crate ~50KB)
- **YAML**: Popular for config but adds dependency (serde_yaml ~100KB) and more complex parsing
- **Custom DSL**: Maximum expressiveness but high complexity and unfamiliar syntax

**Schema Structure**:
```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "accounts": {
    "PoolState": {
      "discriminator": [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    }
  },
  "events": {
    "SwapEvent": {
      "discriminator": [0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]
    }
  },
  "instructions": {
    "initialize": {
      "discriminator": [0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28]
    }
  }
}
```

### 2. File Discovery Strategy

**Decision**: Convention-based discovery with explicit override

**Rationale**:
- Developer convenience: zero config for simple cases
- Predictable search order reduces surprises
- Explicit override for complex project structures
- Follows patterns from tools like TypeScript (tsconfig.json), ESLint (.eslintrc)

**Discovery Order**:
1. Explicit `--override-file <path>` CLI argument (highest priority)
2. `./overrides/<idl-name>.json` (per-IDL convention)
3. `./idl-overrides.json` (global fallback for single-IDL projects)
4. No override file found → proceed without overrides (not an error)

**Alternatives Considered**:
- **Require explicit path always**: Safest but adds friction for common case
- **Search multiple directories**: Most flexible but potentially confusing behavior
- **Config file with override mappings**: Adds indirection and another file to manage

### 3. Validation Strategy

**Decision**: Format validation + basic sanity checks (no network validation)

**Rationale**:
- Fast: offline operation, no network latency
- Deterministic: same input always produces same output
- Developer trust: assumes correct values from documentation/testing
- Catches common mistakes: all-zero discriminators, invalid base58

**Validation Rules**:
- **Program Address**: Valid base58 Pubkey, not system default (11111111111111111111111111111111)
- **Discriminators**: Exactly 8 bytes, not all zeros [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
- **Entity Names**: Must match existing entities in IDL (warn and ignore if not found)

**Alternatives Considered**:
- **Format only**: Faster but misses obvious errors (all-zero discriminators)
- **Network validation**: Query blockchain to verify addresses exist - requires network, slow, deployment-specific
- **Deep semantic validation**: Compare discriminator prefixes, check Anchor patterns - overly restrictive

### 4. Override Application Timing

**Decision**: Apply overrides immediately after IDL parsing, before code generation

**Rationale**:
- Clean separation: IDL parsing → override application → codegen
- Transparent to codegen: codegen sees corrected IDL, no special handling needed
- Testable: can verify corrected IDL structure independently
- Follows "pipeline" pattern: parse → transform → generate

**Implementation Approach**:
```rust
// In main.rs
let idl = parse_idl(&cli.input)?;
let idl = apply_overrides(idl, override_file_path)?; // <-- NEW
generate_code(&cli.output, &cli.module_name, &idl)?;
```

**Alternatives Considered**:
- **During codegen**: Codegen needs override awareness, more complex
- **Post-generation**: Too late, requires re-parsing generated code
- **Lazy application**: Apply overrides on-demand during codegen - harder to test and debug

### 5. Multiple Override File Handling

**Decision**: Fail with clear error when multiple override files detected

**Rationale**:
- Prevents accidental misconfigurations (e.g., both `overrides/raydium_amm.json` and explicit `--override-file`)
- Explicit is better than implicit (no silent merging)
- Clear error message guides user to resolution
- Simpler implementation (no merge logic needed)

**Error Message Format**:
```
Error: Multiple override files detected for 'raydium_amm' IDL:
  - ./overrides/raydium_amm.json (convention-based discovery)
  - /custom/path/overrides.json (--override-file argument)

Please remove one of the conflicting override files or use --override-file exclusively.
```

**Alternatives Considered**:
- **Last override wins**: Flexible but potentially surprising behavior
- **Merge overrides**: Complex merge logic, unclear precedence rules
- **First override wins**: Predictable but still confusing which file was chosen

### 6. Override File Scope

**Decision**: One IDL per override file (no multi-IDL override files)

**Rationale**:
- Clear ownership: each override file belongs to one IDL
- Easier version control: changes to one program don't affect others
- Simpler discovery: file name matches IDL name
- Reduced naming conflicts: separate namespaces per IDL

**Alternatives Considered**:
- **Single global override file**: Simpler for projects with many IDLs but harder to maintain and track changes
- **Support both**: Most flexible but adds complexity to discovery and validation logic

## Best Practices Research

### Error Handling Patterns

**Pattern**: Use `anyhow::Result` with `.context()` chains for actionable errors

**Examples from existing code**:
```rust
// src/main.rs
let idl_content = fs::read_to_string(&cli.input)
    .context(format!("Failed to read IDL file: {:?}", cli.input))?;
```

**Apply to overrides**:
```rust
let override_content = fs::read_to_string(&override_path)
    .context(format!("Failed to read override file: {:?}", override_path))?;

let override_data: OverrideFile = serde_json::from_str(&override_content)
    .context(format!("Failed to parse override file: {:?}", override_path))?;
```

### Logging and Warning Patterns

**Pattern**: Use clear, actionable warning messages with context

**Example warnings**:
```
Warning: Override file contains discriminator for account 'UnknownAccount' which does not exist in IDL 'raydium_amm'
  - Override entry: accounts.UnknownAccount
  - Available accounts: PoolState, AmmConfig, UserPosition
  - Action: Remove unknown entry or verify IDL version matches

Warning: Overriding program address for 'raydium_amm'
  - IDL address: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
  - Override address: HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8
  - Reason: IDL address points to devnet, override provides mainnet address
```

### Testing Strategy

**Unit Tests** (in `src/override.rs`):
- Override file parsing (valid JSON, invalid JSON, missing fields)
- Validation logic (invalid addresses, invalid discriminators, sanity checks)
- Discovery logic (file exists, file not found, multiple files)
- Override application (correct entities, unknown entities, empty overrides)

**Integration Tests** (in `tests/integration/override_tests.rs`):
- Full workflow: IDL + override → generated code compiles
- Missing program address corrected via override
- Incorrect discriminators corrected via override
- Warning messages generated for unknown entities
- Error messages for malformed override files
- Multiple override file detection

## Implementation Notes

### Performance Considerations

**File I/O Optimization**:
- Convention-based discovery: max 2 file checks (overrides/<name>.json, idl-overrides.json)
- Early exit on first file found (no unnecessary file system calls)
- Lazy discovery: only discover if IDL address/discriminators missing or `--override-file` provided

**Parsing Optimization**:
- Leverage existing serde_json parser (zero-copy where possible)
- Small override files (<10KB typical) → minimal parsing overhead
- Target: <10ms for override loading + validation

### Backward Compatibility

**No Breaking Changes**:
- Override files are optional (existing workflow unchanged)
- CLI preserves existing flags (only adds `--override-file`)
- IDL parsing unchanged (overrides applied afterward)
- Generated code format unchanged

**Versioning Considerations**:
- Override file schema can evolve independently of IDL schema
- Use `#[serde(default)]` for future optional override fields
- Document schema version in override file examples

## Open Questions (Resolved)

All technical decisions have been made during clarification phase:

1. ✅ **File format**: JSON (clarification Q3)
2. ✅ **Discovery strategy**: Convention-based with explicit override (clarification Q1)
3. ✅ **Multiple files**: Fail with error (clarification Q2)
4. ✅ **File scope**: One IDL per override file (clarification Q4)
5. ✅ **Validation depth**: Format + sanity checks (clarification Q5)

## References

### Existing Codebase Patterns

- **IDL Parsing**: `src/idl.rs` - dual format support with `#[serde(default)]`
- **CLI Argument Handling**: `src/main.rs` - clap derive patterns
- **Error Handling**: `src/main.rs` - anyhow with context chains
- **Validation**: `src/codegen.rs` - type mapping validation patterns

### External Tool Patterns

- **TypeScript**: `tsconfig.json` discovery (convention + explicit)
- **ESLint**: `.eslintrc` cascading resolution (similar to our override discovery)
- **Cargo**: `Cargo.toml` override patterns ([patch] sections)
- **Rust**: `cfg` attributes for conditional compilation (similar override concept)

## Summary

All technical unknowns have been resolved through clarification and research. The override system design is:

- **Simple**: JSON format, convention-based discovery, minimal new code
- **Safe**: Validation with clear errors, fail-fast on conflicts
- **Fast**: <10ms overhead, offline operation
- **Backward compatible**: Existing workflow unchanged, overrides are opt-in
- **Testable**: Clear separation of concerns, comprehensive test coverage planned

Ready to proceed to Phase 1 (Design & Contracts).
