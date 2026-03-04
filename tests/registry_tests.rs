//! Tests for registry generation and event dispatch (T020-T023, T059-T060).

use solana_idl_codegen::idl::*;
use solana_idl_codegen::manifest::ProgramEntry;
use solana_idl_codegen::registry::{collect_program_event_info, generate_registry_crate};

/// Helper: create a ProgramEntry for testing.
fn entry(name: &str) -> ProgramEntry {
    ProgramEntry {
        name: name.to_string(),
        idl: format!("idl/{}.json", name),
        override_file: None,
    }
}

/// Helper: create a minimal IDL with events that have discriminators.
fn idl_with_events(name: &str, address: &str, events: Vec<Event>) -> Idl {
    Idl {
        address: Some(address.to_string()),
        version: None,
        name: None,
        metadata: Some(Metadata {
            name: Some(name.to_string()),
            version: Some("0.1.0".to_string()),
            spec: None,
            description: None,
            address: None,
        }),
        instructions: vec![],
        accounts: None,
        events: Some(events),
        errors: None,
        types: None,
        constants: None,
    }
}

/// Helper: create an event with a discriminator.
fn event(name: &str, disc: Vec<u8>) -> Event {
    Event {
        name: name.to_string(),
        discriminator: Some(disc),
        fields: Some(vec![EventField {
            name: "value".to_string(),
            ty: IdlType::Simple("u64".to_string()),
            index: false,
        }]),
    }
}

/// T020: Registry decode() dispatch — verify routing by program_id.
/// Tests that ProgramEventInfo correctly collects event info and that
/// the generated registry source routes by program_id.
#[test]
fn test_registry_dispatch_by_program_id() {
    let idl = idl_with_events(
        "test_prog",
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
        vec![event("TradeEvent", vec![1, 2, 3, 4, 5, 6, 7, 8])],
    );

    let info = collect_program_event_info(&entry("test_prog"), &idl);

    assert_eq!(info.module_name, "test_prog");
    assert_eq!(
        info.program_id.as_deref(),
        Some("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
    );
    assert!(info.has_decoder);

    // Generate registry and verify dispatch source
    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[info]).unwrap();

    let registry_src = std::fs::read_to_string(dir.path().join("test_registry/src/registry.rs"))
        .expect("registry.rs should exist");

    // Should contain the program_id match arm
    assert!(
        registry_src.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"),
        "Registry should route by program_id"
    );

    // Should contain the decode_event call
    assert!(
        registry_src.contains("decode_event"),
        "Registry should call decoder"
    );
}

/// T021: EventData enum completeness — verify all events from all programs have variants.
#[test]
fn test_event_data_enum_completeness() {
    let prog_a = {
        let idl = idl_with_events(
            "alpha",
            "AlphaAddr111111111111111111111111111111111",
            vec![
                event("TradeEvent", vec![1, 2, 3, 4, 5, 6, 7, 8]),
                event("CreateEvent", vec![9, 10, 11, 12, 13, 14, 15, 16]),
            ],
        );
        collect_program_event_info(&entry("alpha"), &idl)
    };

    let prog_b = {
        let idl = idl_with_events(
            "beta",
            "BetaAddr2222222222222222222222222222222222222",
            vec![event("SwapEvent", vec![17, 18, 19, 20, 21, 22, 23, 24])],
        );
        collect_program_event_info(&entry("beta"), &idl)
    };

    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[prog_a, prog_b]).unwrap();

    let event_data_src =
        std::fs::read_to_string(dir.path().join("test_registry/src/event_data.rs"))
            .expect("event_data.rs should exist");

    // All program-prefixed variants should exist
    assert!(
        event_data_src.contains("AlphaTrade"),
        "Should have AlphaTrade variant"
    );
    assert!(
        event_data_src.contains("AlphaCreate"),
        "Should have AlphaCreate variant"
    );
    assert!(
        event_data_src.contains("BetaSwap"),
        "Should have BetaSwap variant"
    );

    // Unknown variant should always exist
    assert!(
        event_data_src.contains("Unknown(UnknownEvent)"),
        "Should have Unknown variant"
    );
}

