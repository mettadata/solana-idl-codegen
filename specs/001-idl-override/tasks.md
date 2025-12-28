# Tasks: IDL Override System

**Input**: Design documents from `/specs/001-idl-override/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Per Constitution Principles III and VIII:
- **Unit tests for codegen tool code (src/*.rs)**: MANDATORY
- **Integration tests for generated code**: MANDATORY
- **Contract/E2E tests**: Not applicable (CLI tool)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Override system is entirely within existing solana-idl-codegen CLI tool

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure for override system

- [X] T001 Create `src/override.rs` module file with module declaration in `src/main.rs`
- [X] T002 [P] Create `tests/integration/override_tests.rs` file for integration tests
- [X] T003 [P] Create `tests/integration/fixtures/` directory for test IDL and override files
- [X] T004 [P] Create example override files in `overrides/` directory (gitignored except examples)
- [X] T005 [P] Create `docs/override-format.md` documentation file

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data structures and utilities that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Implement `OverrideFile` struct with serde derives in `src/override.rs`
- [X] T007 [P] Implement `DiscriminatorOverride` struct with validation in `src/override.rs`
- [X] T008 [P] Implement `OverrideDiscovery` enum (Found/NotFound/Conflict) in `src/override.rs`
- [X] T009 [P] Implement `ValidationError` enum with thiserror derives in `src/override.rs`
- [X] T010 [P] Implement `AppliedOverride` tracking struct in `src/override.rs`
- [X] T011 Add `--override-file` CLI flag to `Cli` struct in `src/main.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Override Missing Program Addresses (Priority: P1) 🎯 MVP

**Goal**: Enable developers to add missing program addresses to IDL files via override files

**Independent Test**: Create IDL without program address, provide override file with address, run codegen, verify generated code includes correct `PROGRAM_ID` constant

### Unit Tests for User Story 1 (MANDATORY)

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T012 [P] [US1] Unit test for `discover_override_file` with missing file in `src/override.rs` tests module
- [X] T013 [P] [US1] Unit test for `discover_override_file` with found file in `src/override.rs` tests module
- [X] T014 [P] [US1] Unit test for `load_override_file` with valid JSON in `src/override.rs` tests module
- [X] T015 [P] [US1] Unit test for `load_override_file` with invalid JSON error in `src/override.rs` tests module
- [X] T016 [P] [US1] Unit test for `validate_override_file` with valid program address in `src/override.rs` tests module
- [X] T017 [P] [US1] Unit test for `validate_override_file` with invalid base58 address in `src/override.rs` tests module
- [X] T018 [P] [US1] Unit test for `validate_override_file` with system default pubkey error in `src/override.rs` tests module

### Integration Tests for User Story 1 (MANDATORY)

- [X] T019 [P] [US1] Integration test: IDL with missing address + override file → generated code compiles in `tests/override_tests.rs`
- [X] T020 [P] [US1] Integration test: verify `PROGRAM_ID` constant matches override value in generated `lib.rs` in `tests/override_tests.rs`
- [X] T021 [P] [US1] Create test fixture: IDL without program address in `tests/integration/fixtures/test_missing_address.json`
- [X] T022 [P] [US1] Create test fixture: override file with program address in `tests/integration/fixtures/test_address_override.json`

### Implementation for User Story 1

- [X] T023 [P] [US1] Implement `discover_override_file()` function with convention-based search in `src/override.rs`
- [X] T024 [P] [US1] Implement `load_override_file()` function with JSON parsing in `src/override.rs`
- [X] T025 [US1] Implement `validate_override_file()` for program address validation in `src/override.rs`
- [X] T026 [US1] Implement `apply_overrides()` for program address override in `src/override.rs`
- [X] T027 [US1] Integrate override discovery into `main.rs` workflow before IDL parsing
- [X] T028 [US1] Add warning logging for applied program address overrides in `main.rs`
- [X] T029 [US1] Verify all unit tests pass: `cargo test --lib override`
- [X] T030 [US1] Verify integration test passes: `cargo test --test override_tests test_missing_address`

