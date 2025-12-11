# Integration Testing Guide

## Overview

This project includes post-generation integration tests to verify that the generated Rust code compiles and functions correctly. The tests are located in `tests/integration_tests.rs`.

## Running Tests

```bash
# Generate code and run all tests
just test-all

# Run integration tests only (requires generated code)
just test-integration

# Run specific integration test
cargo test --test integration_tests test_name -- --nocapture
```

## Test Structure

### 1. Basic Structure Tests

These tests verify that all generated crates have the required files and structure:

- `test_all_crates_generated` - Checks that all expected crates exist
- `test_all_crates_have_required_files` - Verifies each crate has required files (Cargo.toml, lib.rs, etc.)

### 2. Compilation Tests

The framework tests that generated code compiles by creating small test programs that:

- Import the generated crate
- Use various generated types and functions
- Verify the code compiles and runs successfully

### 3. Pattern Tests

Tests verify that code generation patterns are correctly implemented:

- **Event Wrapper Pattern**: Tests that discriminator constants exist and wrapper structs are generated
- **Account Discriminators**: Tests that account discriminators are generated and unique
- **Instruction Enums**: Tests that instruction enums serialize/deserialize correctly
- **Error Enums**: Tests that error enums have correct codes and can convert from numeric values

## Writing IDL-Specific Tests

Since IDL structures vary and evolve, tests that depend on specific field structures are fragile. Instead:

### ✅ Good Test Patterns

```rust
test_if_generated!("my_crate", test_discriminators_exist, {
    let test_code = r#"
        use my_crate::*;
        
        fn main() {
            // Test that discriminator constants exist (doesn't depend on field structure)
            assert_eq!(CREATE_EVENT_EVENT_DISCM.len(), 8);
            assert_ne!(CREATE_EVENT_EVENT_DISCM, [0; 8]);
            println!("✓ Discriminators exist");
        }
    "#;
    
    run_generated_test_code("my_crate", test_code);
});
```

### ❌ Avoid These Patterns

```rust
// BAD: Too specific to IDL structure
let event = CreateEvent {
    field1: value1,
    field2: value2,
    // Will break if IDL adds new required fields
};
```

## Test Categories

### Core Pattern Tests (Always Included)

These tests verify fundamental code generation patterns:

1. **Module Structure**
   - All required modules exist (accounts, instructions, events, errors, types)
   - Modules can be imported
   - No compilation errors

2. **Discriminator Generation**
   - Constants are generated for accounts/instructions/events
   - Discriminators are 8 bytes
   - Discriminators are unique within a program

3. **Trait Implementations**
   - BorshSerialize/BorshDeserialize work
   - Debug, Clone, PartialEq are implemented
   - Optional serde traits work when feature is enabled

### IDL-Specific Tests (Optional)

For specific programs you're working with, you can add tests that:

1. Check specific instruction variants exist
2. Verify account structures for known programs
3. Test end-to-end serialization with real data

Example:

```rust
// Add to tests/integration_tests.rs
test_if_generated!("my_program", test_my_specific_instruction, {
    let test_code = r#"
        use my_program::*;
        
        fn main() {
            // Test specific instruction exists
            match Instruction::MyInstruction(...) {
                _ => println!("✓ MyInstruction variant exists"),
            }
        }
    "#;
    
    run_generated_test_code("my_program", test_code);
});
```

## Test Helper Macro

The `test_if_generated!` macro automatically skips tests if generated code doesn't exist:

```rust
test_if_generated!("crate_name", test_function_name, {
    let test_code = r#"
        // Rust code to test
    "#;
    
    run_generated_test_code("crate_name", test_code);
});
```

## Current Test Status

As of the latest run, the following tests pass:

- ✅ `test_all_crates_generated` - All 5 crates exist
- ✅ `test_all_crates_have_required_files` - All crates have required files
- ✅ `test_pumpfun_event_discriminators_exist` - Event discriminators are generated correctly

Tests that require IDL-specific field structures are commented out as they're fragile and break when IDLs are updated.

## Continuous Integration

Integration tests can be added to CI/CD pipelines:

```yaml
# .github/workflows/test.yml
- name: Generate code
  run: just generate

- name: Run tests
  run: just test-all
```

## Troubleshooting

### Test fails with "missing fields"

This means the IDL has changed since the test was written. Update the test to use the new field structure, or make the test more generic.

### Test fails with "crate not found"

Run `just generate` to create the generated code.

### Test compilation takes too long

The test framework creates mini Cargo projects for each test. This is necessary to properly test the generated code in isolation, but can be slow. Consider:

1. Running fewer tests during development
2. Using `cargo test --test integration_tests test_name` to run specific tests
3. Running full test suite only in CI

## Best Practices

1. **Keep tests generic** - Don't rely on specific IDL field structures
2. **Test patterns, not data** - Verify the code generation patterns work
3. **Document IDL versions** - If you do write IDL-specific tests, note which IDL version they're for
4. **Use CI** - Run integration tests automatically on commits
5. **Test compilation** - Even simple "does it compile" tests are valuable
