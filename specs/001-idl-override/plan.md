# Implementation Plan: IDL Override System

**Branch**: `001-idl-override` | **Date**: 2025-12-26 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-idl-override/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add an IDL override system that allows developers to correct missing or incorrect program addresses and discriminators in upstream IDL files without modifying the source files. Overrides are loaded from external JSON files using convention-based discovery or explicit CLI arguments, validated for format and basic sanity, and applied during IDL parsing before code generation. This enables safe correction of IDL data mismatches with on-chain deployed programs while keeping upstream IDL files (in git submodules) unmodified.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.70.0 (stable toolchain)
**Primary Dependencies**:
- Existing: serde 1.0, serde_json 1.0, clap (CLI parsing), solana-program 1.18 (Pubkey validation), anyhow (error handling)
- No new dependencies required (JSON parsing already available via serde_json)

**Storage**: File system (JSON override files), no database required
**Testing**:
- Unit tests: cargo test (fast <1s, comprehensive codegen coverage)
- Integration tests: cargo test --test integration_tests (verify generated code compiles, ~60s acceptable)
- New: integration tests for override file loading, validation, and application

**Target Platform**: Cross-platform CLI tool (Linux, macOS, Windows)
**Project Type**: Single project (CLI code generation tool)
**Performance Goals**:
- Override file loading and validation: <10ms per file
- Total codegen overhead from overrides: <5% (<100ms → <105ms)
- Convention-based file discovery: <50ms

**Constraints**:
- No network access required (offline operation)
- Override files must be local and version-controllable
- Must maintain backward compatibility with existing IDL parsing
- Generated code quality gates unchanged (fmt, clippy, compilation)

**Scale/Scope**:
- Typical override file: 1-20 overrides per IDL
- Max override file size: ~10KB JSON (100+ overrides)
- Support 5-10 IDL files per project with overrides

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Principle I - Generated Code is Gitignored**: ✅ PASS
- Override system modifies IDL parsing, not generated code tracking
- Override files are version-controlled source code, not generated artifacts

**Principle II - Quality Gates are NON-NEGOTIABLE**: ✅ PASS
- All override system code will pass `cargo fmt --check` and `cargo clippy -- -D warnings`
- No quality gate exceptions required

**Principle III - Generated Code MUST Compile and Function Correctly**: ✅ PASS
- Override system enhances IDL data before codegen, improving generated code correctness
- Integration tests will verify generated code with overrides compiles successfully
- Existing discriminator validation logic benefits from corrected discriminators

**Principle IV - IDL Format Compatibility**: ✅ PASS
- Override system preserves existing IDL format compatibility
- Overrides augment IDL data without changing IDL format support
- Backward compatible: codegen works with or without override files

**Principle V - Type Safety First**: ✅ PASS
- Override validation ensures type-safe Pubkey parsing (base58)
- Discriminators validated as 8-byte arrays before application
- No unsafe code required

**Principle VI - Discriminators are Mandatory**: ✅ PASS
- Override system enables correction of incorrect discriminators
- Enhances discriminator accuracy for on-chain data matching

**Principle VII - Module Organization for Readability**: ✅ PASS
- Override logic contained in new `src/override.rs` module
- Clear separation from existing IDL parsing and codegen logic

**Principle VIII - Comprehensive Testing is Mandatory**: ✅ PASS
- Unit tests for override file parsing, validation, and application logic
- Integration tests for override + codegen workflow
- Test coverage: override discovery, validation errors, override application

**Conclusion**: All constitution principles satisfied. No violations require justification.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── main.rs           # CLI entry point (updated: add --override-file flag)
├── idl.rs            # IDL data structures (updated: apply overrides)
├── codegen.rs        # Code generation engine (unchanged)
└── override.rs       # NEW: Override file parsing, validation, discovery

tests/
├── integration/
│   ├── override_tests.rs          # NEW: Override + codegen workflow tests
│   └── fixtures/
│       ├── test_idl.json          # Test IDL with missing/incorrect data
│       └── test_override.json     # Test override file
└── unit/ (within src/*.rs test modules)
    └── override.rs tests          # NEW: Unit tests for override logic

# Override files (user-created, not in repo except as examples)
overrides/
├── raydium_amm.json    # Example: Override for raydium_amm IDL
└── pumpfun.json        # Example: Override for pumpfun IDL

# Documentation
docs/
└── override-format.md  # NEW: Override file format documentation
```

**Structure Decision**: Single project (CLI tool). New `src/override.rs` module contains all override logic. Integration tests verify override + codegen workflow. Override files are created by users in `overrides/` directory or custom locations.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No constitution violations. No complexity justifications required.

---

## Phase 0: Research - COMPLETE ✅

**Artifacts Generated**:
- [research.md](./research.md) - Technical decisions and best practices research

**Key Decisions**:
1. JSON format for override files (already a dependency)
2. Convention-based discovery with explicit override
3. Format + sanity validation (no network calls)
4. Apply overrides after IDL parsing, before codegen
5. Fail on multiple override files (no silent merging)
6. One IDL per override file (clear ownership)

**Outcome**: All technical unknowns resolved. Ready for implementation.

---

## Phase 1: Design & Contracts - COMPLETE ✅

**Artifacts Generated**:
- [data-model.md](./data-model.md) - Data structures and validation rules
- [contracts/override-api.md](./contracts/override-api.md) - Public API contract
- [quickstart.md](./quickstart.md) - User-facing quick start guide

**Design Highlights**:
- **OverrideFile**: Root structure with optional program_address and discriminator maps
- **DiscriminatorOverride**: 8-byte array wrapper with validation
- **OverrideDiscovery**: Result enum for discovery process (Found/NotFound/Conflict)
- **4 Core Functions**: discover, load, validate, apply
- **Clear Error Messages**: Actionable errors with file paths and remediation

**Agent Context Updated**: ✅ CLAUDE.md updated with new technologies

---

## Re-evaluated Constitution Check - PASS ✅

All constitution principles remain satisfied after detailed design:

**Principle I** ✅: Override files are version-controlled source code, not generated artifacts
**Principle II** ✅: All code passes fmt and clippy with warnings-as-errors
**Principle III** ✅: Enhanced - corrected discriminators improve generated code correctness
**Principle IV** ✅: Backward compatible - works with or without overrides
**Principle V** ✅: Type-safe Pubkey parsing and discriminator validation
**Principle VI** ✅: Enhanced - enables correction of incorrect discriminators
**Principle VII** ✅: New `src/override.rs` module with clear separation
**Principle VIII** ✅: Comprehensive unit and integration tests planned

---

## Next Steps

**Phase 2: Task Breakdown** - Run `/speckit.tasks` command to generate detailed implementation tasks from this plan.

**Expected Task Categories**:
1. Core implementation (src/override.rs module)
2. CLI integration (main.rs updates)
3. IDL integration (idl.rs modifications)
4. Unit tests (override module tests)
5. Integration tests (override + codegen workflow)
6. Documentation (override format docs, examples)

**Estimated Complexity**: Medium
- New module: ~400-600 LOC
- Tests: ~600-800 LOC
- Clear requirements and design
- No complex algorithms or external dependencies