**Checkpoint**: At this point, program address overrides should be fully functional and testable independently

---

## Phase 4: User Story 2 - Override Incorrect Program Addresses (Priority: P2)

**Goal**: Enable developers to correct incorrect program addresses (e.g., devnet vs mainnet) in IDL files

**Independent Test**: Create IDL with incorrect program address, provide override with correct address, run codegen, verify generated code uses override value and logs warning

### Unit Tests for User Story 2 (MANDATORY)

- [ ] T031 [P] [US2] Unit test for override with conflicting address (IDL has address, override provides different one) in `src/override.rs` tests module
- [ ] T032 [P] [US2] Unit test for override with same address (no-op case) in `src/override.rs` tests module
- [ ] T033 [P] [US2] Unit test for warning message generation when overriding existing address in `src/override.rs` tests module

### Integration Tests for User Story 2 (MANDATORY)

- [ ] T034 [P] [US2] Integration test: IDL with incorrect address + override → generated code uses override value in `tests/integration/override_tests.rs`
- [ ] T035 [P] [US2] Integration test: verify warning logged showing original vs override address in `tests/integration/override_tests.rs`
- [ ] T036 [P] [US2] Create test fixture: IDL with incorrect program address in `tests/integration/fixtures/test_incorrect_address.json`
- [ ] T037 [P] [US2] Create test fixture: override file correcting address in `tests/integration/fixtures/test_address_correction.json`

### Implementation for User Story 2

- [ ] T038 [US2] Enhance `apply_overrides()` to handle existing program address replacement in `src/override.rs`
- [ ] T039 [US2] Add warning detection logic (override differs from IDL) in `src/override.rs`
- [ ] T040 [US2] Implement warning message formatting showing old → new values in `src/override.rs`
- [ ] T041 [US2] Update `AppliedOverride` tracking to capture original values in `src/override.rs`
- [ ] T042 [US2] Verify all unit tests pass: `cargo test --lib override`
- [ ] T043 [US2] Verify integration tests pass: `cargo test --test override_tests test_incorrect_address`

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently (missing and incorrect addresses)

---

## Phase 5: User Story 3 - Override Incorrect Account Discriminators (Priority: P1)

**Goal**: Enable developers to correct account discriminators that don't match on-chain deployed programs

**Independent Test**: Create IDL with incorrect account discriminators, provide override with correct discriminators, run codegen, verify generated account structs use corrected discriminator constants

### Unit Tests for User Story 3 (MANDATORY)

- [ ] T044 [P] [US3] Unit test for `DiscriminatorOverride` parsing from JSON in `src/override.rs` tests module
- [ ] T045 [P] [US3] Unit test for discriminator validation (exactly 8 bytes) in `src/override.rs` tests module
- [ ] T046 [P] [US3] Unit test for discriminator validation (not all zeros) in `src/override.rs` tests module
- [ ] T047 [P] [US3] Unit test for account discriminator override application in `src/override.rs` tests module
- [ ] T048 [P] [US3] Unit test for unknown account name warning in `src/override.rs` tests module

### Integration Tests for User Story 3 (MANDATORY)

- [ ] T049 [P] [US3] Integration test: IDL with incorrect account discriminators + override → generated code compiles in `tests/integration/override_tests.rs`
- [ ] T050 [P] [US3] Integration test: verify account struct `DISCRIMINATOR` constant matches override in `tests/integration/override_tests.rs`
- [ ] T051 [P] [US3] Integration test: deserialization with corrected discriminator succeeds in `tests/integration/override_tests.rs`
- [ ] T052 [P] [US3] Create test fixture: IDL with incorrect account discriminators in `tests/integration/fixtures/test_account_disc.json`
- [ ] T053 [P] [US3] Create test fixture: override file with corrected account discriminators in `tests/integration/fixtures/test_account_override.json`

