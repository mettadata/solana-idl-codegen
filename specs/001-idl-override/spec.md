# Feature Specification: IDL Override System

**Feature Branch**: `001-idl-override`
**Created**: 2025-12-26
**Status**: Draft
**Input**: User description: "Some IDL files don't come with an 'address' or the 'address' is incorrect such as idl/raydium-idl/raydium_amm/idl.json (missing). I need a way to not edit the IDL files but somehow add an address so when the codegen runs my changes are added into the generated code. A bit like a monkey patch or a post IDL process before codegen. Some of the discriminator listed in the IDL files are also incorrect when matched against the deployed mainNet version. Same as before I need a way to have a list of changes that then get applied as part of the codegen process."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Override Missing Program Addresses (Priority: P1)

A developer using the IDL codegen tool discovers that an upstream IDL file (e.g., raydium_amm/idl.json) is missing the program address field entirely. Without modifying the upstream IDL file (which is maintained in a git submodule), they need to provide the correct program address so the generated code includes it.

**Why this priority**: Without program addresses, generated code cannot interact with on-chain programs. This is a critical blocker for any blockchain integration work.

**Independent Test**: Can be fully tested by providing an override file with a program address, running codegen, and verifying the generated code contains the correct address constant.

**Acceptance Scenarios**:

1. **Given** an IDL file without a program address field, **When** developer creates an override file specifying the address, **Then** the generated code includes the correct program address constant
2. **Given** an override file with a program address, **When** codegen runs, **Then** the generated Rust code compiles successfully and references the correct on-chain program
3. **Given** multiple IDL files needing addresses, **When** override file specifies addresses for each, **Then** all generated modules include their respective addresses

---

### User Story 2 - Override Incorrect Program Addresses (Priority: P2)

A developer identifies that an upstream IDL file contains an incorrect program address (pointing to devnet instead of mainnet, or an outdated program deployment). They need to override this incorrect value without editing the source IDL file.

**Why this priority**: Incorrect addresses cause runtime failures when interacting with blockchain programs. While less critical than missing addresses (which prevent compilation), this still blocks production deployments.

**Independent Test**: Can be tested by providing an override for a known incorrect address, verifying the generated code uses the override value instead of the IDL's original value.

**Acceptance Scenarios**:

1. **Given** an IDL file with an incorrect program address, **When** developer specifies correct address in override file, **Then** generated code uses the override value instead of IDL value
2. **Given** an override address and an IDL address, **When** codegen runs, **Then** a warning is logged showing the address was overridden
3. **Given** the same program address in both IDL and override, **When** codegen runs, **Then** no override is applied and no warning is logged

---

### User Story 3 - Override Incorrect Account Discriminators (Priority: P1)

A developer discovers that account discriminators in an IDL file don't match the actual on-chain program's discriminators (due to IDL being out of sync with deployed code). When attempting to decode account data from mainnet, deserialization fails. They need to override the discriminators without modifying the upstream IDL.

**Why this priority**: Discriminator mismatches cause silent data corruption or deserialization failures. This is critical for data ingestion systems that decode on-chain accounts.

**Independent Test**: Can be tested by providing override discriminators, running codegen, and verifying the generated discriminator constants match the override values.

**Acceptance Scenarios**:

1. **Given** an IDL file with incorrect account discriminators, **When** developer specifies correct discriminators in override file, **Then** generated code uses override discriminator values
2. **Given** override discriminators for multiple accounts, **When** codegen runs, **Then** each account struct includes the correct discriminator constant
3. **Given** a discriminator override, **When** attempting to deserialize mainnet account data, **Then** deserialization succeeds using the corrected discriminator

---

### User Story 4 - Override Incorrect Event Discriminators (Priority: P2)

A developer working with program logs discovers that event discriminators in the IDL don't match the actual on-chain program's event discriminators. Event parsing from transaction logs fails. They need to override event discriminators to match the deployed program.