/// T022: LoggableEvent trait impls — verify event_name() and log() generated.
#[test]
fn test_loggable_event_trait_impls() {
    let prog = {
        let idl = idl_with_events(
            "test_prog",
            "TestAddr11111111111111111111111111111111111",
            vec![
                event("TradeEvent", vec![1, 2, 3, 4, 5, 6, 7, 8]),
                event("SwapEvent", vec![9, 10, 11, 12, 13, 14, 15, 16]),
            ],
        );
        collect_program_event_info(&entry("test_prog"), &idl)
    };

    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[prog]).unwrap();

    let event_data_src =
        std::fs::read_to_string(dir.path().join("test_registry/src/event_data.rs"))
            .expect("event_data.rs should exist");

    // LoggableEvent impl should exist
    assert!(
        event_data_src.contains("impl crate::LoggableEvent for EventData"),
        "Should implement LoggableEvent for EventData"
    );

    // event_name() should return correct names
    assert!(
        event_data_src.contains("fn event_name"),
        "Should have event_name() method"
    );
    assert!(
        event_data_src.contains("\"TestProgTrade\""),
        "Should return correct event name for Trade"
    );
    assert!(
        event_data_src.contains("\"TestProgSwap\""),
        "Should return correct event name for Swap"
    );

    // log() should exist
    assert!(
        event_data_src.contains("fn log"),
        "Should have log() method"
    );
}

/// T023: Unknown program_id — verify decode() returns empty result (FR-014).
#[test]
fn test_unknown_program_id_returns_empty() {
    let prog = {
        let idl = idl_with_events(
            "known",
            "KnownAddr11111111111111111111111111111111111",
            vec![event("TradeEvent", vec![1, 2, 3, 4, 5, 6, 7, 8])],
        );
        collect_program_event_info(&entry("known"), &idl)
    };

    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[prog]).unwrap();

    let registry_src = std::fs::read_to_string(dir.path().join("test_registry/src/registry.rs"))
        .expect("registry.rs should exist");

    // The wildcard arm should return empty (not error)
    assert!(
        registry_src.contains("_ =>"),
        "Registry should have wildcard arm for unknown program_id"
    );

    // The function returns a Vec, empty for unknown programs
    assert!(
        registry_src.contains("let mut results = Vec::new()"),
        "Registry should initialize empty results vec"
    );
}

/// T059: Edge case — generate decoder for an IDL with zero events.
#[test]
fn test_zero_events_idl_registry() {
    let idl = Idl {
        address: Some("EmptyAddr111111111111111111111111111111111".to_string()),
        version: None,
        name: Some("empty_program".to_string()),
        metadata: None,
        instructions: vec![],
        accounts: None,
        events: Some(vec![]),
        errors: None,
        types: None,
        constants: None,
    };

    let info = collect_program_event_info(&entry("empty_prog"), &idl);

    assert!(
        !info.has_decoder,
        "Program with no events should have has_decoder=false"
    );
    assert!(info.event_names.is_empty(), "Event names should be empty");

    // Registry should still generate without errors
    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[info]).unwrap();

    let event_data_src =
        std::fs::read_to_string(dir.path().join("test_registry/src/event_data.rs"))
            .expect("event_data.rs should exist");

    // Should still have Unknown variant
    assert!(event_data_src.contains("Unknown(UnknownEvent)"));
}