### Implementation for User Story 3

- [ ] T054 [P] [US3] Implement discriminator format validation (8-byte check) in `src/override.rs`
- [ ] T055 [P] [US3] Implement discriminator sanity validation (not all zeros) in `src/override.rs`
- [ ] T056 [US3] Enhance `validate_override_file()` to check account names exist in IDL in `src/override.rs`
- [ ] T057 [US3] Enhance `apply_overrides()` to apply account discriminator overrides in `src/override.rs`
- [ ] T058 [US3] Add warning for unknown account names in override file in `src/override.rs`
- [ ] T059 [US3] Update IDL account discriminator fields with override values in `src/override.rs`
- [ ] T060 [US3] Verify all unit tests pass: `cargo test --lib override`
- [ ] T061 [US3] Verify integration tests pass: `cargo test --test override_tests test_account_disc`

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently

---

## Phase 6: User Story 4 - Override Incorrect Event Discriminators (Priority: P2)

**Goal**: Enable developers to correct event discriminators for parsing transaction logs

**Independent Test**: Create IDL with incorrect event discriminators, provide override with correct discriminators, run codegen, verify generated event structs use corrected discriminators

### Unit Tests for User Story 4 (MANDATORY)

- [ ] T062 [P] [US4] Unit test for event discriminator override application in `src/override.rs` tests module
- [ ] T063 [P] [US4] Unit test for unknown event name warning in `src/override.rs` tests module
- [ ] T064 [P] [US4] Unit test for multiple event overrides in same file in `src/override.rs` tests module

### Integration Tests for User Story 4 (MANDATORY)

- [ ] T065 [P] [US4] Integration test: IDL with incorrect event discriminators + override → generated code compiles in `tests/integration/override_tests.rs`
- [ ] T066 [P] [US4] Integration test: verify event struct `DISCRIMINATOR` constant matches override in `tests/integration/override_tests.rs`
- [ ] T067 [P] [US4] Create test fixture: IDL with incorrect event discriminators in `tests/integration/fixtures/test_event_disc.json`
- [ ] T068 [P] [US4] Create test fixture: override file with corrected event discriminators in `tests/integration/fixtures/test_event_override.json`

### Implementation for User Story 4

- [ ] T069 [US4] Enhance `validate_override_file()` to check event names exist in IDL in `src/override.rs`
- [ ] T070 [US4] Enhance `apply_overrides()` to apply event discriminator overrides in `src/override.rs`
- [ ] T071 [US4] Add warning for unknown event names in override file in `src/override.rs`
- [ ] T072 [US4] Update IDL event discriminator fields with override values in `src/override.rs`
- [ ] T073 [US4] Verify all unit tests pass: `cargo test --lib override`
- [ ] T074 [US4] Verify integration tests pass: `cargo test --test override_tests test_event_disc`

**Checkpoint**: User Stories 1-4 should all work independently (program addresses, account discriminators, event discriminators)

---

## Phase 7: User Story 5 - Override Instruction Discriminators (Priority: P3)

**Goal**: Enable developers to correct instruction discriminators for transaction construction

**Independent Test**: Create IDL with incorrect instruction discriminators, provide override with correct discriminators, run codegen, verify generated instruction enums use corrected discriminators

### Unit Tests for User Story 5 (MANDATORY)

- [ ] T075 [P] [US5] Unit test for instruction discriminator override application in `src/override.rs` tests module
- [ ] T076 [P] [US5] Unit test for unknown instruction name warning in `src/override.rs` tests module

### Integration Tests for User Story 5 (MANDATORY)

- [ ] T077 [P] [US5] Integration test: IDL with incorrect instruction discriminators + override → generated code compiles in `tests/integration/override_tests.rs`
- [ ] T078 [P] [US5] Integration test: verify instruction enum discriminator matches override in `tests/integration/override_tests.rs`
- [ ] T079 [P] [US5] Create test fixture: IDL with incorrect instruction discriminators in `tests/integration/fixtures/test_instruction_disc.json`
- [ ] T080 [P] [US5] Create test fixture: override file with corrected instruction discriminators in `tests/integration/fixtures/test_instruction_override.json`

