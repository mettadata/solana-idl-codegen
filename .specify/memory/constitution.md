<!--
============================================================================
SYNC IMPACT REPORT - Constitution Update
============================================================================
Version Change: 1.1.0 → 1.1.1 (PATCH - Clarification of existing requirements)

Modified Principles:
- Principle III: Clarified integration test requirement scope
- Principle VIII: Expanded to explicitly include integration testing requirement

Added Sections:
- None

Removed Sections:
- None

Templates Requiring Updates:
✅ plan-template.md - Already includes testing in Technical Context
✅ spec-template.md - Already supports testing in user stories/requirements
✅ tasks-template.md - Updated to mark both unit AND integration tests as MANDATORY

Follow-up TODOs:
- None - all placeholders resolved

Constitution Verification:
- No remaining unexplained bracket tokens
- Version follows semantic versioning (1.1.1 for clarification)
- Dates in ISO format (YYYY-MM-DD)
- All principles declarative and testable
- Clarifications strengthen existing requirements without adding new principles

Amendment Rationale:
- User requested: "also testing of the generated code via integration tests are also mandatory"
- PATCH version bump justified: Clarifying/strengthening existing requirements (Principles III, VIII)
- Integration tests were already mentioned but not emphasized enough
- Makes explicit what was implicit: both unit AND integration tests are mandatory
- Aligns with existing pre-merge requirement: `just test-integration` MUST pass
============================================================================
-->

# Solana IDL Code Generator Constitution

## Core Principles

### I. Generated Code is Gitignored

The `generated/` directory MUST be gitignored and rebuilt on-demand. Manual edits to generated code are STRICTLY FORBIDDEN. All changes MUST be made to IDL files or the codegen tool itself.

**Rationale**: Generated code is a build artifact, not source. Committing generated code creates maintenance burden, merge conflicts, and drift from IDL source of truth. Regeneration ensures consistency and reproducibility.

### II. Quality Gates are NON-NEGOTIABLE

All code MUST pass formatting (`cargo fmt --check`) and linting (`cargo clippy -- -D warnings`) checks. These are CI-enforced and MUST pass before merging.

**Rationale**: Code quality is foundational to maintainability and collaboration. Automated enforcement prevents quality degradation and reduces review burden. Warnings-as-errors prevents technical debt accumulation.

### III. Generated Code MUST Compile and Function Correctly

All generated Rust crates MUST compile successfully without errors or warnings. Integration tests MUST verify:
- Compilation of all generated outputs
- Correct serialization/deserialization with Borsh and Bytemuck
- Discriminator validation for accounts and events
- Type safety and proper derives
- Module structure and exports

**Rationale**: Broken generated code defeats the tool's purpose. Compilation verification catches type mapping errors, missing derives, and syntax issues. Integration tests verify the generated code works correctly in realistic usage scenarios, catching issues that unit tests alone cannot detect.

### IV. IDL Format Compatibility

The codegen MUST support both old and new IDL formats via optional fields and helper methods. Breaking changes to IDL support require MAJOR version bump.

**Rationale**: Solana ecosystem uses multiple IDL versions. Backward compatibility prevents breaking existing users. Clear versioning signals breaking changes.

### V. Type Safety First

Generated code MUST use strong typing with proper derives (Borsh, Bytemuck) and type-safe Pubkey handling. No unsafe code unless explicitly justified and documented.

**Rationale**: Blockchain data is untrusted. Type safety prevents deserialization vulnerabilities, memory corruption, and data misinterpretation. Safety first enables reliable on-chain data processing.

### VI. Discriminators are Mandatory

Account and event structs MUST generate discriminator constants and validation methods. Discriminators MUST be validated before deserialization.

**Rationale**: Solana accounts and logs are untyped byte arrays. Discriminators prevent data misinterpretation, enable safe deserialization, and are critical for blockchain data ingestion systems.

### VII. Module Organization for Readability

Generated crates MUST split code across 6 modules (lib.rs, types.rs, accounts.rs, instructions.rs, errors.rs, events.rs) to avoid monolithic files.

**Rationale**: Modular organization improves discoverability, reduces cognitive load, enables parallel comprehension, and follows Rust conventions.

### VIII. Comprehensive Testing is Mandatory