**Why this priority**: Event discriminator mismatches prevent parsing transaction logs, breaking event-driven systems. While important, it's slightly less critical than account discriminators since events are often used for notifications rather than critical data processing.

**Independent Test**: Can be tested by providing event discriminator overrides, running codegen, and verifying generated event structs include corrected discriminators.

**Acceptance Scenarios**:

1. **Given** an IDL with incorrect event discriminators, **When** developer specifies correct discriminators in override file, **Then** generated event structs use override values
2. **Given** event discriminator overrides, **When** parsing transaction logs from mainnet, **Then** events are correctly identified and deserialized
3. **Given** multiple events with discriminator overrides, **When** codegen runs, **Then** all event discriminators are correctly applied

---

### User Story 5 - Override Instruction Discriminators (Priority: P3)

A developer needs to override instruction discriminators that don't match the deployed program's actual instruction encoding.

**Why this priority**: While instruction discriminators are important for transaction construction, they are less commonly needed than account/event discriminators since most users consume data rather than create transactions. This is valuable but lower priority.

**Independent Test**: Can be tested by providing instruction discriminator overrides and verifying generated instruction enums include corrected discriminator values.

**Acceptance Scenarios**:

1. **Given** an IDL with incorrect instruction discriminators, **When** developer specifies correct discriminators in override file, **Then** generated instruction enum uses override values
2. **Given** instruction discriminator overrides, **When** constructing transactions, **Then** instructions encode with correct discriminator values

---

### Edge Cases

- What happens when an override file specifies an address for an IDL that already has an address? (Answer: Override takes precedence, warning logged)
- What happens when override file contains discriminators for accounts/events that don't exist in the IDL? (Answer: Warning logged, override ignored)
- What happens when override file is malformed or contains invalid data? (Answer: Clear error message, codegen fails fast)
- What happens when multiple override files are detected for the same IDL? (Answer: Fail with clear error message identifying the conflicting override files and requiring user to resolve the conflict)
- What happens when an override file exists but is empty? (Answer: No overrides applied, codegen proceeds normally)
- What happens when discriminator values in override are not 8-byte arrays? (Answer: Validation error with clear message)
- What happens when override file is updated after code generation? (Answer: Generated code remains unchanged until codegen is re-run)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support loading override configuration from external files without modifying source IDL files
- **FR-002**: System MUST support overriding missing program addresses in IDL files
- **FR-003**: System MUST support overriding incorrect program addresses in IDL files
- **FR-004**: System MUST support overriding account discriminators specified in IDL files
- **FR-005**: System MUST support overriding event discriminators specified in IDL files
- **FR-006**: System MUST support overriding instruction discriminators specified in IDL files
- **FR-007**: System MUST validate override data before applying overrides: format validation (addresses as valid base58 pubkeys, discriminators as 8-byte arrays) and basic sanity checks (discriminators not all zeros, addresses not system default pubkey)
- **FR-008**: System MUST log warnings when override values differ from IDL values (showing both original and override)
- **FR-009**: System MUST fail fast with clear error messages when override files are malformed
- **FR-010**: System MUST apply overrides before code generation, not after
- **FR-011**: System MUST support multiple discriminator overrides per IDL file (accounts, events, instructions)
- **FR-012**: Override files MUST be in JSON format for version control and human readability
- **FR-013**: System MUST allow selective overrides (override only specific accounts/events without affecting others)
- **FR-014**: Override file location MUST be configurable (via command-line argument); when not specified, system MUST use convention-based discovery (check `./overrides/<idl-name>.json` then `./idl-overrides.json` in that order)
- **FR-015**: System MUST ignore override entries for entities that don't exist in the IDL (with warning)
- **FR-016**: System MUST fail with clear error when multiple override files are detected for the same IDL, identifying all conflicting files
- **FR-017**: Each override file MUST contain overrides for exactly one IDL file (no multi-IDL override files)

### Key Entities

