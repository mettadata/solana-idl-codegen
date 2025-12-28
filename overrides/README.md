# Override File Examples

This directory contains example override files demonstrating common use cases for the IDL Override System.

## Example Files

### `example_program_address.json`
Minimal override file that only sets the program address. Use when:
- IDL file is missing the `address` field
- IDL has incorrect program address for your target network

### `example_account_discriminators.json`
Override account discriminators. Use when:
- Account discriminators in IDL don't match deployed program
- Need to decode on-chain account data with correct discriminators

### `example_event_discriminators.json`
Override event discriminators. Use when:
- Event discriminators in IDL don't match program logs
- Parsing transaction logs for blockchain data ingestion

### `example_instruction_discriminators.json`
Override instruction discriminators. Use when:
- Instruction discriminators in IDL don't match deployed program
- Building transactions that call program instructions

### `example_complete.json`
Complete example combining all override types. Use as reference for:
- Projects requiring multiple override types
- Understanding override file structure

## Usage

These are examples only - **do not use these files directly**.

### For Real Projects

1. **Copy the relevant example** to create your override file:
   ```bash
   cp overrides/example_program_address.json ./overrides/your_idl_name.json
   ```

2. **Edit the discriminators** to match your deployed program:
   - Use on-chain data inspection tools
   - Verify against program source code
   - Test with real blockchain data

3. **Follow naming convention**: `./overrides/{idl_name}.json`
   - Example: `./overrides/raydium_amm.json` for `raydium_amm` IDL
   - Example: `./overrides/pumpfun.json` for `pumpfun` IDL

### Validation

All override files are strictly validated:
- Program address must be valid base58 Pubkey
- Discriminators must be exactly 8 bytes
- Discriminators cannot be all zeros `[0, 0, 0, 0, 0, 0, 0, 0]`
- Entity names must match IDL exactly (case-sensitive)
- Unknown entity names cause validation errors

## See Also

- [Override File Format Specification](../docs/override-format.md) - Complete schema and validation rules
- [README.md](../README.md) - Main project documentation with override system guide