/// T059 variant: IDL with events but no discriminators.
#[test]
fn test_events_without_discriminators() {
    let idl = Idl {
        address: Some("NoDiscAddr11111111111111111111111111111111".to_string()),
        version: None,
        name: Some("nodisc_program".to_string()),
        metadata: None,
        instructions: vec![],
        accounts: None,
        events: Some(vec![Event {
            name: "SomeEvent".to_string(),
            discriminator: None,
            fields: Some(vec![]),
        }]),
        errors: None,
        types: None,
        constants: None,
    };

    let info = collect_program_event_info(&entry("nodisc"), &idl);

    assert!(
        !info.has_decoder,
        "Events without discriminators should not have decoder"
    );
    assert!(info.event_names.is_empty());
}

/// T060: Edge case — two programs with same-named event types.
#[test]
fn test_same_named_events_no_collision() {
    let prog_a = {
        let idl = idl_with_events(
            "alpha",
            "AlphaAddr111111111111111111111111111111111",
            vec![event("SwapEvent", vec![1, 2, 3, 4, 5, 6, 7, 8])],
        );
        collect_program_event_info(&entry("alpha"), &idl)
    };

    let prog_b = {
        let idl = idl_with_events(
            "beta",
            "BetaAddr2222222222222222222222222222222222222",
            vec![event("SwapEvent", vec![9, 10, 11, 12, 13, 14, 15, 16])],
        );
        collect_program_event_info(&entry("beta"), &idl)
    };

    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[prog_a, prog_b]).unwrap();

    let event_data_src =
        std::fs::read_to_string(dir.path().join("test_registry/src/event_data.rs"))
            .expect("event_data.rs should exist");

    // Both variants should exist with program prefixes (no collision)
    assert!(
        event_data_src.contains("AlphaSwap"),
        "Should have AlphaSwap variant"
    );
    assert!(
        event_data_src.contains("BetaSwap"),
        "Should have BetaSwap variant"
    );

    // They should be distinct variants
    let alpha_count = event_data_src.matches("AlphaSwap").count();
    let beta_count = event_data_src.matches("BetaSwap").count();
    assert!(
        alpha_count >= 2 && beta_count >= 2,
        "Each variant should appear in enum definition and in match arms"
    );
}

/// T021 (continued): Verify traits.rs has LoggableEvent trait definition.
#[test]
fn test_traits_rs_generated() {
    let prog = {
        let idl = idl_with_events(
            "test_prog",
            "TestAddr11111111111111111111111111111111111",
            vec![event("TradeEvent", vec![1, 2, 3, 4, 5, 6, 7, 8])],
        );
        collect_program_event_info(&entry("test_prog"), &idl)
    };

    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[prog]).unwrap();

    let traits_src = std::fs::read_to_string(dir.path().join("test_registry/src/traits.rs"))
        .expect("traits.rs should exist");

    assert!(
        traits_src.contains("pub trait LoggableEvent"),
        "Should define LoggableEvent trait"
    );
    assert!(
        traits_src.contains("fn event_name"),
        "LoggableEvent should have event_name"
    );
    assert!(
        traits_src.contains("fn log"),
        "LoggableEvent should have log"
    );
}

/// Verify generated Cargo.toml has correct dependencies.
#[test]
fn test_registry_cargo_toml_generated() {
    let prog_a = {
        let idl = idl_with_events(
            "alpha",
            "AlphaAddr111111111111111111111111111111111",
            vec![event("TradeEvent", vec![1, 2, 3, 4, 5, 6, 7, 8])],
        );
        collect_program_event_info(&entry("alpha"), &idl)
    };

    let dir = tempfile::TempDir::new().unwrap();
    generate_registry_crate(dir.path(), "test_registry", &[prog_a]).unwrap();

    let cargo_toml = std::fs::read_to_string(dir.path().join("test_registry/Cargo.toml"))
        .expect("Cargo.toml should exist");

    assert!(cargo_toml.contains("name = \"test_registry\""));
    assert!(cargo_toml.contains("serde"));
    assert!(cargo_toml.contains("base64"));
    assert!(cargo_toml.contains("alpha = { path = \"../alpha\" }"));
}