### Implementation for User Story 5

- [ ] T081 [US5] Enhance `validate_override_file()` to check instruction names exist in IDL in `src/override.rs`
- [ ] T082 [US5] Enhance `apply_overrides()` to apply instruction discriminator overrides in `src/override.rs`
- [ ] T083 [US5] Add warning for unknown instruction names in override file in `src/override.rs`
- [ ] T084 [US5] Update IDL instruction discriminator fields with override values in `src/override.rs`
- [ ] T085 [US5] Verify all unit tests pass: `cargo test --lib override`
- [ ] T086 [US5] Verify integration tests pass: `cargo test --test override_tests test_instruction_disc`

**Checkpoint**: All 5 user stories should now be independently functional

---

## Phase 8: Edge Cases & Error Handling

**Purpose**: Comprehensive error handling and edge case coverage

### Unit Tests (MANDATORY)

- [ ] T087 [P] Unit test for multiple override files detected (Conflict error) in `src/override.rs` tests module
- [ ] T088 [P] Unit test for empty override file (EmptyOverrideFile error) in `src/override.rs` tests module
- [ ] T089 [P] Unit test for malformed JSON error handling in `src/override.rs` tests module
- [ ] T090 [P] Unit test for file not found error handling in `src/override.rs` tests module

### Integration Tests (MANDATORY)

- [ ] T091 [P] Integration test: multiple override files detected error in `tests/integration/override_tests.rs`
- [ ] T092 [P] Integration test: malformed override file fails gracefully in `tests/integration/override_tests.rs`
- [ ] T093 [P] Integration test: empty override file error in `tests/integration/override_tests.rs`

### Implementation

- [ ] T094 Implement multiple override file detection in `discover_override_file()` in `src/override.rs`
- [ ] T095 [P] Implement empty override file validation in `validate_override_file()` in `src/override.rs`
- [ ] T096 [P] Add comprehensive error context to all file operations in `src/override.rs`
- [ ] T097 [P] Implement conflict error message formatting in `src/override.rs`
- [ ] T098 Verify all edge case tests pass: `cargo test --lib override`

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories, documentation, and final validation

- [ ] T099 [P] Write override file format documentation in `docs/override-format.md`
- [ ] T100 [P] Add override system usage examples to main `README.md`
- [ ] T101 [P] Create example override files for common scenarios in `overrides/` directory
- [ ] T102 [P] Update `CLAUDE.md` with override system documentation
- [ ] T103 [P] Add override file schema documentation to `docs/override-format.md`
- [ ] T104 Code cleanup and refactoring in `src/override.rs` (remove dead code, improve naming)
- [ ] T105 [P] Run `cargo fmt` and `cargo clippy -- -D warnings` on all code
- [ ] T106 [P] Verify comprehensive unit test coverage (MANDATORY - Constitution Principle VIII): `cargo tarpaulin`
- [ ] T107 [P] Verify comprehensive integration test coverage (MANDATORY - Constitution Principles III & VIII): `just test-integration`
- [ ] T108 [P] Run quickstart.md validation (test all examples work): `bash test-quickstart.sh`
- [ ] T109 [P] Performance validation: verify override overhead <5% in benchmarks
- [ ] T110 Run complete test suite: `just test-all` (unit + integration)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order: US1/US3 (P1) → US2/US4 (P2) → US5 (P3)
- **Edge Cases (Phase 8)**: Can start after Foundational, parallel with user stories
- **Polish (Phase 9)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Extends US1 but independently testable
- **User Story 3 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - Parallel to US3, same pattern
- **User Story 5 (P3)**: Can start after Foundational (Phase 2) - Parallel to US3/US4, same pattern