- **Override File**: Configuration file containing corrections for a single IDL file; includes program address overrides, account discriminator overrides, event discriminator overrides, instruction discriminator overrides; stored in version control alongside or separate from IDL files; one override file per IDL

- **Program Address Override**: Specifies the correct Solana program address (Pubkey) for an IDL file that is missing or has an incorrect address; applied during IDL parsing before code generation

- **Discriminator Override**: Specifies the correct 8-byte discriminator value for accounts, events, or instructions; organized by entity type and name; applied to matching entities during code generation

- **Override Application Context**: Represents the state during codegen where IDL data is merged with override data; includes validation logic, conflict detection, and logging of applied overrides

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Developers can correct missing or incorrect IDL data without editing upstream IDL files stored in git submodules
- **SC-002**: Generated code compiles successfully with overridden addresses and discriminators
- **SC-003**: Account deserialization succeeds when using overridden discriminators that match mainnet program behavior
- **SC-004**: Event parsing from transaction logs succeeds when using overridden event discriminators
- **SC-005**: Override application process completes in under 100ms for typical IDL files (less than 5% overhead)
- **SC-006**: Malformed override files produce clear error messages within 1 second of detection
- **SC-007**: Developers can maintain override files in version control alongside project code
- **SC-008**: Changes to override files are reflected in generated code upon re-running codegen (100% consistency)

## Scope *(mandatory)*

### In Scope

- Loading override configuration from external JSON files
- Overriding program addresses (missing or incorrect)
- Overriding account discriminators
- Overriding event discriminators
- Overriding instruction discriminators
- Validation of override data format and values
- Logging warnings for applied overrides
- Error handling for malformed override files
- Command-line interface for specifying override file location
- Documentation for override file format and usage

### Out of Scope

- Automatically detecting incorrect discriminators by querying mainnet programs
- GUI or interactive tools for creating override files
- Overriding other IDL fields (types, instruction arguments, etc.) - only addresses and discriminators
- Merging multiple override files automatically (only single override file per codegen invocation)
- Override file schema migration or versioning
- Cloud storage or remote override file fetching

## Assumptions *(mandatory)*

- Developers have knowledge of correct program addresses and discriminators (obtained through documentation, on-chain inspection, or testing)
- Override files are maintained manually by developers
- IDL files remain in git submodules and are not edited directly
- Codegen is re-run after override file changes (no automatic hot-reload)
- Override files use JSON format (human-readable and git-friendly, matches IDL format)
- Each override file contains overrides for exactly one IDL file (no multi-IDL override files)
- Single override file per IDL file is sufficient (no complex merge scenarios)
- Override files are stored locally in the project (not fetched from remote sources)
- Discriminator values are provided as hex strings or byte arrays in override files
- Program addresses are provided as base58-encoded Pubkey strings

## Clarifications

### Session 2025-12-26

- Q: How should the system locate override files when the user doesn't explicitly specify a path? → A: Convention-based discovery (e.g., check `./overrides/<idl-name>.json` then `./idl-overrides.json`) with explicit path override - balances convenience and explicitness
- Q: What should happen when multiple override files are detected/specified for the same IDL file? → A: Fail with clear error requiring user to resolve conflict - safest, most explicit
- Q: Which file format(s) should the override system support? → A: JSON only - simplest, already required dependency, matches IDL format
- Q: Should override files support overrides for multiple IDL files or just one IDL per file? → A: One IDL per override file - clearer ownership, easier to version control and maintain
- Q: What level of validation should be performed on override values beyond format checking? → A: Format + basic sanity (e.g., discriminator not all zeros) - catches obvious mistakes

## Dependencies *(include if feature requires external systems)*

- Existing IDL parsing logic in `src/idl.rs`
- Existing code generation engine in `src/codegen.rs`
- Serde JSON for override file parsing (already required for IDL parsing)
- Solana SDK for Pubkey validation
- Command-line argument parsing (clap)
