# Codegen Tool Improvement Recommendations

This document analyzes the extra boilerplate code in `decode_blocks.rs` that exists on top of the IDL-generated interface files, and provides recommendations for improving the `solana-idl-codegen` tool to eliminate this manual work.

## Table of Contents

1. [Current Pain Points](#current-pain-points)
2. [Recommendations](#recommendations)
3. [Impact Summary](#impact-summary)
4. [Simplified decode_blocks.rs](#simplified-decode_blocksrs)

---

## Current Pain Points

After reviewing `chains/solana/src/bin/decode_blocks.rs` and the generated interfaces in `interfaces/solana/*/`, I've identified **7 major categories** of boilerplate that must be manually written on top of the generated interfaces.

### 1. Manual Instruction-to-Event Dispatch (~200+ lines per program)

The massive match statements in `decode_pumpfun_event`, `decode_pumpfun_amm_event`, etc.:

```rust
match instruction {
    "Buy" | "BuyExactSolIn" | "Sell" => {
        let event = pumpfun::events::TradeEventEvent::deserialize(&mut data)?;
        // ...
    }
    "Create" | "CreateV2" => { ... }
    // 18+ more cases for pumpfun alone
}
```

**Problem**: Every instruction→event mapping must be manually coded.

### 2. Duplicate Event Schema (`core/src/events/solana/*.rs`)

A parallel event schema is maintained with `String` fields instead of `Pubkey`:

| Generated Type | Manual Schema |
|---------------|---------------|
| `pub mint: Pubkey` | `pub mint: String` |
| `pub user: Pubkey` | `pub user: String` |

**Files affected**:
- `core/src/events/solana/pumpfun.rs`
- `core/src/events/solana/pumpfun_amm.rs`
- `core/src/events/solana/raydium_clmm.rs`
- `core/src/events/solana/raydium_cpmm.rs`

### 3. Manual Field-by-Field Conversion (~100+ lines per program)

Every event requires explicit conversion from generated types to the serializable schema:

```rust
EventData::PumpfunCreate(pandora_core::events::solana::PumpfunCreateEvent {
    name: event.name,
    symbol: event.symbol,
    mint: event.mint.to_string(),  // repeated for every Pubkey
    bonding_curve: event.bonding_curve.to_string(),
    associated_bonding_curve: associated_bonding_curve.to_string(),
    creator: event.creator.to_string(),
    uri: event.uri,
})
```

**Problem**: `.to_string()` must be called on every `Pubkey` field manually.

### 4. 50+ Logging Functions

One logging function per event type:

```rust
fn log_trade_event(worker, event, slot, block_height) { ... }
fn log_create_event(worker, event, slot, block_height) { ... }
fn log_pumpfun_admin_set_creator_event(...) { ... }
fn log_pumpfun_admin_set_idl_authority_event(...) { ... }
fn log_pumpfun_admin_update_token_incentives_event(...) { ... }
// ~50 more functions
```

**Problem**: Each event type requires a dedicated logging function with similar boilerplate.

### 5. Wrapper Type `.0` Access Pattern

The generated code uses wrapper types like `CreateEventEvent(CreateEvent)`, requiring `.0` access everywhere:

```rust
let event = pumpfun::events::TradeEventEvent::deserialize(&mut data)?;
log_trade_event(worker, &event.0, slot, block_height);  // .0 access
let event_data = convert_pumpfun_trade_event(event.0);  // .0 access
```

### 6. Manual Program Dispatcher (`dispatch_to_decoder`)

~30 lines matching program names to decoder functions:

```rust
match program_name {
    "pumpfun" => decode_pumpfun_event(worker, instruction, &decoded_data, slot, block_height),
    "pumpfun_amm" => decode_pumpfun_amm_event(...),
    "raydium_amm" => decode_raydium_amm_event(...),
    "raydium_clmm" => decode_raydium_clmm_event(...),
    "raydium_cpmm" => decode_raydium_cpmm_event(...),
    _ => Ok(None),
}
```

### 7. EventData Enum Maintenance

Manually keeping the `EventData` enum variants in sync with IDL events (~95 variants in `core/src/events/solana/mod.rs`):

```rust
pub enum EventData {
    PumpfunTrade(PumpfunTradeEvent),
    PumpfunCreate(PumpfunCreateEvent),
    PumpfunComplete(PumpfunCompleteEvent),
    // ... 92 more variants
}
```

---

## Recommendations

### 1. Generate a Unified Decoder with Instruction Mapping

Generate a `decoder.rs` per program that maps instructions to events automatically:

```rust
// Generated in pumpfun/src/decoder.rs
pub fn decode_instruction(instruction: &str, data: &[u8]) -> Result<DecodedEvent, DecodeError> {
    let mut data = data;
    match instruction {
        "Buy" | "BuyExactSolIn" | "Sell" => {
            let event = TradeEventEvent::deserialize(&mut data)?;
            Ok(DecodedEvent::Trade(event.0))
        }
        "Create" | "CreateV2" => {
            let event = CreateEventEvent::deserialize(&mut data)?;
            Ok(DecodedEvent::Create(event.0))
        }
        // Auto-generated from IDL instruction→event relationships
        _ => Err(DecodeError::UnknownInstruction(instruction.to_string())),
    }
}
```

**IDL Enhancement**: If your IDL doesn't specify which instruction emits which event, consider adding metadata:

```json
{
  "instructions": [{
    "name": "buy",
    "emitsEvent": "TradeEvent"
  }]
}
```

### 2. Generate String-Serializable Event Variants

Add a feature flag to generate events with `String` pubkeys for downstream serialization:

```rust
// Generated with --features serde-strings
#[cfg(feature = "serde-strings")]
#[derive(Serialize, Deserialize)]
pub struct TradeEventSerializable {
    pub mint: String,
    pub user: String,
    pub is_buy: bool,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub timestamp: i64,
}

impl From<TradeEvent> for TradeEventSerializable {
    fn from(e: TradeEvent) -> Self {
        Self {
            mint: e.mint.to_string(),
            user: e.user.to_string(),
            is_buy: e.is_buy,
            sol_amount: e.sol_amount,
            token_amount: e.token_amount,
            virtual_sol_reserves: e.virtual_sol_reserves,
            virtual_token_reserves: e.virtual_token_reserves,
            timestamp: e.timestamp,
        }
    }
}
```

**Benefit**: Eliminates the entire `core/src/events/solana/` module.

### 3. Generate Cross-Program Registry

Create a `registry.rs` at the workspace level:

```rust
// Generated in interfaces/solana/registry.rs
use std::collections::HashMap;

pub type DecodeResult = Result<Box<dyn Event>, DecodeError>;

lazy_static! {
    static ref DECODERS: HashMap<&'static str, fn(&str, &[u8]) -> DecodeResult> = {
        let mut m = HashMap::new();
        m.insert(
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
            pumpfun::decode_instruction as fn(&str, &[u8]) -> DecodeResult
        );
        m.insert(
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
            pumpfun_amm::decode_instruction
        );
        // Auto-discovered from lib.rs declare_id!
        m
    };
}

pub fn decode(program_id: &str, instruction: &str, data: &[u8]) -> DecodeResult {
    DECODERS.get(program_id)
        .ok_or(DecodeError::UnknownProgram)?
        (instruction, data)
}
```

**Benefit**: `dispatch_to_decoder` becomes a one-liner.

### 4. Generate a Common Event Trait with Logging

```rust
// Generated trait
pub trait LoggableEvent {
    fn log(&self, worker: usize, slot: u64, block_height: u64);
    fn program_name() -> &'static str;
}

// Generated impl for each event
impl LoggableEvent for TradeEvent {
    fn log(&self, worker: usize, slot: u64, block_height: u64) {
        log::debug!(
            "Worker: {}, [{}] Trade - Slot: {}, Block: {}, Mint: {}, User: {}, SOL: {} lamports",
            worker,
            Self::program_name(),
            slot,
            block_height,
            self.mint,
            self.user,
            self.sol_amount
        );
    }

    fn program_name() -> &'static str { "pumpfun" }
}
```

**Benefit**: Eliminates all 50+ log functions.

### 5. Generate Unified EventData Enum

Generate a cross-program `EventData` enum automatically:

```rust
// Generated in interfaces/solana/event_data.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventData {
    // Auto-generated from all interface events
    PumpfunTrade(pumpfun::events::TradeEventSerializable),
    PumpfunCreate(pumpfun::events::CreateEventSerializable),
    PumpfunAmmBuy(pumpfun_amm::events::BuyEventSerializable),
    RaydiumClmmSwap(raydium_clmm::events::SwapEventSerializable),
    // ... auto-generated for all events
}
```

**Benefit**: No need to maintain `core/src/events/solana/mod.rs` manually.

### 6. Use Deref Instead of Wrapper Types (or Flatten)

**Option A** - Use `#[repr(transparent)]` with `Deref`:

```rust
#[derive(Debug)]
#[repr(transparent)]
pub struct TradeEventEvent(pub TradeEvent);

impl std::ops::Deref for TradeEventEvent {
    type Target = TradeEvent;
    fn deref(&self) -> &Self::Target { &self.0 }
}
```

**Option B** - Remove wrapper entirely:

```rust
// Just use TradeEvent directly, put discriminator logic in deserialize
impl TradeEvent {
    pub fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let maybe_discm = <[u8; 8]>::deserialize(buf)?;
        if maybe_discm != TRADE_EVENT_DISCM {
            return Err(/* ... */);
        }
        // deserialize fields...
    }
}
```

### 7. Generate From Implementations

```rust
// Generated
impl From<pumpfun::events::TradeEvent> for EventData {
    fn from(e: pumpfun::events::TradeEvent) -> Self {
        EventData::PumpfunTrade(e.into())
    }
}
```

Your decode code becomes:

```rust
let event = TradeEventEvent::deserialize(&mut data)?;
Ok(Some(EventData::from(event.0)))
```

---

## Impact Summary

| Current Code | Lines | After Codegen Changes |
|--------------|-------|----------------------|
| `decode_pumpfun_event` match | ~200 | **0** (generated) |
| `decode_pumpfun_amm_event` match | ~250 | **0** (generated) |
| `decode_raydium_*` matches | ~200 | **0** (generated) |
| `dispatch_to_decoder` | ~30 | **3** (registry call) |
| EventData conversions | ~150 | **0** (generated From impls) |
| 50+ log functions | ~500 | **0** (generated trait) |
| `core/src/events/solana/*.rs` | ~200 | **0** (generated serializable types) |
| **Total** | **~1530** | **~50** |

**Estimated reduction: ~1500+ lines of manual code → ~50 lines**

---

## Simplified decode_blocks.rs

After implementing these codegen improvements, `decode_blocks.rs` would simplify dramatically:

```rust
use interfaces::registry;
use interfaces::event_data::EventData;

fn dispatch_to_decoder(
    program_id: &str,
    instruction: &str,
    data: &[u8],
) -> Result<Option<EventData>> {
    match registry::decode(program_id, instruction, data) {
        Ok(event) => {
            // Logging is now built into the event via LoggableEvent trait
            event.log(worker, slot, block_height);
            Ok(Some(event.into()))
        }
        Err(DecodeError::UnknownInstruction(_)) => Ok(None),
        Err(DecodeError::UnknownProgram) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
```

The entire `decode_blocks.rs` file could shrink from **~2500 lines** to **~400 lines**, with all the repetitive matching, conversion, and logging logic moved into generated code.

---

## Implementation Priority

1. **High Impact, Low Effort**
   - Generate `From` implementations for Pubkey→String conversion
   - Add `Deref` to wrapper types

2. **High Impact, Medium Effort**
   - Generate instruction→event decoder per program
   - Generate cross-program registry

3. **High Impact, Higher Effort**
   - Generate unified `EventData` enum
   - Generate `LoggableEvent` trait implementations

4. **Nice to Have**
   - IDL schema extension for instruction→event mapping
   - Feature flags for different serialization formats
