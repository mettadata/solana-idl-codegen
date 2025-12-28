# Override Files

This directory contains JSON override files for correcting missing or incorrect data in Solana IDL files.

## Purpose

Override files allow you to:
- Add missing program addresses to IDL files
- Correct incorrect program addresses (e.g., devnet vs mainnet)
- Fix incorrect account discriminators
- Fix incorrect event discriminators
- Fix incorrect instruction discriminators

## File Naming Convention

- **Per-IDL override**: `{idl_name}.json` (e.g., `raydium_amm.json`)
- **Example files**: `*.example.json` (committed to git for reference)
- **User-created files**: Automatically gitignored (except examples)

## Example Override File

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

## Usage

Override files are auto-discovered when running code generation:

```bash
# Convention-based discovery (checks ./overrides/{idl_name}.json)
solana-idl-codegen -i idl/raydium_amm/idl.json -o generated -m raydium_amm

# Explicit override file path
solana-idl-codegen -i idl.json -o generated -m module --override-file path/to/override.json
```

## Documentation

See `docs/override-format.md` for complete override file format specification.
