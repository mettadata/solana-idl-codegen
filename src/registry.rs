//! Registry crate generation.
//!
//! Generates `solana_registry` — a cross-program crate providing:
//! - Unified `EventData` enum spanning all programs
//! - `decode(program_id, data)` dispatch function
//! - `LoggableEvent` and `DecoderTrait` trait definitions
//! - `UnknownEvent` fallback type

use crate::codegen;
use crate::idl::Idl;
use crate::manifest::ProgramEntry;
use anyhow::{Context, Result};
use heck::ToPascalCase;
use std::fs;
use std::path::Path;

/// Info about a program's events, collected during the first codegen pass.
#[derive(Debug, Clone)]
pub struct ProgramEventInfo {
    /// Module name (snake_case, e.g., "pumpfun")
    pub module_name: String,
    /// Program ID string (on-chain address), if available
    pub program_id: Option<String>,
    /// Event names from the IDL (e.g., "TradeEvent", "CreateEvent")
    pub event_names: Vec<String>,
    /// Whether the program has events with discriminators
    pub has_decoder: bool,
}

/// Collect event info from an IDL for registry generation.
pub fn collect_program_event_info(entry: &ProgramEntry, idl: &Idl) -> ProgramEventInfo {
    let event_names: Vec<String> = idl
        .events
        .as_ref()
        .map(|events| {
            events
                .iter()
                .filter(|e| e.discriminator.is_some())
                .map(|e| e.name.clone())
                .collect()
        })
        .unwrap_or_default();

    let has_decoder = !event_names.is_empty();

    ProgramEventInfo {
        module_name: entry.name.clone(),
        program_id: idl.get_address().map(|s| s.to_string()),
        event_names,
        has_decoder,
    }
}

/// Generate the entire solana_registry crate.
pub fn generate_registry_crate(
    output_dir: &Path,
    registry_name: &str,
    programs: &[ProgramEventInfo],
) -> Result<()> {
    let crate_dir = output_dir.join(registry_name);
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("Failed to create registry src dir: {}", src_dir.display()))?;

    // Generate all modules
    let lib_rs = generate_lib_rs(programs);
    let traits_rs = generate_traits_rs();
    let event_data_rs = generate_event_data_rs(programs);
    let registry_rs = generate_registry_rs(programs);
    let cargo_toml = generate_cargo_toml(registry_name, programs);

    // Write files
    fs::write(src_dir.join("lib.rs"), &lib_rs).context("Failed to write lib.rs")?;
    fs::write(src_dir.join("traits.rs"), &traits_rs).context("Failed to write traits.rs")?;
    fs::write(src_dir.join("event_data.rs"), &event_data_rs)
        .context("Failed to write event_data.rs")?;
    fs::write(src_dir.join("registry.rs"), &registry_rs).context("Failed to write registry.rs")?;
    fs::write(crate_dir.join("Cargo.toml"), &cargo_toml).context("Failed to write Cargo.toml")?;

    // Format all generated files
    let files: Vec<String> = ["lib.rs", "traits.rs", "event_data.rs", "registry.rs"]
        .iter()
        .map(|f| src_dir.join(f).to_string_lossy().to_string())
        .collect();

    let rustfmt_result = std::process::Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .args(&files)
        .output();

    if let Err(e) = rustfmt_result {
        eprintln!("Warning: Failed to run rustfmt on registry: {}", e);
    }

    Ok(())
}

fn generate_lib_rs(programs: &[ProgramEventInfo]) -> String {
    let mut s = String::from(
        "//! Solana program registry — cross-program event decoding and dispatch.\n\
         //!\n\
         //! **Auto-generated. DO NOT EDIT.**\n\
         \n\
         pub mod event_data;\n\
         pub mod registry;\n\
         pub mod traits;\n\
         \n\
         pub use event_data::EventData;\n\
         pub use event_data::UnknownEvent;\n\
         pub use registry::decode;\n\
         pub use traits::LoggableEvent;\n",
    );

    // Re-export program_id constants
    for prog in programs {
        if prog.has_decoder {
            if let Some(ref pid) = prog.program_id {
                s.push_str(&format!(
                    "\n/// {} program ID.\npub const {}_PROGRAM_ID: &str = \"{}\";\n",
                    prog.module_name.to_pascal_case(),
                    prog.module_name.to_uppercase(),
                    pid
                ));
            }
        }
    }

    s
}

fn generate_traits_rs() -> String {
    let loggable = codegen::generate_loggable_event_trait();

    let tokens = quote::quote! {
        #loggable
    };

    let file = syn::parse2(tokens).expect("Failed to parse traits tokens");
    prettyplease::unparse(&file)
}

