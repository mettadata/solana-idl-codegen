# Quickstart: IDL Override System

**Feature**: IDL Override System | **Date**: 2025-12-26 | **Plan**: [plan.md](./plan.md)

## 5-Minute Quick Start

### Problem

You're using the Solana IDL codegen tool with an IDL file from a git submodule (e.g., `raydium_amm/idl.json`). The IDL is missing the program address or has incorrect discriminators that don't match the deployed mainnet program. You need to fix this without editing the upstream IDL file.

### Solution

Create an override file to correct the IDL data.

### Steps

**1. Create override file** at `./overrides/raydium_amm.json`:

```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "accounts": {
    "PoolState": {
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
    }
  }
}
```

**2. Run codegen** (override is auto-discovered):

```bash
solana-idl-codegen -i idl/raydium_amm/idl.json -o generated -m raydium_amm
```

**3. Verify** generated code includes corrected values:

```bash
grep "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" generated/raydium_amm/src/lib.rs
grep "DISCRIMINATOR.*\[1, 2, 3, 4, 5, 6, 7, 8\]" generated/raydium_amm/src/accounts.rs
```

**Done!** Your generated code now has correct program address and discriminators.

---

## Common Use Cases

### Use Case 1: Missing Program Address

**Problem**: IDL file has no `address` field

**Override File** (`overrides/raydium_amm.json`):
```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
}
```

**Result**: Generated code includes `pub const PROGRAM_ID: Pubkey = ...`

---

### Use Case 2: Incorrect Account Discriminators

**Problem**: IDL discriminators don't match on-chain program

**Override File** (`overrides/pumpfun.json`):
```json
{
  "accounts": {
    "BondingCurve": {
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
    },
    "Global": {
      "discriminator": [9, 10, 11, 12, 13, 14, 15, 16]
    }
  }
}
```

**Result**: Generated account structs use corrected discriminators for deserialization

---

### Use Case 3: Event Discriminators for Log Parsing

**Problem**: Event discriminators in IDL don't match actual program logs

**Override File** (`overrides/raydium_clmm.json`):
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

**Result**: Event parsing from transaction logs uses correct discriminators

---

### Use Case 4: Multiple Overrides

**Problem**: Multiple issues in same IDL

**Override File** (`overrides/raydium_amm.json`):
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

**Result**: All overrides applied in single pass

---

## Override File Locations

### Convention-Based Discovery (Recommended)

**Per-IDL Files** (most common):
```
overrides/
├── raydium_amm.json     # Overrides for raydium_amm IDL
├── raydium_clmm.json    # Overrides for raydium_clmm IDL
└── pumpfun.json         # Overrides for pumpfun IDL
```

**Command**:
```bash
just generate  # Auto-discovers overrides in overrides/ directory
```

**Global Fallback** (simple projects with one IDL):
```
idl-overrides.json       # Global override file
```

### Explicit Override File

**Use Case**: Custom location or complex project structure

**Command**:
```bash
solana-idl-codegen \
  -i idl/raydium_amm/idl.json \
  -o generated \
  -m raydium_amm \
  --override-file custom/path/my-overrides.json
```

---

## Workflow Integration

### With `justfile`

**Update `justfile` to use convention-based discovery**:

```makefile
# Generate bindings from IDL files (with auto-override discovery)
idl-codegen-generate:
    #!/usr/bin/env bash
    set -euo pipefail
    {{...}}
    for item in {{idls}}; do
        {{...}}
        cargo run --release -- \
            -i "$idl_path" \
            -o generated \
            -m "$module_name"
        # Override files auto-discovered from overrides/ directory
    done
```

**No changes needed** - convention-based discovery works automatically!

### Manual Workflow

**1. Create/update override files** in `overrides/`

**2. Regenerate code**:
```bash
just generate
# or
cargo run --release -- -i idl/raydium_amm/idl.json -o generated -m raydium_amm
```

**3. Verify generated code compiles**:
```bash
just check-generated
```

**4. Commit override files** (not generated code):
```bash
git add overrides/
git commit -m "Add overrides for raydium_amm discriminators"
```

---

## Getting Override Values

### Finding Correct Program Addresses

**From Solana Explorer**:
1. Go to https://explorer.solana.com/
2. Search for program name (e.g., "Raydium AMM")
3. Copy program address from URL or page

**From Source Code**:
```rust
// Look for PROGRAM_ID constant in program source
pub const PROGRAM_ID: Pubkey = ...;
```

**From Deployed IDL**:
```bash
# Fetch IDL from deployed program
anchor idl fetch <PROGRAM_ADDRESS>
```

### Finding Correct Discriminators

**Option 1: Inspect On-Chain Account Data**
```bash
# Fetch account data and check first 8 bytes
solana account <ACCOUNT_ADDRESS> --output json
```