### Within Each User Story

- Unit tests MUST be written and FAIL before implementation
- Integration tests MUST be written and FAIL before implementation
- Core implementation before integration with main workflow
- All tests MUST pass before story marked complete

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T002-T005)
- All Foundational tasks marked [P] can run in parallel within Phase 2 (T007-T010)
- Once Foundational phase completes, user stories US1/US3 (P1) can start in parallel
- All unit tests for a user story marked [P] can run in parallel
- All integration tests for a user story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Launch all unit tests for User Story 1 together:
Task: "Unit test for discover_override_file with missing file in src/override.rs"
Task: "Unit test for discover_override_file with found file in src/override.rs"
Task: "Unit test for load_override_file with valid JSON in src/override.rs"
Task: "Unit test for validate_override_file with valid program address in src/override.rs"

# Launch all integration tests for User Story 1 together:
Task: "Integration test: IDL with missing address + override → generated code compiles"
Task: "Integration test: verify PROGRAM_ID constant matches override value"

# Launch all implementation tasks that don't depend on each other:
Task: "Implement discover_override_file() function in src/override.rs"
Task: "Implement load_override_file() function in src/override.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 3 - Both P1)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Missing Program Addresses)
4. Complete Phase 5: User Story 3 (Incorrect Account Discriminators)
5. **STOP and VALIDATE**: Test US1 and US3 independently
6. This MVP covers the most critical use cases (80% of real-world needs)

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (Basic MVP)
3. Add User Story 3 → Test independently → Deploy/Demo (Full MVP for P1)
4. Add User Story 2 → Test independently → Deploy/Demo (Incorrect addresses)
5. Add User Story 4 → Test independently → Deploy/Demo (Event discriminators)
6. Add User Story 5 → Test independently → Deploy/Demo (Instruction discriminators)
7. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Missing addresses)
   - Developer B: User Story 3 (Account discriminators)
   - Developer C: User Story 2 (Incorrect addresses)
3. Stories complete and integrate independently
4. Developer B can then tackle US4 (events) while A tackles US5 (instructions)

---

## Summary

**Total Tasks**: 110 tasks
- Setup: 5 tasks
- Foundational: 6 tasks
- User Story 1: 19 tasks (7 unit tests, 4 integration tests, 8 implementation)
- User Story 2: 13 tasks (3 unit tests, 4 integration tests, 6 implementation)
- User Story 3: 18 tasks (5 unit tests, 5 integration tests, 8 implementation)
- User Story 4: 14 tasks (3 unit tests, 4 integration tests, 7 implementation)
- User Story 5: 12 tasks (2 unit tests, 4 integration tests, 6 implementation)
- Edge Cases: 12 tasks (4 unit tests, 3 integration tests, 5 implementation)
- Polish: 12 tasks

**Task Breakdown by Story**:
- US1 (P1): 19 tasks - Missing program addresses (CRITICAL)
- US2 (P2): 13 tasks - Incorrect program addresses
- US3 (P1): 18 tasks - Account discriminators (CRITICAL)
- US4 (P2): 14 tasks - Event discriminators
- US5 (P3): 12 tasks - Instruction discriminators

**Parallel Opportunities**: 72 tasks marked [P] can run in parallel (65% of tasks)

**Independent Test Criteria**:
- US1: Override missing address → generated code has PROGRAM_ID
- US2: Override incorrect address → warning logged, code uses override
- US3: Override account discriminators → deserialization succeeds
- US4: Override event discriminators → event parsing succeeds
- US5: Override instruction discriminators → instruction encoding correct

**MVP Scope (Recommended)**: User Stories 1 + 3 (38 tasks, covers 80% of real-world needs)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Unit and integration tests are MANDATORY per Constitution Principles III and VIII
- Verify tests fail before implementing (TDD approach)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- All tasks follow strict checklist format: `- [ ] [ID] [P?] [Story?] Description with file path`