All codegen tool code (src/*.rs) and all generated code outputs MUST have comprehensive test coverage:

**Unit Tests (MANDATORY)**:
- Every codegen function, type mapping, code generation pattern, and edge case
- Fast (<1s total) to enable rapid iteration and TDD
- Located in src/*.rs test modules

**Integration Tests (MANDATORY)**:
- Verify all generated crates compile without errors or warnings
- Test serialization/deserialization round-trips for all types
- Validate discriminator checking for accounts and events
- Verify module structure and public API exports
- Test edge cases with real IDL files from production programs
- Comprehensive (~60s acceptable for full suite)
- Located in tests/integration/

**Rationale**: Code generation is complex with many edge cases and type combinations. Unit tests prevent regressions in the codegen tool itself. Integration tests verify the generated code works correctly in realistic scenarios, catching issues that unit tests alone cannot detect. Both are essential for reliability. Fast unit tests enable TDD and rapid iteration. Comprehensive integration tests ensure generated code meets user expectations.

## Quality Standards

### Code Generation Output

- **Formatting**: All generated code MUST pass `cargo fmt --check` and `cargo clippy`
- **Naming**: Snake_case for fields, PascalCase for types/enums via heck crate
- **Documentation**: Preserve IDL documentation in generated code
- **Performance**: Code generation MUST complete in <100ms per program for fast iteration

### Testing Requirements

- **Unit tests**: MANDATORY - Fast (<1s total) - comprehensive coverage of all codegen functions
- **Integration tests**: MANDATORY - Comprehensive verification of generated code correctness (~60s acceptable)
  - All generated crates MUST compile
  - Serialization/deserialization MUST work correctly
  - Discriminators MUST validate properly
  - Module structure MUST be correct
  - Public API MUST export correctly
- **Performance tests**: RECOMMENDED - Regression detection for codegen performance
- **Test organization**: Separate unit, integration, and performance tests clearly
- **Test coverage**: Unit tests for every code generation pattern, type mapping, and edge case; integration tests for every IDL format and production program

### Error Handling Standards

- **Tool code**: Use `anyhow::Result` with `.context()` for error chains
- **Generated code**: Use `std::io::Result` for discriminator validation
- **Errors MUST be actionable**: Include file paths, line numbers, and remediation guidance

## Development Workflow

### Pre-Commit Requirements (NON-NEGOTIABLE)

1. `just fmt-check` MUST pass (formatting)
2. `just clippy` MUST pass (linting with warnings-as-errors)
3. `just test` MUST pass (fast unit tests - all MUST pass)

### Pre-Merge Requirements (CI-ENFORCED)

1. All pre-commit checks MUST pass
2. `just generate` MUST complete successfully
3. `just check-generated` MUST pass (all generated crates compile)
4. `just test-integration` MUST pass (comprehensive verification of generated code)

### Workflow Sequence

1. Update IDL submodules (if needed): `git submodule update --remote`
2. Generate bindings: `just generate`
3. Make changes to codegen tool (src/*.rs)
4. **Write unit tests FIRST** (TDD recommended)
5. Run quality checks: `just fmt-check && just clippy`
6. Test changes: `just test` (fast unit tests), `just test-all` (unit + integration)
7. **Verify integration tests pass** before committing
8. Commit changes (generated/ not included)

### Adding New Features

When adding codegen features:

1. **Write unit tests FIRST** for new codegen functionality (TDD)
2. Update `idl.rs` for new IDL fields (`#[serde(default)]` for compatibility)
3. Implement in `codegen.rs` using `quote!` macros
4. **Verify unit tests pass** in `src/codegen.rs` tests
5. **Add integration test** verifying generated code compiles and functions correctly
6. **Run full test suite**: `just test-all` to verify both unit and integration tests pass
7. Update type mapping documentation if applicable
8. Verify against reference implementation (imported/pump_interface/)

## Governance

This constitution supersedes all other practices. Constitution compliance MUST be verified in all PR reviews.

### Amendment Procedure

1. **Proposal**: Document proposed changes with rationale and impact analysis
2. **Validation**: Verify alignment with project goals and user needs
3. **Template Sync**: Update all dependent templates and documentation
4. **Version Bump**: Follow semantic versioning (MAJOR/MINOR/PATCH)
5. **Migration Plan**: Document any breaking changes and migration path

### Version Bump Rules

- **MAJOR**: Backward incompatible principle removals or redefinitions (e.g., removing IDL format support)
- **MINOR**: New principle added or materially expanded guidance (e.g., mandatory unit testing)
- **PATCH**: Clarifications, wording refinements, strengthening existing requirements (e.g., explicit integration test requirement)

### Complexity Justification

Any violation of constitution principles MUST be explicitly justified in implementation plan with:

- **What principle is violated**: Specific principle name and why
- **Why needed**: Current business/technical requirement necessitating violation
- **Simpler alternative rejected**: What simpler approach was considered and why it's insufficient

### Compliance Review

All PRs MUST verify:

- ✅ Generated code is gitignored (Principle I)
- ✅ Quality gates pass (Principle II)
- ✅ Generated code compiles and functions correctly (Principle III)
- ✅ IDL compatibility maintained (Principle IV)
- ✅ Type safety preserved (Principle V)
- ✅ Discriminators implemented (Principle VI)
- ✅ Module organization followed (Principle VII)
- ✅ Unit tests written and passing (Principle VIII)
- ✅ Integration tests written and passing (Principle VIII)

### Runtime Guidance

Development workflow and operational guidance: `CLAUDE.md`

**Version**: 1.1.1 | **Ratified**: 2025-12-26 | **Last Amended**: 2025-12-26
