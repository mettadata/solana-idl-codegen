use solana_idl_codegen::codegen;
use solana_idl_codegen::idl::*;

/// Helper to create a minimal IDL with events that have discriminators.
fn test_idl_with_events() -> Idl {
    Idl {
        address: Some("11111111111111111111111111111111".to_string()),
        version: None,
        name: None,
        metadata: Some(Metadata {
            name: Some("test_program".to_string()),
            version: Some("0.1.0".to_string()),
            spec: None,
            description: None,
            address: None,
        }),
        instructions: vec![Instruction {
            name: "initialize".to_string(),
            discriminator: Some(vec![0, 1, 2, 3, 4, 5, 6, 7]),
            accounts: vec![],
            args: vec![],
            docs: None,
        }],
        accounts: None,
        events: Some(vec![
            Event {
                name: "TradeEvent".to_string(),
                discriminator: Some(vec![189, 219, 127, 211, 78, 230, 97, 238]),
                fields: Some(vec![
                    EventField {
                        name: "amount".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        index: false,
                    },
                    EventField {
                        name: "price".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        index: false,
                    },
                ]),
            },
            Event {
                name: "CreateEvent".to_string(),
                discriminator: Some(vec![27, 114, 169, 77, 222, 235, 99, 118]),
                fields: Some(vec![EventField {
                    name: "name".to_string(),
                    ty: IdlType::Simple("string".to_string()),
                    index: false,
                }]),
            },
        ]),
        errors: None,
        types: None,
        constants: None,
    }
}

fn empty_idl() -> Idl {
    Idl {
        address: None,
        version: None,
        name: Some("empty_program".to_string()),
        metadata: None,
        instructions: vec![],
        accounts: None,
        events: None,
        errors: None,
        types: None,
        constants: None,
    }
}

/// T009: Unit test for generate_decoder() — verify generated decoder source
/// contains correct discriminator match arms.
#[test]
fn test_generate_decoder_contains_discriminator_arms() {
    let idl = test_idl_with_events();
    let result = codegen::generate_decoder(&idl, "test_program").unwrap();

    assert!(!result.is_empty(), "Decoder should be generated for IDL with events");

    // Should contain the DecodeError type
    assert!(result.contains("DecodeError"), "Should contain DecodeError type");

    // Should contain the decode_event function
    assert!(result.contains("fn decode_event"), "Should contain decode_event function");

    // Should reference discriminator constants
    assert!(
        result.contains("TRADE_EVENT_EVENT_DISCM"),
        "Should reference TradeEvent discriminator constant"
    );
    assert!(
        result.contains("CREATE_EVENT_EVENT_DISCM"),
        "Should reference CreateEvent discriminator constant"
    );

    // Should reference wrapper types
    assert!(
        result.contains("TradeEventEvent"),
        "Should reference TradeEventEvent wrapper"
    );
    assert!(
        result.contains("CreateEventEvent"),
        "Should reference CreateEventEvent wrapper"
    );

    // Should contain ParsedEvent variants
    assert!(
        result.contains("ParsedEvent"),
        "Should reference ParsedEvent enum"
    );
}

/// T009: Verify decoder handles IDL with no events gracefully.
#[test]
fn test_generate_decoder_no_events() {
    let idl = empty_idl();
    let result = codegen::generate_decoder(&idl, "empty_program").unwrap();
    assert!(result.is_empty(), "No decoder should be generated for IDL without events");
}

/// T010: Unit test for generate_deref_impls() — verify Deref impl generated
/// for all wrapper types.
#[test]
fn test_generate_deref_impls() {
    let idl = test_idl_with_events();
    let result = codegen::generate_deref_impls(&idl).unwrap();

    assert!(!result.is_empty(), "Deref impls should be generated");

    // Should contain Deref for both wrapper types (prettyplease formatting)
    assert!(
        result.contains("Deref for TradeEventEvent"),
        "Should have Deref impl for TradeEventEvent, got:\n{}",
        &result[..300.min(result.len())]
    );
    assert!(
        result.contains("Deref for CreateEventEvent"),
        "Should have Deref impl for CreateEventEvent"
    );

    // Should reference inner types as Target
    assert!(result.contains("TradeEvent"), "Should reference TradeEvent as Target");
    assert!(result.contains("CreateEvent"), "Should reference CreateEvent as Target");
}

/// T010: Verify deref impls handles no events gracefully.
#[test]
fn test_generate_deref_impls_no_events() {
    let idl = empty_idl();
    let result = codegen::generate_deref_impls(&idl).unwrap();
    assert!(result.is_empty());
}

/// T012a: Unit test for FR-006: feed IDL with duplicate discriminators to codegen,
/// verify it produces a clear error and does not generate code.
#[test]
fn test_generate_decoder_duplicate_discriminators_error() {
    let idl = Idl {
        address: Some("11111111111111111111111111111111".to_string()),
        version: None,
        name: None,
        metadata: Some(Metadata {
            name: Some("dup_program".to_string()),
            version: Some("0.1.0".to_string()),
            spec: None,
            description: None,
            address: None,
        }),
        instructions: vec![],
        accounts: None,
        events: Some(vec![
            Event {
                name: "EventA".to_string(),
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                fields: Some(vec![EventField {
                    name: "value".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    index: false,
                }]),
            },
            Event {
                name: "EventB".to_string(),
                // Same discriminator as EventA — should cause error
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                fields: Some(vec![EventField {
                    name: "data".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    index: false,
                }]),
            },
        ]),
        errors: None,
        types: None,
        constants: None,
    };

    let result = codegen::generate_decoder(&idl, "dup_program");
    assert!(result.is_err(), "Should fail with duplicate discriminators");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Duplicate discriminator"),
        "Error should mention duplicate discriminator: {}",
        err_msg
    );
    assert!(
        err_msg.contains("EventA") && err_msg.contains("EventB"),
        "Error should name both conflicting events: {}",
        err_msg
    );
}

/// T017: Verify DecodedProgramEvent includes program_id() method.
#[test]
fn test_serializable_includes_program_id() {
    let idl = test_idl_with_events();
    let generated = codegen::generate(&idl, "test_program").unwrap();

    assert!(
        generated.serializable.contains("program_id"),
        "Serializable module should contain program_id function"
    );
}

/// T017: Verify generated lib.rs includes ID_STR constant.
#[test]
fn test_lib_includes_id_str() {
    let idl = test_idl_with_events();
    let generated = codegen::generate(&idl, "test_program").unwrap();

    assert!(
        generated.lib.contains("ID_STR"),
        "Generated lib.rs should contain ID_STR constant"
    );
}

/// Verify GeneratedCode has decoder and deref_impls populated after generate().
#[test]
fn test_generate_populates_decoder_fields() {
    let idl = test_idl_with_events();
    let generated = codegen::generate(&idl, "test_program").unwrap();

    assert!(!generated.decoder.is_empty(), "decoder field should be populated");
    assert!(!generated.deref_impls.is_empty(), "deref_impls field should be populated");
}

/// Verify generated lib.rs includes decoder module declaration.
#[test]
fn test_lib_includes_decoder_mod() {
    let idl = test_idl_with_events();
    let generated = codegen::generate(&idl, "test_program").unwrap();

    assert!(
        generated.lib.contains("pub mod decoder;"),
        "Generated lib.rs should declare decoder module"
    );
}