fn generate_event_data_rs(programs: &[ProgramEventInfo]) -> String {
    let mut variants = Vec::new();
    let mut event_name_arms = Vec::new();
    let mut log_arms = Vec::new();

    for prog in programs {
        let prog_pascal = prog.module_name.to_pascal_case();
        let prog_mod = quote::format_ident!("{}", prog.module_name);

        for event_name in &prog.event_names {
            // Match existing naming: strip "Event" suffix from IDL name, prefix with program
            let base_name = event_name.strip_suffix("Event").unwrap_or(event_name);
            let variant_name = format!("{}{}", prog_pascal, base_name.to_pascal_case());
            let variant_ident = quote::format_ident!("{}", variant_name);
            let serializable_type =
                quote::format_ident!("{}Serializable", event_name.to_pascal_case());

            variants.push(quote::quote! {
                #variant_ident(#prog_mod::serializable::#serializable_type)
            });

            let variant_name_str = &variant_name;
            let prog_name = &prog.module_name;
            event_name_arms.push(quote::quote! {
                EventData::#variant_ident(_) => #variant_name_str
            });
            log_arms.push(quote::quote! {
                EventData::#variant_ident(ref e) => {
                    log::debug!(
                        "Worker: {}, [{}] {} - Slot: {}, Block: {}, data: {:?}",
                        worker, #prog_name, #variant_name_str, slot, block_height, e
                    );
                }
            });
        }
    }

    let tokens = quote::quote! {
        //! Unified event data enum spanning all Solana programs.
        //!
        //! **Auto-generated. DO NOT EDIT.**

        /// Unified event enum for all Solana programs.
        ///
        /// Each variant is prefixed with the program name to avoid collisions.
        /// Uses adjacently-tagged serde format: `{"type": "PumpfunTrade", "data": {...}}`.
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(tag = "type", content = "data")]
        pub enum EventData {
            #(#variants,)*
            /// Fallback for unrecognized events.
            Unknown(UnknownEvent),
        }

        impl crate::LoggableEvent for EventData {
            fn event_name(&self) -> &'static str {
                match self {
                    #(#event_name_arms,)*
                    EventData::Unknown(_) => "Unknown",
                }
            }

            fn log(&self, worker: usize, slot: u64, block_height: u64) {
                match self {
                    #(#log_arms)*
                    EventData::Unknown(ref e) => {
                        log::debug!(
                            "Worker: {}, [unknown] Unknown - Slot: {}, Block: {}, error: {:?}",
                            worker, slot, block_height, e.parse_error
                        );
                    }
                }
            }
        }

        /// Represents an unrecognized event, preserving the raw data.
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct UnknownEvent {
            /// Base64-encoded raw event data.
            pub raw_data: String,
            /// Optional error message explaining why parsing failed.
            pub parse_error: Option<String>,
        }
    };

    let file = syn::parse2(tokens).expect("Failed to parse event_data tokens");
    prettyplease::unparse(&file)
}

fn generate_registry_rs(programs: &[ProgramEventInfo]) -> String {
    let mut match_arms = Vec::new();

    for prog in programs {
        if !prog.has_decoder {
            continue;
        }

        if let Some(ref pid) = prog.program_id {
            let prog_mod = quote::format_ident!("{}", prog.module_name);
            let prog_pascal = prog.module_name.to_pascal_case();

            // Build conversion arms from per-program ParsedEvent to EventData
            let mut event_conversions = Vec::new();
            for event_name in &prog.event_names {
                let base_name = event_name.strip_suffix("Event").unwrap_or(event_name);
                let variant_name = format!("{}{}", prog_pascal, base_name.to_pascal_case());
                let variant_ident = quote::format_ident!("{}", variant_name);
                let parsed_variant = quote::format_ident!("{}", event_name.to_pascal_case());

                event_conversions.push(quote::quote! {
                    #prog_mod::events::ParsedEvent::#parsed_variant(wrapper) => {
                        results.push(crate::EventData::#variant_ident(wrapper.0.into()));
                    }
                });
            }

            match_arms.push(quote::quote! {
                #pid => {
                    match #prog_mod::decoder::decode_event(data) {
                        Ok(parsed_events) => {
                            for parsed in parsed_events {
                                match parsed {
                                    #(#event_conversions)*
                                }
                            }
                        }
                        Err(#prog_mod::decoder::DecodeError::UnknownDiscriminator(disc)) => {
                            results.push(crate::EventData::Unknown(crate::UnknownEvent {
                                raw_data: base64::engine::general_purpose::STANDARD.encode(data),
                                parse_error: Some(format!("Unknown {} discriminator: {:?}", #prog_pascal, disc)),
                            }));
                        }
                        Err(e) => {
                            results.push(crate::EventData::Unknown(crate::UnknownEvent {
                                raw_data: base64::engine::general_purpose::STANDARD.encode(data),
                                parse_error: Some(format!("{} decode error: {}", #prog_pascal, e)),
                            }));
                        }
                    }
                }
            });
        }
    }

    let tokens = quote::quote! {
        //! Program registry — routes (program_id, data) to the correct decoder.
        //!
        //! **Auto-generated. DO NOT EDIT.**

        use base64::Engine;

        /// Decode raw event data by routing to the correct per-program decoder.
        ///
        /// Returns decoded events as `EventData` variants, or an empty Vec
        /// for unrecognized program IDs (FR-014).
        pub fn decode(program_id: &str, data: &[u8]) -> Vec<crate::EventData> {
            let mut results = Vec::new();

            match program_id {
                #(#match_arms)*
                _ => {
                    // Unknown program_id — return empty (FR-014)
                }
            }

            results
        }
    };

    let file = syn::parse2(tokens).expect("Failed to parse registry tokens");
    prettyplease::unparse(&file)
}

fn generate_cargo_toml(registry_name: &str, programs: &[ProgramEventInfo]) -> String {
    let mut deps = String::new();

    for prog in programs {
        deps.push_str(&format!(
            "{name} = {{ path = \"../{name}\" }}\n",
            name = prog.module_name
        ));
    }

    format!(
        r#"[workspace]

[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "Cross-program Solana event registry — generated by solana-idl-codegen"
license = "MIT OR Apache-2.0"

[dependencies]
serde = {{ version = "^1.0", features = ["derive"] }}
base64 = "^0.22"
log = "^0.4"

{}
"#,
        registry_name, deps
    )
}