**Option 2: Check Program Source Code**
```rust
// Account discriminator is SHA256 hash of "account:<AccountName>"
// First 8 bytes of hash
let discriminator = sha256("account:PoolState")[0..8];
```

**Option 3: Test Against Mainnet**
```rust
// Attempt deserialization and observe discriminator mismatch error
let data = fetch_account_data(address);
let account = PoolState::try_from_slice(&data)?;
// Error will show expected vs actual discriminator
```

---

## Troubleshooting

### Multiple Override Files Detected

**Error**:
```
Error: Multiple override files detected for 'raydium_amm' IDL:
  - ./overrides/raydium_amm.json (convention-based discovery)
  - /custom/path/overrides.json (--override-file argument)
```

**Fix**: Remove one of the conflicting files or use `--override-file` exclusively

---

### Invalid Discriminator

**Error**:
```
Error: Invalid discriminator for account 'PoolState': cannot be all zeros
```

**Fix**: Use actual discriminator values, not placeholders:
```json
{
  "accounts": {
    "PoolState": {
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]  // Replace with actual values
    }
  }
}
```

---

### Unknown Entity Warning

**Warning**:
```
Warning: Override file contains discriminator for account 'UnknownAccount' which does not exist in IDL 'raydium_amm'
  Override entry: accounts.UnknownAccount
  Available accounts: PoolState, AmmConfig, UserPosition
```

**Fix**: Check entity name spelling or verify IDL version:
```json
{
  "accounts": {
    "PoolState": {  // Correct entity name
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
    }
  }
}
```

---

### Invalid Program Address

**Error**:
```
Error: Invalid program address: "not-a-valid-pubkey". Must be valid base58-encoded Pubkey.
```

**Fix**: Use valid base58 Solana address:
```json
{
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
}
```

---

## Tips & Best Practices

### Version Control

**Do commit**:
- Override files (`overrides/*.json`)
- Documentation of why overrides are needed

**Don't commit**:
- Generated code (`generated/`)
- Temporary override files

### Documentation

**Add comments to override files** (not standard JSON, but helpful):
```json
{
  "_comment": "Override for raydium_amm mainnet deployment",
  "_reason": "IDL missing program address, discriminators don't match mainnet",
  "_source": "https://explorer.solana.com/address/675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "program_address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "accounts": {
    "PoolState": {
      "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
    }
  }
}
```

Or create separate `overrides/README.md`:
```markdown
# Override Files

## raydium_amm.json
- **Reason**: IDL missing program address
- **Source**: Mainnet deployment 675kPX9...
- **Date**: 2025-12-26
```

### Testing

**Verify overrides are applied**:
```bash
# Run codegen
just generate

# Check generated code
grep "PROGRAM_ID" generated/raydium_amm/src/lib.rs
grep "DISCRIMINATOR" generated/raydium_amm/src/accounts.rs

# Compile generated code
just check-generated
```

### Maintenance

**Keep overrides in sync**:
1. When updating IDL submodules, review override files
2. Check if overrides are still needed (IDL may be fixed upstream)
3. Verify override values still match mainnet programs
4. Remove unnecessary overrides

---

## Example Project Setup

```
solana-idl-codegen/
├── idl/                    # Git submodules with upstream IDLs
│   ├── raydium-idl/
│   │   ├── raydium_amm/
│   │   │   └── idl.json   # Original IDL (missing address)
│   │   └── raydium_clmm/
│   │       └── idl.json   # Original IDL (incorrect discriminators)
│   └── pump-public-docs/
│       └── pumpfun/
│           └── idl.json   # Original IDL (missing address)
├── overrides/              # Your override files (version controlled)
│   ├── raydium_amm.json   # Override for raydium_amm
│   ├── raydium_clmm.json  # Override for raydium_clmm
│   ├── pumpfun.json       # Override for pumpfun
│   └── README.md          # Documentation of overrides
├── generated/              # Generated code (gitignored)
│   ├── raydium_amm/       # Generated with overrides applied
│   ├── raydium_clmm/      # Generated with overrides applied
│   └── pumpfun/           # Generated with overrides applied
├── src/                    # Codegen tool source
├── justfile                # Build automation
└── README.md               # Project documentation
```

---

## Next Steps

1. **Create your first override file** in `overrides/`
2. **Run codegen** to verify it works: `just generate`
3. **Check generated code** to confirm overrides applied
4. **Commit override file** to version control
5. **Document** why overrides are needed in `overrides/README.md`

For more details, see:
- [Data Model](./data-model.md) - Override file schema and structures
- [API Contract](./contracts/override-api.md) - Function signatures and behaviors
- [Implementation Plan](./plan.md) - Technical architecture
