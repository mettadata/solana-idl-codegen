use crate::idl::{ArrayType, *};
use anyhow::Result;
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_str;

/// Represents the generated code split into modules
pub struct GeneratedCode {
    pub lib: String,
    pub accounts: String,
    pub instructions: String,
    pub errors: String,
    pub events: String,
    pub types: String,
}

pub fn generate(idl: &Idl, module_name: &str) -> Result<GeneratedCode> {
    let mut types_tokens = TokenStream::new();
    let mut accounts_tokens = TokenStream::new();
    let mut instructions_tokens = TokenStream::new();
    let mut errors_tokens = TokenStream::new();
    let mut events_tokens = TokenStream::new();

    // Generate module header
    let _module_ident = format_ident!("{}", module_name);

    // Generate account discriminators
    // Note: In new format IDLs, accounts reference types that are generated separately
    // We'll add discriminator impl blocks for accounts that match type names
    let mut account_discriminators = std::collections::HashMap::new();
    if let Some(accounts) = &idl.accounts {
        for account in accounts {
            // Only generate if account has type definition (old format)
            if account.ty.is_some() {
                // Inline type definitions handle their own discriminators
                accounts_tokens.extend(generate_account(account)?);
            } else if let Some(disc) = &account.discriminator {
                // For accounts that reference types (new format), store discriminator
                // to be applied to the matching type later
                account_discriminators.insert(account.name.clone(), disc.clone());
            }
        }
    }

    // Generate types (including those referenced by accounts)
    if let Some(types) = &idl.types {
        for ty in types {
            let mut type_tokens = generate_type_def(ty)?;

            // Check if this type has a discriminator (is an account)
            let has_discriminator = account_discriminators.contains_key(&ty.name);

            // Add discriminator methods if there's a matching account discriminator
            if let Some(disc) = account_discriminators.get(&ty.name) {
                let name = format_ident!("{}", ty.name);
                let disc_bytes = disc.iter().map(|b| quote! { #b });

                // Check if this type uses bytemuck serialization
                let use_bytemuck = ty
                    .serialization
                    .as_ref()
                    .map(|s| s == "bytemuckunsafe" || s == "bytemuck")
                    .unwrap_or(false);

                if use_bytemuck {
                    // For bytemuck types, use bytemuck for deserialization
                    type_tokens.extend(quote! {
                        impl #name {
                            pub const DISCRIMINATOR: [u8; 8] = [#(#disc_bytes),*];

                            pub fn try_from_slice_with_discriminator(data: &[u8]) -> std::io::Result<Self> {
                                if data.len() < 8 {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "Data too short for discriminator",
                                    ));
                                }
                                if data[..8] != Self::DISCRIMINATOR {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "Invalid discriminator",
                                    ));
                                }
                                bytemuck::try_from_bytes::<Self>(&data[8..])
                                    .copied()
                                    .map_err(|e| std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        format!("Bytemuck conversion error: {:?}", e),
                                    ))
                            }

                            pub fn serialize_with_discriminator<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                                writer.write_all(&Self::DISCRIMINATOR)?;
                                writer.write_all(bytemuck::bytes_of(self))
                            }
                        }
                    });
                } else {
                    // For borsh types, use borsh for deserialization
                    type_tokens.extend(quote! {
                        impl #name {
                            pub const DISCRIMINATOR: [u8; 8] = [#(#disc_bytes),*];

                            pub fn try_from_slice_with_discriminator(data: &[u8]) -> std::io::Result<Self> {
                                if data.len() < 8 {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "Data too short for discriminator",
                                    ));
                                }
                                if data[..8] != Self::DISCRIMINATOR {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "Invalid discriminator",
                                    ));
                                }
                                borsh::BorshDeserialize::try_from_slice(&data[8..])
                            }

                            pub fn serialize_with_discriminator<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                                writer.write_all(&Self::DISCRIMINATOR)?;
                                borsh::BorshSerialize::serialize(self, writer)
                            }
                        }
                    });
                }
            }

            // Types with discriminators go to accounts module, others to types module
            if has_discriminator {
                accounts_tokens.extend(type_tokens);
            } else {
                types_tokens.extend(type_tokens);
            }
        }
    }

    // Generate account validation helpers
    if !accounts_tokens.is_empty() {
        accounts_tokens.extend(generate_account_validation_helpers(idl)?);
    }

    // Generate instruction structs and enums
    let has_program_id = idl.get_address().is_some();
    instructions_tokens.extend(generate_instructions(&idl.instructions, has_program_id)?);

    // Generate errors
    if let Some(errors) = &idl.errors {
        errors_tokens.extend(generate_errors(errors)?);
    }

    // Generate events
    if let Some(events) = &idl.events {
        for event in events {
            events_tokens.extend(generate_event(event, &idl.types)?);
        }
        // Generate event parsing helpers
        events_tokens.extend(generate_event_parsing_helpers(events)?);
    }

    // Format each module with appropriate imports
    let types_code = format_module(types_tokens, &[], "types")?;
    let accounts_code = format_module(accounts_tokens, &["types"], "accounts")?;
    let instructions_code =
        format_module(instructions_tokens, &["types", "accounts"], "instructions")?;
    let errors_code = format_module(errors_tokens, &[], "errors")?;
    let events_code = format_module(events_tokens, &["types"], "events")?;

    // Generate lib.rs that re-exports all modules
    let lib_code = generate_lib_module(idl);

    Ok(GeneratedCode {
        lib: lib_code,
        accounts: accounts_code,
        instructions: instructions_code,
        errors: errors_code,
        events: events_code,
        types: types_code,
    })
}

fn format_module(tokens: TokenStream, imports: &[&str], module_type: &str) -> Result<String> {
    if tokens.is_empty() {
        return Ok(String::new());
    }

    // Build import statements based on what this module needs
    // Sort imports alphabetically for rustfmt compliance
    let mut import_tokens = TokenStream::new();
    let mut sorted_imports: Vec<&str> = imports.to_vec();
    sorted_imports.sort();
    for module in sorted_imports {
        match module {
            "accounts" => {
                import_tokens.extend(quote! {
                    #[allow(unused_imports)]
                    use crate::accounts::*;
                });
            }
            "types" => {
                import_tokens.extend(quote! {
                    #[allow(unused_imports)]
                    use crate::types::*;
                });
            }
            _ => {}
        }
    }

    // Different modules need different imports
    let common_imports = match module_type {
        "errors" => {
            // Errors module only needs program_error imports
            quote! {}
        }
        _ => {
            // Other modules need borsh, bytemuck, pubkey
            quote! {
                use borsh::{BorshDeserialize, BorshSerialize};
                #[allow(unused_imports)]
                use bytemuck::{Pod, Zeroable};
                #[allow(unused_imports)]
                use solana_program::instruction::AccountMeta;
                use solana_program::pubkey::Pubkey;
            }
        }
    };

    // Format the code with common imports
    // Note: crate:: imports should come before external imports for rustfmt
    let code = quote! {
        #import_tokens

        #common_imports

        #[allow(clippy::all)]
        #[allow(dead_code)]
        const _: () = {
            // This const block ensures the allows are applied to all items
        };

        #tokens
    };

    // Parse and pretty-print the generated code
    let code_str = code.to_string();
    let syntax_tree: syn::File = parse_str(&code_str).map_err(|e| {
        // Write the unparsed code to a temp file for debugging
        if let Err(write_err) = std::fs::write("/tmp/failed_codegen.rs", &code_str) {
            eprintln!("Failed to write debug file: {}", write_err);
        } else {
            eprintln!("Unparsed code written to /tmp/failed_codegen.rs");
        }
        anyhow::anyhow!("Failed to parse generated code: {}", e)
    })?;
    Ok(prettyplease::unparse(&syntax_tree))
}

fn generate_lib_module(idl: &Idl) -> String {
    let program_id_declaration = if let Some(address) = idl.get_address() {
        format!("solana_program::declare_id!(\"{}\");\n\n", address)
    } else {
        // If no address is provided, use a placeholder comment
        "// Program ID not specified in IDL\n// solana_program::declare_id!(\"YourProgramIdHere\");\n\n".to_string()
    };

    // Note: We don't re-export events::* to avoid ambiguous glob re-exports
    // since events are often also defined in types. Users can access events
    // via the events module directly (e.g., crate::events::EventName)

    format!(
        r#"//! Generated Solana program bindings

{}pub mod accounts;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod types;

// Re-export commonly used types
pub use accounts::*;
pub use errors::*;
pub use instructions::*;
pub use types::*;

// Helper function for serde serialization of Pubkey as string
#[cfg(feature = "serde")]
pub fn serialize_pubkey_as_string<S>(
    pubkey: &solana_program::pubkey::Pubkey,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{{
    serializer.serialize_str(&pubkey.to_string())
}}
"#,
        program_id_declaration
    )
}

/// Check if a type is an array with more than 32 elements
/// (serde only supports arrays up to size 32 by default)
fn is_large_array(ty: &IdlType) -> bool {
    match ty {
        IdlType::Array { array } => match array {
            ArrayType::Tuple((_, size)) => *size > 32,
        },
        _ => false,
    }
}

/// Check if a struct has any large arrays
fn has_large_arrays_in_struct(fields: &StructFields) -> bool {
    match fields {
        StructFields::Named(fields) => fields.iter().any(|f| is_large_array(&f.ty)),
        StructFields::Tuple(types) => types.iter().any(is_large_array),
    }
}

fn generate_type_def(ty: &TypeDef) -> Result<TokenStream> {
    let name = format_ident!("{}", ty.name);
    let docs = generate_docs(ty.docs.as_ref());

    // Determine serialization type
    let use_bytemuck = ty
        .serialization
        .as_ref()
        .map(|s| s == "bytemuckunsafe" || s == "bytemuck")
        .unwrap_or(false);

    // Check if type is packed (for repr attribute)
    let is_packed = ty.repr.as_ref().and_then(|r| r.packed).unwrap_or(false);

    let repr_attr = if use_bytemuck && is_packed {
        quote! { #[repr(C, packed)] }
    } else if use_bytemuck {
        quote! { #[repr(C)] }
    } else {
        quote! {}
    };

    match &ty.ty {
        TypeDefType::Struct { fields } => {
            // Check if this struct has large arrays (> 32 elements)
            // If so, we can't derive serde automatically
            let has_large_arrays = has_large_arrays_in_struct(fields);

            match fields {
                StructFields::Named(fields) => {
                    let field_tokens: Vec<_> = fields
                        .iter()
                        .map(|f| {
                            let field_name = format_ident!("{}", f.name.to_snake_case());
                            let field_type = map_idl_type(&f.ty);
                            let field_docs = generate_docs(f.docs.as_ref());

                            quote! {
                                #field_docs
                                pub #field_name: #field_type
                            }
                        })
                        .collect();

                    if use_bytemuck {
                        // For bytemuck types, we need unsafe implementations for Pod and Zeroable
                        let safety_doc = concat!(
                            "SAFETY: Pod and Zeroable require unsafe impl because they make guarantees about memory layout.\n",
                            "This is safe because:\n",
                            "1. The struct is #[repr(C)] or #[repr(C, packed)], ensuring predictable memory layout\n",
                            "2. All fields are themselves Pod types (verified by IDL)\n",
                            "3. No padding bytes contain uninitialized data\n",
                            "4. The type can be safely transmuted to/from bytes\n",
                            "\n",
                            "These traits enable zero-copy deserialization of blockchain account data,\n",
                            "which is critical for performance when processing large numbers of accounts."
                        );
                        Ok(quote! {
                            #docs
                            #repr_attr
                            #[derive(Debug, Clone, Copy, PartialEq)]
                            pub struct #name {
                                #(#field_tokens),*
                            }

                            #[doc = #safety_doc]
                            unsafe impl bytemuck::Pod for #name {}
                            unsafe impl bytemuck::Zeroable for #name {}
                        })
                    } else if has_large_arrays {
                        // Skip serde for structs with large arrays
                        Ok(quote! {
                            #docs
                            #repr_attr
                            #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                            pub struct #name {
                                #(#field_tokens),*
                            }
                        })
                    } else {
                        Ok(quote! {
                            #docs
                            #repr_attr
                            #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
                            pub struct #name {
                                #(#field_tokens),*
                            }
                        })
                    }
                }
                StructFields::Tuple(types) => {
                    let field_types: Vec<_> = types.iter().map(map_idl_type).collect();

                    if use_bytemuck {
                        // For bytemuck types, we need unsafe implementations for Pod and Zeroable
                        let safety_doc = concat!(
                            "SAFETY: Pod and Zeroable require unsafe impl because they make guarantees about memory layout.\n",
                            "This is safe because:\n",
                            "1. The struct is #[repr(C)] or #[repr(C, packed)], ensuring predictable memory layout\n",
                            "2. All fields are themselves Pod types (verified by IDL)\n",
                            "3. No padding bytes contain uninitialized data\n",
                            "4. The type can be safely transmuted to/from bytes\n",
                            "\n",
                            "These traits enable zero-copy deserialization of blockchain account data,\n",
                            "which is critical for performance when processing large numbers of accounts."
                        );
                        Ok(quote! {
                            #docs
                            #repr_attr
                            #[derive(Debug, Clone, Copy, PartialEq)]
                            pub struct #name(#(pub #field_types),*);

                            #[doc = #safety_doc]
                            unsafe impl bytemuck::Pod for #name {}
                            unsafe impl bytemuck::Zeroable for #name {}
                        })
                    } else if has_large_arrays {
                        // Skip serde for structs with large arrays
                        Ok(quote! {
                            #docs
                            #repr_attr
                            #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                            pub struct #name(#(pub #field_types),*);
                        })
                    } else {
                        Ok(quote! {
                            #docs
                            #repr_attr
                            #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
                            pub struct #name(#(pub #field_types),*);
                        })
                    }
                }
            }
        }
        TypeDefType::Enum { variants } => {
            let variant_tokens: Vec<_> = variants
                .iter()
                .map(|v| {
                    let variant_name = format_ident!("{}", v.name.to_pascal_case());
                    match &v.fields {
                        Some(EnumFields::Named(fields)) => {
                            let field_tokens: Vec<_> = fields
                                .iter()
                                .map(|f| {
                                    let field_name = format_ident!("{}", f.name.to_snake_case());
                                    let field_type = map_idl_type(&f.ty);
                                    quote! { #field_name: #field_type }
                                })
                                .collect();
                            quote! { #variant_name { #(#field_tokens),* } }
                        }
                        Some(EnumFields::Tuple(types)) => {
                            let type_tokens: Vec<_> = types.iter().map(map_idl_type).collect();
                            quote! { #variant_name(#(#type_tokens),*) }
                        }
                        None => quote! { #variant_name },
                    }
                })
                .collect();

            if use_bytemuck {
                // For bytemuck enums, we need unsafe implementations
                let safety_doc = concat!(
                    "SAFETY: Pod and Zeroable require unsafe impl because they make guarantees about memory layout.\n",
                    "This is safe because:\n",
                    "1. The enum is #[repr(C)] or #[repr(C, packed)], ensuring predictable memory layout\n",
                    "2. All variant fields are themselves Pod types (verified by IDL)\n",
                    "3. No padding bytes contain uninitialized data\n",
                    "4. The type can be safely transmuted to/from bytes\n",
                    "\n",
                    "These traits enable zero-copy deserialization of blockchain account data,\n",
                    "which is critical for performance when processing large numbers of accounts."
                );
                Ok(quote! {
                    #docs
                    #repr_attr
                    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                    pub enum #name {
                        #(#variant_tokens),*
                    }

                    #[doc = #safety_doc]
                    unsafe impl bytemuck::Pod for #name {}
                    unsafe impl bytemuck::Zeroable for #name {}
                })
            } else {
                Ok(quote! {
                    #docs
                    #repr_attr
                    #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
                    pub enum #name {
                        #(#variant_tokens),*
                    }
                })
            }
        }
    }
}

fn generate_account(account: &Account) -> Result<TokenStream> {
    // In old format IDLs, accounts can have type definitions
    // In new format IDLs, they're just references (discriminators added to types directly)
    if let Some(ty) = &account.ty {
        let mut tokens = generate_type_def(&TypeDef {
            name: account.name.clone(),
            docs: account.docs.clone(),
            ty: ty.clone(),
            serialization: None,
            repr: None,
        })?;

        // Add discriminator methods if discriminator is present
        if let Some(disc) = &account.discriminator {
            let name = format_ident!("{}", account.name);
            let disc_bytes = disc.iter().map(|b| quote! { #b });

            tokens.extend(quote! {
                impl #name {
                    pub const DISCRIMINATOR: [u8; 8] = [#(#disc_bytes),*];

                    pub fn try_from_slice_with_discriminator(data: &[u8]) -> std::io::Result<Self> {
                        if data.len() < 8 {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Data too short for discriminator",
                            ));
                        }
                        if data[..8] != Self::DISCRIMINATOR {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid discriminator",
                            ));
                        }
                        borsh::BorshDeserialize::try_from_slice(&data[8..])
                    }

                    pub fn serialize_with_discriminator<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                        writer.write_all(&Self::DISCRIMINATOR)?;
                        borsh::BorshSerialize::serialize(self, writer)
                    }
                }
            });
        }

        Ok(tokens)
    } else {
        // In new format, accounts are references to types defined elsewhere
        // Discriminators are added directly to the type definitions
        Ok(TokenStream::new())
    }
}

fn generate_account_validation_helpers(idl: &Idl) -> Result<TokenStream> {
    let program_id_expr = if let Some(_addr) = idl.get_address() {
        quote! { crate::ID }
    } else {
        return Ok(TokenStream::new()); // Can't validate without program ID
    };

    // Collect all account types with discriminators
    let mut account_validations = Vec::new();
    // Track which account names have already been processed to avoid duplicates
    let mut processed_accounts = std::collections::HashSet::new();

    // Check accounts from accounts array (old format)
    if let Some(accounts) = &idl.accounts {
        for account in accounts {
            // Only generate validation methods if account has a discriminator
            // (validation methods reference DISCRIMINATOR and try_from_slice_with_discriminator)
            if account.discriminator.is_some() {
                let name = format_ident!("{}", account.name);
                let docs = generate_docs(account.docs.as_ref());

                // Track that we've processed this account
                processed_accounts.insert(account.name.clone());

                account_validations.push(quote! {
                    #docs
                    impl #name {
                        /// Validate that an AccountInfo matches this account type
                        ///
                        /// This function checks:
                        /// - The account owner matches the program ID
                        /// - The account data starts with the correct discriminator
                        /// - The account data is long enough to contain the discriminator
                        ///
                        /// # Example
                        /// ```no_run
                        /// use solana_program::account_info::AccountInfo;
                        /// use crate::accounts::*;
                        ///
                        /// fn validate_account(account_info: &AccountInfo) -> Result<(), ValidationError> {
                        ///     #name::validate_account_info(account_info)?;
                        ///     Ok(())
                        /// }
                        /// ```
                        pub fn validate_account_info(
                            account_info: &solana_program::account_info::AccountInfo,
                        ) -> Result<(), ValidationError> {
                            // Check owner
                            if account_info.owner != &#program_id_expr {
                                return Err(ValidationError::InvalidOwner {
                                    expected: #program_id_expr,
                                    actual: *account_info.owner,
                                });
                            }

                            // Check discriminator
                            let data = account_info.data.borrow();
                            if data.len() < 8 {
                                return Err(ValidationError::DataTooShort {
                                    expected: 8,
                                    actual: data.len(),
                                });
                            }

                            if data[..8] != Self::DISCRIMINATOR {
                                return Err(ValidationError::InvalidDiscriminator {
                                    expected: Self::DISCRIMINATOR,
                                    actual: <[u8; 8]>::try_from(&data[..8])
                                        .map_err(|_| ValidationError::DataTooShort {
                                            expected: 8,
                                            actual: data.len(),
                                        })?,
                                });
                            }

                            Ok(())
                        }

                        /// Validate and deserialize an account from AccountInfo
                        ///
                        /// This is a convenience method that combines validation and deserialization.
                        ///
                        /// # Example
                        /// ```no_run
                        /// use solana_program::account_info::AccountInfo;
                        /// use crate::accounts::*;
                        ///
                        /// fn load_account(account_info: &AccountInfo) -> Result<#name, ValidationError> {
                        ///     #name::try_from_account_info(account_info)
                        /// }
                        /// ```
                        pub fn try_from_account_info(
                            account_info: &solana_program::account_info::AccountInfo,
                        ) -> Result<Self, ValidationError> {
                            Self::validate_account_info(account_info)?;
                            let data = account_info.data.borrow();
                            Self::try_from_slice_with_discriminator(&data)
                                .map_err(|e| ValidationError::DeserializationError(e.to_string()))
                        }
                    }
                });
            }
        }
    }

    // Note: Types with discriminators that are referenced in the accounts array
    // are already handled above. We don't need to process them again here.

    if account_validations.is_empty() {
        return Ok(TokenStream::new());
    }

    Ok(quote! {
        /// Error type for account validation
        #[derive(Debug, thiserror::Error)]
        pub enum ValidationError {
            #[error("Invalid account owner. Expected: {expected}, Actual: {actual}")]
            InvalidOwner {
                expected: solana_program::pubkey::Pubkey,
                actual: solana_program::pubkey::Pubkey,
            },
            #[error("Account data too short. Expected at least {expected} bytes, got {actual}")]
            DataTooShort {
                expected: usize,
                actual: usize,
            },
            #[error("Invalid discriminator. Expected: {expected:?}, Actual: {actual:?}")]
            InvalidDiscriminator {
                expected: [u8; 8],
                actual: [u8; 8],
            },
            #[error("Deserialization error: {0}")]
            DeserializationError(String),
        }

        #(#account_validations)*
    })
}

fn generate_instructions(
    instructions: &[Instruction],
    has_program_id: bool,
) -> Result<TokenStream> {
    let mut tokens = TokenStream::new();

    // Generate module-level discriminator constants and IxData wrapper structs for each instruction
    for (idx, ix) in instructions.iter().enumerate() {
        let ix_name_snake = ix.name.to_snake_case();
        let ix_name_pascal = ix.name.to_pascal_case();
        let discm_const_name = format_ident!("{}_IX_DISCM", ix_name_snake.to_uppercase());
        let ix_data_struct = format_ident!("{}IxData", ix_name_pascal);

        // Get discriminator bytes
        let discriminator_bytes: Vec<u8> = if let Some(disc) = &ix.discriminator {
            disc.clone()
        } else {
            // Use index as discriminator if not provided (old format)
            (idx as u64).to_le_bytes().to_vec()
        };

        let disc_bytes = discriminator_bytes.iter().map(|b| quote! { #b });

        // Generate module-level discriminator constant
        tokens.extend(quote! {
            pub const #discm_const_name: [u8; 8] = [#(#disc_bytes),*];
        });

        // Generate IxData wrapper struct
        if ix.args.is_empty() {
            // No-args instruction
            tokens.extend(quote! {
                #[derive(Clone, Debug, PartialEq)]
                pub struct #ix_data_struct;

                impl #ix_data_struct {
                    pub fn deserialize(buf: &[u8]) -> std::io::Result<Self> {
                        use std::io::Read;
                        let mut reader = buf;
                        let mut maybe_discm = [0u8; 8];
                        reader.read_exact(&mut maybe_discm)?;
                        if maybe_discm != #discm_const_name {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!(
                                    "discm does not match. Expected: {:?}. Received: {:?}",
                                    #discm_const_name, maybe_discm
                                ),
                            ));
                        }
                        Ok(Self)
                    }

                    pub fn serialize<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
                        writer.write_all(&#discm_const_name)
                    }

                    pub fn try_to_vec(&self) -> std::io::Result<Vec<u8>> {
                        let mut data = Vec::new();
                        self.serialize(&mut data)?;
                        Ok(data)
                    }
                }
            });
        } else {
            // Instruction with args
            let args_struct = format_ident!("{}IxArgs", ix_name_pascal);

            tokens.extend(quote! {
                #[derive(Clone, Debug, PartialEq)]
                pub struct #ix_data_struct(pub #args_struct);

                impl From<#args_struct> for #ix_data_struct {
                    fn from(args: #args_struct) -> Self {
                        Self(args)
                    }
                }

                impl #ix_data_struct {
                    pub fn deserialize(buf: &[u8]) -> std::io::Result<Self> {
                        use std::io::Read;
                        let mut reader = buf;
                        let mut maybe_discm = [0u8; 8];
                        reader.read_exact(&mut maybe_discm)?;
                        if maybe_discm != #discm_const_name {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!(
                                    "discm does not match. Expected: {:?}. Received: {:?}",
                                    #discm_const_name, maybe_discm
                                ),
                            ));
                        }
                        Ok(Self(#args_struct::deserialize(&mut reader)?))
                    }

                    pub fn serialize<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
                        writer.write_all(&#discm_const_name)?;
                        self.0.serialize(&mut writer)
                    }

                    pub fn try_to_vec(&self) -> std::io::Result<Vec<u8>> {
                        let mut data = Vec::new();
                        self.serialize(&mut data)?;
                        Ok(data)
                    }
                }
            });
        }
    }

    // Generate instruction enum
    let instruction_variants: Vec<_> = instructions
        .iter()
        .map(|ix| {
            let variant_name = format_ident!("{}", ix.name.to_pascal_case());
            if ix.args.is_empty() {
                quote! { #variant_name }
            } else {
                let args_struct = format_ident!("{}IxArgs", ix.name.to_pascal_case());
                quote! { #variant_name(#args_struct) }
            }
        })
        .collect();

    // Generate discriminator match arms for serialization
    let serialize_arms: Vec<_> = instructions
        .iter()
        .map(|ix| {
            let variant_name = format_ident!("{}", ix.name.to_pascal_case());
            let discm_const_name =
                format_ident!("{}_IX_DISCM", ix.name.to_snake_case().to_uppercase());

            if ix.args.is_empty() {
                quote! {
                    Self::#variant_name => {
                        writer.write_all(&#discm_const_name)?;
                        Ok(())
                    }
                }
            } else {
                quote! {
                    Self::#variant_name(args) => {
                        writer.write_all(&#discm_const_name)?;
                        args.serialize(writer)
                    }
                }
            }
        })
        .collect();

    // Generate discriminator match for deserialization
    let deserialize_arms: Vec<_> = instructions
        .iter()
        .enumerate()
        .map(|(idx, ix)| {
            let variant_name = format_ident!("{}", ix.name.to_pascal_case());
            let discriminator_bytes = if let Some(disc) = &ix.discriminator {
                disc.clone()
            } else {
                (idx as u64).to_le_bytes().to_vec()
            };

            let disc_pattern = discriminator_bytes.iter().map(|b| quote! { #b });

            if ix.args.is_empty() {
                quote! {
                    [#(#disc_pattern),*] => Ok(Self::#variant_name)
                }
            } else {
                let args_struct = format_ident!("{}IxArgs", ix.name.to_pascal_case());
                quote! {
                    [#(#disc_pattern),*] => {
                        let args = #args_struct::deserialize(&mut buf)?;
                        Ok(Self::#variant_name(args))
                    }
                }
            }
        })
        .collect();

    tokens.extend(quote! {
        #[derive(Debug, Clone, PartialEq)]
        pub enum Instruction {
            #(#instruction_variants),*
        }

        impl Instruction {
            pub fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                match self {
                    #(#serialize_arms),*
                }
            }

            pub fn try_from_slice(data: &[u8]) -> std::io::Result<Self> {
                if data.len() < 8 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Data too short for instruction discriminator",
                    ));
                }

                use borsh::BorshDeserialize;
                let mut buf = &data[8..];

                match &data[..8] {
                    #(#deserialize_arms),*,
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unknown instruction discriminator",
                    )),
                }
            }
        }
    });

    // Generate args structs for each instruction
    for ix in instructions {
        if !ix.args.is_empty() {
            let args_struct = format_ident!("{}IxArgs", ix.name.to_pascal_case());
            let field_tokens: Vec<_> = ix
                .args
                .iter()
                .map(|arg| {
                    let field_name = format_ident!("{}", arg.name.to_snake_case());
                    let field_type = map_idl_type(&arg.ty);
                    quote! {
                        pub #field_name: #field_type
                    }
                })
                .collect();

            tokens.extend(quote! {
                #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
                pub struct #args_struct {
                    #(#field_tokens),*
                }
            });
        }

        // Generate Keys struct for each instruction (for building instructions)
        let keys_struct = format_ident!("{}Keys", ix.name.to_pascal_case());
        let account_fields: Vec<_> = ix
            .accounts
            .iter()
            .map(|acc| {
                let field_name = format_ident!("{}", acc.name.to_snake_case());
                let docs = generate_docs(acc.docs.as_ref());
                quote! {
                    #docs
                    pub #field_name: Pubkey
                }
            })
            .collect();

        // Generate const for accounts length
        let accounts_len_const =
            format_ident!("{}_IX_ACCOUNTS_LEN", ix.name.to_snake_case().to_uppercase());
        let accounts_len = ix.accounts.len();

        tokens.extend(quote! {
            pub const #accounts_len_const: usize = #accounts_len;

            #[derive(Debug, Clone, PartialEq)]
            pub struct #keys_struct {
                #(#account_fields),*
            }
        });

        // Generate From<Keys> for [AccountMeta; N] conversion
        let account_metas: Vec<_> = ix
            .accounts
            .iter()
            .map(|acc| {
                let field_name = format_ident!("{}", acc.name.to_snake_case());
                let is_signer = acc.signer;
                let is_writable = acc.writable;
                quote! {
                    AccountMeta {
                        pubkey: keys.#field_name,
                        is_signer: #is_signer,
                        is_writable: #is_writable,
                    }
                }
            })
            .collect();

        if !ix.accounts.is_empty() {
            tokens.extend(quote! {
                impl From<#keys_struct> for [AccountMeta; #accounts_len_const] {
                    fn from(keys: #keys_struct) -> Self {
                        [
                            #(#account_metas),*
                        ]
                    }
                }
            });
        }

        // Generate instruction builder functions
        let ix_name_snake = ix.name.to_snake_case();
        let ix_fn = format_ident!("{}_ix", ix_name_snake);
        let ix_with_program_id_fn = format_ident!("{}_ix_with_program_id", ix_name_snake);
        let ix_data_struct = format_ident!("{}IxData", ix.name.to_pascal_case());

        if ix.args.is_empty() {
            // No-args instruction builder
            tokens.extend(quote! {
                pub fn #ix_with_program_id_fn(
                    program_id: Pubkey,
                    keys: #keys_struct,
                ) -> std::io::Result<solana_program::instruction::Instruction> {
                    let metas: [AccountMeta; #accounts_len_const] = keys.into();
                    Ok(solana_program::instruction::Instruction {
                        program_id,
                        accounts: Vec::from(metas),
                        data: #ix_data_struct.try_to_vec()?,
                    })
                }
            });

            // Only generate the version without program_id if we have a program ID
            if has_program_id {
                tokens.extend(quote! {
                    pub fn #ix_fn(keys: #keys_struct) -> std::io::Result<solana_program::instruction::Instruction> {
                        #ix_with_program_id_fn(crate::ID, keys)
                    }
                });
            }
        } else {
            // Instruction with args
            let args_struct = format_ident!("{}IxArgs", ix.name.to_pascal_case());

            tokens.extend(quote! {
                pub fn #ix_with_program_id_fn(
                    program_id: Pubkey,
                    keys: #keys_struct,
                    args: #args_struct,
                ) -> std::io::Result<solana_program::instruction::Instruction> {
                    let metas: [AccountMeta; #accounts_len_const] = keys.into();
                    let data: #ix_data_struct = args.into();
                    Ok(solana_program::instruction::Instruction {
                        program_id,
                        accounts: Vec::from(metas),
                        data: data.try_to_vec()?,
                    })
                }
            });

            // Only generate the version without program_id if we have a program ID
            if has_program_id {
                tokens.extend(quote! {
                    pub fn #ix_fn(
                        keys: #keys_struct,
                        args: #args_struct,
                    ) -> std::io::Result<solana_program::instruction::Instruction> {
                        #ix_with_program_id_fn(crate::ID, keys, args)
                    }
                });
            }
        }
    }

    Ok(tokens)
}

fn generate_errors(errors: &[Error]) -> Result<TokenStream> {
    let error_variants: Vec<_> = errors
        .iter()
        .map(|e| {
            let variant_name = format_ident!("{}", e.name.to_pascal_case());
            let msg = e.msg.as_deref().unwrap_or(&e.name);
            let code = e.code;
            quote! {
                #[error(#msg)]
                #variant_name = #code
            }
        })
        .collect();

    Ok(quote! {
        use solana_program::program_error::ProgramError;
        use thiserror::Error;

        #[derive(Clone, Copy, Debug, Eq, Error, num_derive::FromPrimitive, PartialEq)]
        #[repr(u32)]
        pub enum ErrorCode {
            #(#error_variants),*
        }

        impl From<ErrorCode> for ProgramError {
            fn from(e: ErrorCode) -> Self {
                ProgramError::Custom(e as u32)
            }
        }
    })
}

fn generate_event(event: &Event, types: &Option<Vec<TypeDef>>) -> Result<TokenStream> {
    // Helper function to check if a type is Pubkey
    fn is_pubkey_type(ty: &IdlType) -> bool {
        match ty {
            IdlType::Simple(s) => matches!(s.as_str(), "publicKey" | "pubkey" | "Pubkey"),
            _ => false,
        }
    }

    // Helper function to generate field tokens with Pubkey serialization
    fn generate_field_tokens(fields: &[EventField]) -> Vec<TokenStream> {
        fields
            .iter()
            .map(|f| {
                let field_name = format_ident!("{}", f.name.to_snake_case());
                let field_type = map_idl_type(&f.ty);

                // Add custom serde attribute for Pubkey fields
                let serde_attr = if is_pubkey_type(&f.ty) {
                    quote! {
                        #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_pubkey_as_string"))]
                    }
                } else {
                    quote! {}
                };

                quote! {
                    #serde_attr
                    pub #field_name: #field_type
                }
            })
            .collect()
    }

    // Helper function to generate field tokens from struct fields
    fn generate_field_tokens_from_struct_fields(fields: &StructFields) -> Vec<TokenStream> {
        match fields {
            StructFields::Named(named_fields) => {
                named_fields
                    .iter()
                    .map(|f| {
                        let field_name = format_ident!("{}", f.name.to_snake_case());
                        let field_type = map_idl_type(&f.ty);

                        // Add custom serde attribute for Pubkey fields
                        let serde_attr = if is_pubkey_type(&f.ty) {
                            quote! {
                                #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_pubkey_as_string"))]
                            }
                        } else {
                            quote! {}
                        };

                        quote! {
                            #serde_attr
                            pub #field_name: #field_type
                        }
                    })
                    .collect()
            }
            StructFields::Tuple(_) => {
                // Tuple structs as events are unusual, just skip them
                vec![]
            }
        }
    }

    let name = format_ident!("{}", event.name);
    let wrapper_name = format_ident!("{}Event", event.name);

    // Determine if we have fields to generate
    let field_tokens = if let Some(fields) = &event.fields {
        // Old format: fields are directly in the event
        generate_field_tokens(fields)
    } else if let Some(types) = types {
        // New format: look for the type definition
        if let Some(type_def) = types.iter().find(|t| t.name == event.name) {
            // Found the type definition for this event
            match &type_def.ty {
                TypeDefType::Struct { fields } => generate_field_tokens_from_struct_fields(fields),
                TypeDefType::Enum { .. } => {
                    // Enums as events are unusual, skip them
                    return Ok(TokenStream::new());
                }
            }
        } else {
            // No fields and no matching type definition
            return Ok(TokenStream::new());
        }
    } else {
        // No fields and no types to look up
        return Ok(TokenStream::new());
    };

    // If we have no fields, return empty
    if field_tokens.is_empty() {
        return Ok(TokenStream::new());
    }

    let mut tokens = TokenStream::new();

    // Generate module-level discriminator constant
    if let Some(disc) = &event.discriminator {
        let discm_const =
            format_ident!("{}_EVENT_DISCM", event.name.to_snake_case().to_uppercase());
        let disc_bytes = disc.iter().map(|b| quote! { #b });

        tokens.extend(quote! {
            pub const #discm_const: [u8; 8] = [#(#disc_bytes),*];
        });
    }

    // Generate data struct with enhanced documentation
    let enhanced_docs = format!("Event: {}\n///\n/// # Usage\n/// ```no_run\n/// use crate::events::*;\n///\n/// // Parse event from transaction data\n/// let event = parse_event(&event_data)?;\n/// match event {{\n///     ParsedEvent::{}(e) => println!(\"Event: {{:?}}\", e),\n///     _ => {{}}\n/// }}\n/// ```", event.name, event.name.to_pascal_case());

    tokens.extend(quote! {
        #[doc = #enhanced_docs]
        #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct #name {
            #(#field_tokens),*
        }
    });

    // Generate wrapper struct with discriminator handling
    if let Some(_disc) = &event.discriminator {
        let discm_const =
            format_ident!("{}_EVENT_DISCM", event.name.to_snake_case().to_uppercase());

        tokens.extend(quote! {
            #[derive(Clone, Debug, PartialEq)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            pub struct #wrapper_name(pub #name);

            impl borsh::BorshSerialize for #wrapper_name {
                fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                    #discm_const.serialize(writer)?;
                    self.0.serialize(writer)
                }
            }

            impl #wrapper_name {
                pub fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
                    let maybe_discm = <[u8; 8]>::deserialize(buf)?;
                    if maybe_discm != #discm_const {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "discm does not match. Expected: {:?}. Received: {:?}",
                                #discm_const, maybe_discm
                            ),
                        ));
                    }
                    Ok(Self(#name::deserialize(buf)?))
                }
            }
        });
    }

    Ok(tokens)
}

fn generate_event_parsing_helpers(events: &[Event]) -> Result<TokenStream> {
    if events.is_empty() {
        return Ok(TokenStream::new());
    }

    // Collect all events with discriminators
    let mut event_variants = Vec::new();
    let mut parse_arms = Vec::new();
    let mut parse_arms_with_size = Vec::new();

    for event in events {
        if event.discriminator.is_some() {
            let wrapper_name = format_ident!("{}Event", event.name);
            let variant_name = format_ident!("{}", event.name.to_pascal_case());
            let discm_const =
                format_ident!("{}_EVENT_DISCM", event.name.to_snake_case().to_uppercase());

            event_variants.push(quote! {
                #variant_name(#wrapper_name)
            });

            parse_arms.push(quote! {
                #discm_const => {
                    let mut data_slice = data;
                    match #wrapper_name::deserialize(&mut data_slice) {
                        Ok(event) => Ok(ParsedEvent::#variant_name(event)),
                        Err(e) => Err(EventParseError::DeserializationError(format!("Failed to deserialize {}: {}", stringify!(#variant_name), e))),
                    }
                }
            });

            // Generate arms that track bytes consumed for parse_event_with_size
            parse_arms_with_size.push(quote! {
                #discm_const => {
                    let initial_len = data_slice.len();
                    match #wrapper_name::deserialize(&mut data_slice) {
                        Ok(event) => {
                            let bytes_consumed = initial_len - data_slice.len();
                            Ok((ParsedEvent::#variant_name(event), bytes_consumed))
                        }
                        Err(e) => Err(EventParseError::DeserializationError(format!("Failed to deserialize {}: {}", stringify!(#variant_name), e))),
                    }
                }
            });
        }
    }

    if event_variants.is_empty() {
        return Ok(TokenStream::new());
    }

    Ok(quote! {
        /// Enum representing all parsed events from this program
        #[derive(Debug, Clone, PartialEq)]
        pub enum ParsedEvent {
            #(#event_variants),*
        }

        /// Error type for event parsing
        #[derive(Debug, thiserror::Error)]
        pub enum EventParseError {
            #[error("Data too short for discriminator")]
            DataTooShort,
            #[error("Unknown event discriminator: {0:?}")]
            UnknownDiscriminator([u8; 8]),
            #[error("Deserialization error: {0}")]
            DeserializationError(String),
        }

        /// Parse an event from raw bytes (including discriminator)
        ///
        /// # Example
        /// ```no_run
        /// use crate::events::*;
        ///
        /// let event_data: &[u8] = /* event data from transaction log */;
        /// match parse_event(event_data) {
        ///     Ok(ParsedEvent::CreateEvent(event)) => {
        ///         println!("Created: {:?}", event.0);
        ///     }
        ///     Ok(ParsedEvent::TradeEvent(event)) => {
        ///         println!("Traded: {:?}", event.0);
        ///     }
        ///     Err(e) => eprintln!("Failed to parse event: {}", e),
        /// }
        /// ```
        pub fn parse_event(data: &[u8]) -> Result<ParsedEvent, EventParseError> {
            if data.len() < 8 {
                return Err(EventParseError::DataTooShort);
            }

            let discm = <[u8; 8]>::try_from(&data[..8])
                .map_err(|_| EventParseError::DataTooShort)?;

            match discm {
                #(#parse_arms),*
                _ => Err(EventParseError::UnknownDiscriminator(discm)),
            }
        }

        /// Helper function to parse an event and return the number of bytes consumed
        fn parse_event_with_size(data: &[u8]) -> Result<(ParsedEvent, usize), EventParseError> {
            if data.len() < 8 {
                return Err(EventParseError::DataTooShort);
            }

            let discm = <[u8; 8]>::try_from(&data[..8])
                .map_err(|_| EventParseError::DataTooShort)?;

            // Create a mutable slice to track bytes consumed
            let mut data_slice = data;

            match discm {
                #(#parse_arms_with_size),*
                _ => Err(EventParseError::UnknownDiscriminator(discm)),
            }
        }

        /// Parse events from raw transaction log data
        ///
        /// This function attempts to parse events from a slice of raw bytes.
        /// For Solana transaction logs, you typically need to:
        /// 1. Extract program data from logs (often base64-encoded)
        /// 2. Decode the base64 data
        /// 3. Call this function with the decoded bytes
        ///
        /// This function correctly handles events of varying sizes by tracking
        /// the actual bytes consumed during deserialization, rather than using
        /// hardcoded size estimates.
        ///
        /// # Example
        /// ```no_run
        /// use crate::events::*;
        ///
        /// // From transaction logs, extract and decode program data
        /// // let decoded_data: Vec<u8> = /* decode base64 from logs */;
        /// // let events = parse_events_from_data(&decoded_data)?;
        ///
        /// // Or parse a single event
        /// // let event = parse_event(&decoded_data)?;
        /// ```
        pub fn parse_events_from_data(data: &[u8]) -> Vec<Result<ParsedEvent, EventParseError>> {
            let mut events = Vec::new();
            let mut offset = 0;

            while offset < data.len() {
                if data.len() - offset < 8 {
                    break;
                }

                match parse_event_with_size(&data[offset..]) {
                    Ok((event, bytes_consumed)) => {
                        events.push(Ok(event));
                        offset += bytes_consumed;
                    }
                    Err(e) => {
                        events.push(Err(e));
                        break;
                    }
                }
            }

            events
        }
    })
}

fn map_idl_type(ty: &IdlType) -> TokenStream {
    match ty {
        IdlType::Simple(s) => match s.as_str() {
            "bool" => quote! { bool },
            "u8" => quote! { u8 },
            "i8" => quote! { i8 },
            "u16" => quote! { u16 },
            "i16" => quote! { i16 },
            "u32" => quote! { u32 },
            "i32" => quote! { i32 },
            "u64" => quote! { u64 },
            "i64" => quote! { i64 },
            "u128" => quote! { u128 },
            "i128" => quote! { i128 },
            "f32" => quote! { f32 },
            "f64" => quote! { f64 },
            "string" => quote! { String },
            "publicKey" | "pubkey" | "Pubkey" => quote! { Pubkey },
            "bytes" => quote! { Vec<u8> },
            _ => {
                let ident = format_ident!("{}", s);
                quote! { #ident }
            }
        },
        IdlType::Vec { vec } => {
            let inner = map_idl_type(vec);
            quote! { Vec<#inner> }
        }
        IdlType::Option { option } => {
            let inner = map_idl_type(option);
            quote! { Option<#inner> }
        }
        IdlType::Array { array } => match array {
            ArrayType::Tuple((inner, size)) => {
                let inner_ty = map_idl_type(inner);
                quote! { [#inner_ty; #size] }
            }
        },
        IdlType::Defined { defined } => {
            let ident = format_ident!("{}", defined.name());
            quote! { #ident }
        }
    }
}

fn generate_docs(docs: Option<&Vec<String>>) -> TokenStream {
    if let Some(doc_lines) = docs {
        let docs: Vec<_> = doc_lines
            .iter()
            .filter(|line| !line.is_empty())
            .map(|line| quote! { #[doc = #line] })
            .collect();
        quote! { #(#docs)* }
    } else {
        TokenStream::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    // ============================================================================
    // Helper Functions Tests
    // ============================================================================

    #[test]
    fn test_map_idl_type_primitives() {
        let test_cases = vec![
            (IdlType::Simple("bool".to_string()), quote! { bool }),
            (IdlType::Simple("u8".to_string()), quote! { u8 }),
            (IdlType::Simple("i8".to_string()), quote! { i8 }),
            (IdlType::Simple("u16".to_string()), quote! { u16 }),
            (IdlType::Simple("i16".to_string()), quote! { i16 }),
            (IdlType::Simple("u32".to_string()), quote! { u32 }),
            (IdlType::Simple("i32".to_string()), quote! { i32 }),
            (IdlType::Simple("u64".to_string()), quote! { u64 }),
            (IdlType::Simple("i64".to_string()), quote! { i64 }),
            (IdlType::Simple("u128".to_string()), quote! { u128 }),
            (IdlType::Simple("i128".to_string()), quote! { i128 }),
            (IdlType::Simple("f32".to_string()), quote! { f32 }),
            (IdlType::Simple("f64".to_string()), quote! { f64 }),
            (IdlType::Simple("string".to_string()), quote! { String }),
            (IdlType::Simple("publicKey".to_string()), quote! { Pubkey }),
            (IdlType::Simple("pubkey".to_string()), quote! { Pubkey }),
            (IdlType::Simple("Pubkey".to_string()), quote! { Pubkey }),
            (IdlType::Simple("bytes".to_string()), quote! { Vec<u8> }),
        ];

        for (input, expected) in test_cases {
            let result = map_idl_type(&input);
            assert_eq!(
                result.to_string(),
                expected.to_string(),
                "Failed for input: {:?}",
                input
            );
        }
    }

    #[test]
    fn test_map_idl_type_custom() {
        let custom_type = IdlType::Simple("MyCustomType".to_string());
        let result = map_idl_type(&custom_type);
        assert_eq!(result.to_string(), quote! { MyCustomType }.to_string());
    }

    #[test]
    fn test_map_idl_type_vec() {
        let vec_type = IdlType::Vec {
            vec: Box::new(IdlType::Simple("u64".to_string())),
        };
        let result = map_idl_type(&vec_type);
        assert_eq!(result.to_string(), quote! { Vec<u64> }.to_string());
    }

    #[test]
    fn test_map_idl_type_nested_vec() {
        let nested_vec = IdlType::Vec {
            vec: Box::new(IdlType::Vec {
                vec: Box::new(IdlType::Simple("u8".to_string())),
            }),
        };
        let result = map_idl_type(&nested_vec);
        let result_str = result.to_string();
        // Token streams may have different whitespace
        assert!(
            result_str.contains("Vec") && result_str.contains("u8"),
            "Result should contain nested Vec type: {}",
            result_str
        );
    }

    #[test]
    fn test_map_idl_type_option() {
        let option_type = IdlType::Option {
            option: Box::new(IdlType::Simple("u64".to_string())),
        };
        let result = map_idl_type(&option_type);
        assert_eq!(result.to_string(), quote! { Option<u64> }.to_string());
    }

    #[test]
    fn test_map_idl_type_option_custom() {
        let option_type = IdlType::Option {
            option: Box::new(IdlType::Simple("MyType".to_string())),
        };
        let result = map_idl_type(&option_type);
        assert_eq!(result.to_string(), quote! { Option<MyType> }.to_string());
    }

    #[test]
    fn test_map_idl_type_array() {
        let array_type = IdlType::Array {
            array: ArrayType::Tuple((Box::new(IdlType::Simple("u8".to_string())), 32)),
        };
        let result = map_idl_type(&array_type);
        let result_str = result.to_string();
        // The array size might have usize suffix
        assert!(
            result_str.contains("[u8") && result_str.contains("32"),
            "Result should contain array type: {}",
            result_str
        );
    }

    #[test]
    fn test_map_idl_type_defined_string() {
        let defined_type = IdlType::Defined {
            defined: DefinedTypeOrString::String("MyStruct".to_string()),
        };
        let result = map_idl_type(&defined_type);
        assert_eq!(result.to_string(), quote! { MyStruct }.to_string());
    }

    #[test]
    fn test_map_idl_type_defined_nested() {
        let defined_type = IdlType::Defined {
            defined: DefinedTypeOrString::Nested(DefinedType {
                name: "MyStruct".to_string(),
            }),
        };
        let result = map_idl_type(&defined_type);
        assert_eq!(result.to_string(), quote! { MyStruct }.to_string());
    }

    #[test]
    fn test_generate_docs_empty() {
        let result = generate_docs(None);
        assert_eq!(result.to_string(), "");
    }

    #[test]
    fn test_generate_docs_single_line() {
        let docs = vec!["This is a single line doc".to_string()];
        let result = generate_docs(Some(&docs));
        assert!(result.to_string().contains("This is a single line doc"));
    }

    #[test]
    fn test_generate_docs_multiple_lines() {
        let docs = vec![
            "First line".to_string(),
            "Second line".to_string(),
            "Third line".to_string(),
        ];
        let result = generate_docs(Some(&docs));
        let result_str = result.to_string();
        assert!(result_str.contains("First line"));
        assert!(result_str.contains("Second line"));
        assert!(result_str.contains("Third line"));
    }

    #[test]
    fn test_generate_docs_with_empty_lines() {
        let docs = vec![
            "First line".to_string(),
            "".to_string(),
            "Third line".to_string(),
        ];
        let result = generate_docs(Some(&docs));
        // Empty lines should be filtered out
        let result_str = result.to_string();
        assert!(result_str.contains("First line"));
        assert!(result_str.contains("Third line"));
    }

    // ============================================================================
    // Type Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_type_def_simple_struct() {
        let type_def = TypeDef {
            name: "MyStruct".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![
                    Field {
                        name: "field1".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    },
                    Field {
                        name: "field2".to_string(),
                        ty: IdlType::Simple("string".to_string()),
                        docs: None,
                    },
                ]),
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub struct MyStruct"));
        assert!(result_str.contains("pub field1 : u64"));
        assert!(result_str.contains("pub field2 : String"));
        assert!(result_str.contains("BorshSerialize"));
        assert!(result_str.contains("BorshDeserialize"));
    }

    #[test]
    fn test_generate_type_def_struct_with_docs() {
        let type_def = TypeDef {
            name: "MyStruct".to_string(),
            docs: Some(vec!["This is a documented struct".to_string()]),
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![Field {
                    name: "field1".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    docs: Some(vec!["Field documentation".to_string()]),
                }]),
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("This is a documented struct"));
        assert!(result_str.contains("Field documentation"));
    }

    #[test]
    fn test_generate_type_def_bytemuck_struct() {
        let type_def = TypeDef {
            name: "MyBytemuckStruct".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![
                    Field {
                        name: "field1".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    },
                    Field {
                        name: "field2".to_string(),
                        ty: IdlType::Simple("u32".to_string()),
                        docs: None,
                    },
                ]),
            },
            serialization: Some("bytemuck".to_string()),
            repr: Some(Repr {
                kind: "C".to_string(),
                packed: None,
            }),
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub struct MyBytemuckStruct"));
        assert!(result_str.contains("repr") && result_str.contains("C"));
        assert!(result_str.contains("unsafe impl bytemuck :: Pod"));
        assert!(result_str.contains("unsafe impl bytemuck :: Zeroable"));
        assert!(!result_str.contains("BorshSerialize"));
    }

    #[test]
    fn test_generate_type_def_bytemuck_packed_struct() {
        let type_def = TypeDef {
            name: "PackedStruct".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![Field {
                    name: "field1".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    docs: None,
                }]),
            },
            serialization: Some("bytemuckunsafe".to_string()),
            repr: Some(Repr {
                kind: "C".to_string(),
                packed: Some(true),
            }),
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(
            result_str.contains("repr")
                && result_str.contains("C")
                && result_str.contains("packed")
        );
    }

    #[test]
    fn test_generate_type_def_tuple_struct() {
        let type_def = TypeDef {
            name: "OptionBool".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Tuple(vec![IdlType::Simple("bool".to_string())]),
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub struct OptionBool"));
        assert!(result_str.contains("pub bool"));
        assert!(result_str.contains("BorshSerialize"));
        assert!(result_str.contains("BorshDeserialize"));
    }

    #[test]
    fn test_generate_type_def_simple_enum() {
        let type_def = TypeDef {
            name: "MyEnum".to_string(),
            docs: None,
            ty: TypeDefType::Enum {
                variants: vec![
                    EnumVariant {
                        name: "Variant1".to_string(),
                        fields: None,
                    },
                    EnumVariant {
                        name: "Variant2".to_string(),
                        fields: None,
                    },
                ],
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub enum MyEnum"));
        assert!(result_str.contains("Variant1"));
        assert!(result_str.contains("Variant2"));
        assert!(result_str.contains("BorshSerialize"));
    }

    #[test]
    fn test_generate_type_def_enum_with_named_fields() {
        let type_def = TypeDef {
            name: "MyEnum".to_string(),
            docs: None,
            ty: TypeDefType::Enum {
                variants: vec![EnumVariant {
                    name: "VariantWithFields".to_string(),
                    fields: Some(EnumFields::Named(vec![
                        Field {
                            name: "field1".to_string(),
                            ty: IdlType::Simple("u64".to_string()),
                            docs: None,
                        },
                        Field {
                            name: "field2".to_string(),
                            ty: IdlType::Simple("string".to_string()),
                            docs: None,
                        },
                    ])),
                }],
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("VariantWithFields"));
        assert!(result_str.contains("field1 : u64"));
        assert!(result_str.contains("field2 : String"));
    }

    #[test]
    fn test_generate_type_def_enum_with_tuple_fields() {
        let type_def = TypeDef {
            name: "MyEnum".to_string(),
            docs: None,
            ty: TypeDefType::Enum {
                variants: vec![EnumVariant {
                    name: "TupleVariant".to_string(),
                    fields: Some(EnumFields::Tuple(vec![
                        IdlType::Simple("u64".to_string()),
                        IdlType::Simple("string".to_string()),
                    ])),
                }],
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("TupleVariant"));
        assert!(result_str.contains("u64"));
        assert!(result_str.contains("String"));
    }

    #[test]
    fn test_generate_type_def_snake_case_fields() {
        let type_def = TypeDef {
            name: "MyStruct".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![Field {
                    name: "CamelCaseField".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    docs: None,
                }]),
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("camel_case_field"));
    }

    // ============================================================================
    // Error Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_errors_simple() {
        let errors = vec![
            Error {
                code: 6000,
                name: "InvalidAmount".to_string(),
                msg: Some("The amount is invalid".to_string()),
            },
            Error {
                code: 6001,
                name: "Unauthorized".to_string(),
                msg: Some("User is not authorized".to_string()),
            },
        ];

        let result = generate_errors(&errors).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub enum ErrorCode"));
        assert!(result_str.contains("InvalidAmount"));
        assert!(result_str.contains("Unauthorized"));
        assert!(result_str.contains("The amount is invalid"));
        assert!(result_str.contains("User is not authorized"));
        assert!(result_str.contains("= 6000"));
        assert!(result_str.contains("= 6001"));
        assert!(result_str.contains("thiserror :: Error"));
        assert!(result_str.contains("impl From < ErrorCode > for ProgramError"));
    }

    #[test]
    fn test_generate_errors_no_message() {
        let errors = vec![Error {
            code: 6000,
            name: "ErrorWithoutMessage".to_string(),
            msg: None,
        }];

        let result = generate_errors(&errors).unwrap();
        let result_str = result.to_string();

        // Should use name as message when msg is None
        assert!(result_str.contains("ErrorWithoutMessage"));
        assert!(result_str.contains("= 6000"));
    }

    #[test]
    fn test_generate_errors_empty() {
        let errors = vec![];
        let result = generate_errors(&errors).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub enum ErrorCode"));
    }

    // ============================================================================
    // Event Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_event_with_fields() {
        let event = Event {
            name: "TransferEvent".to_string(),
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            fields: Some(vec![
                EventField {
                    name: "from".to_string(),
                    ty: IdlType::Simple("publicKey".to_string()),
                    index: false,
                },
                EventField {
                    name: "to".to_string(),
                    ty: IdlType::Simple("publicKey".to_string()),
                    index: false,
                },
                EventField {
                    name: "amount".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    index: false,
                },
            ]),
        };

        let result = generate_event(&event, &None).unwrap();
        let result_str = result.to_string();

        // Check for module-level discriminator constant
        assert!(result_str.contains("TRANSFER_EVENT_EVENT_DISCM"));
        assert!(result_str.contains("[1u8 , 2u8 , 3u8 , 4u8 , 5u8 , 6u8 , 7u8 , 8u8]"));

        // Check for data struct
        assert!(result_str.contains("pub struct TransferEvent"));
        assert!(result_str.contains("pub from : Pubkey"));
        assert!(result_str.contains("pub to : Pubkey"));
        assert!(result_str.contains("pub amount : u64"));

        // Check for wrapper struct
        assert!(result_str.contains("pub struct TransferEventEvent"));
        assert!(result_str.contains("pub fn deserialize"));

        // Check for custom serde serialization of Pubkey fields
        assert!(result_str.contains("serialize_pubkey_as_string"));
    }

    #[test]
    fn test_generate_event_without_discriminator() {
        let event = Event {
            name: "SimpleEvent".to_string(),
            discriminator: None,
            fields: Some(vec![EventField {
                name: "value".to_string(),
                ty: IdlType::Simple("u64".to_string()),
                index: false,
            }]),
        };

        let result = generate_event(&event, &None).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub struct SimpleEvent"));
        assert!(!result_str.contains("DISCRIMINATOR"));
    }

    #[test]
    fn test_generate_event_without_fields() {
        let event = Event {
            name: "EmptyEvent".to_string(),
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            fields: None,
        };

        let result = generate_event(&event, &None).unwrap();
        let result_str = result.to_string();

        // Events without fields should not generate anything
        assert_eq!(result_str, "");
    }

    #[test]
    fn test_generate_event_from_type_definition() {
        // New IDL format: event has only name and discriminator,
        // fields are in a matching type definition
        let event = Event {
            name: "AdminSetCreatorEvent".to_string(),
            discriminator: Some(vec![64, 69, 192, 104, 29, 30, 25, 107]),
            fields: None, // No fields in event itself
        };

        let types = Some(vec![TypeDef {
            name: "AdminSetCreatorEvent".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![
                    Field {
                        name: "timestamp".to_string(),
                        ty: IdlType::Simple("i64".to_string()),
                        docs: None,
                    },
                    Field {
                        name: "admin_set_creator_authority".to_string(),
                        ty: IdlType::Simple("pubkey".to_string()),
                        docs: None,
                    },
                    Field {
                        name: "mint".to_string(),
                        ty: IdlType::Simple("pubkey".to_string()),
                        docs: None,
                    },
                ]),
            },
            serialization: None,
            repr: None,
        }]);

        let result = generate_event(&event, &types).unwrap();
        let result_str = result.to_string();

        // Check for module-level discriminator constant
        assert!(result_str.contains("ADMIN_SET_CREATOR_EVENT_EVENT_DISCM"));
        assert!(result_str.contains("[64u8 , 69u8 , 192u8 , 104u8 , 29u8 , 30u8 , 25u8 , 107u8]"));

        // Check for data struct
        assert!(result_str.contains("pub struct AdminSetCreatorEvent"));
        assert!(result_str.contains("pub timestamp : i64"));
        assert!(result_str.contains("pub admin_set_creator_authority : Pubkey"));
        assert!(result_str.contains("pub mint : Pubkey"));

        // Check for wrapper struct
        assert!(result_str.contains("pub struct AdminSetCreatorEventEvent"));
        assert!(result_str.contains("pub fn deserialize"));

        // Check for custom serde serialization of Pubkey fields
        assert!(result_str.contains("serialize_pubkey_as_string"));
    }

    // ============================================================================
    // Instruction Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_instructions_simple() {
        let instructions = vec![
            Instruction {
                name: "initialize".to_string(),
                docs: None,
                discriminator: Some(vec![175, 175, 109, 31, 13, 152, 155, 237]),
                accounts: vec![],
                args: vec![],
            },
            Instruction {
                name: "transfer".to_string(),
                docs: None,
                discriminator: Some(vec![163, 52, 200, 231, 140, 3, 69, 186]),
                accounts: vec![],
                args: vec![Arg {
                    name: "amount".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                }],
            },
        ];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub enum Instruction"));
        assert!(result_str.contains("Initialize"));
        assert!(result_str.contains("Transfer"));
        assert!(result_str.contains("TransferIxArgs"));
        assert!(result_str.contains("TransferIxData"));
        assert!(result_str.contains("INITIALIZE_IX_DISCM"));
        assert!(result_str.contains("TRANSFER_IX_DISCM"));
        assert!(result_str.contains("pub amount : u64"));
        assert!(result_str.contains("serialize"));
        assert!(result_str.contains("try_from_slice"));
    }

    #[test]
    fn test_generate_instructions_with_accounts() {
        let instructions = vec![Instruction {
            name: "swap".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![
                AccountArg {
                    name: "user".to_string(),
                    docs: Some(vec!["The user account".to_string()]),
                    signer: true,
                    writable: true,
                    pda: None,
                    address: None,
                    optional: None,
                },
                AccountArg {
                    name: "pool".to_string(),
                    docs: None,
                    signer: false,
                    writable: true,
                    pda: None,
                    address: None,
                    optional: None,
                },
            ],
            args: vec![],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("SwapKeys"));
        assert!(result_str.contains("pub user : Pubkey"));
        assert!(result_str.contains("pub pool : Pubkey"));
        assert!(result_str.contains("The user account"));
    }

    #[test]
    fn test_generate_instructions_multiple_args() {
        let instructions = vec![Instruction {
            name: "complex_instruction".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![],
            args: vec![
                Arg {
                    name: "amount".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                },
                Arg {
                    name: "recipient".to_string(),
                    ty: IdlType::Simple("publicKey".to_string()),
                },
                Arg {
                    name: "memo".to_string(),
                    ty: IdlType::Option {
                        option: Box::new(IdlType::Simple("string".to_string())),
                    },
                },
            ],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("ComplexInstructionIxArgs"));
        assert!(result_str.contains("ComplexInstructionIxData"));
        assert!(result_str.contains("pub amount : u64"));
        assert!(result_str.contains("pub recipient : Pubkey"));
        assert!(result_str.contains("pub memo : Option < String >"));
    }

    #[test]
    fn test_generate_instructions_without_discriminator() {
        let instructions = vec![
            Instruction {
                name: "first".to_string(),
                docs: None,
                discriminator: None,
                accounts: vec![],
                args: vec![],
            },
            Instruction {
                name: "second".to_string(),
                docs: None,
                discriminator: None,
                accounts: vec![],
                args: vec![],
            },
        ];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Should generate with index-based discriminators
        assert!(result_str.contains("First"));
        assert!(result_str.contains("Second"));
    }

    // ============================================================================
    // Account Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_account_with_type() {
        let account = Account {
            name: "UserAccount".to_string(),
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            docs: Some(vec!["User account structure".to_string()]),
            ty: Some(TypeDefType::Struct {
                fields: StructFields::Named(vec![Field {
                    name: "balance".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    docs: None,
                }]),
            }),
        };

        let result = generate_account(&account).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("pub struct UserAccount"));
        assert!(result_str.contains("pub balance : u64"));
        assert!(result_str.contains("DISCRIMINATOR"));
        assert!(result_str.contains("try_from_slice_with_discriminator"));
        assert!(result_str.contains("serialize_with_discriminator"));
    }

    #[test]
    fn test_generate_account_without_type() {
        let account = Account {
            name: "ReferenceAccount".to_string(),
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            docs: None,
            ty: None,
        };

        let result = generate_account(&account).unwrap();
        let result_str = result.to_string();

        // Should return empty TokenStream for reference accounts
        assert_eq!(result_str, "");
    }

    // ============================================================================
    // Integration Tests - Full Code Generation
    // ============================================================================

    #[test]
    fn test_generate_minimal_idl() {
        let idl = Idl {
            address: Some("11111111111111111111111111111111".to_string()),
            version: Some("0.1.0".to_string()),
            name: Some("minimal_program".to_string()),
            metadata: None,
            // Include at least one instruction to avoid empty match arms
            instructions: vec![Instruction {
                name: "noop".to_string(),
                docs: None,
                discriminator: Some(vec![0, 0, 0, 0, 0, 0, 0, 0]),
                accounts: vec![],
                args: vec![],
            }],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate(&idl, "minimal_program");
        assert!(
            result.is_ok(),
            "Generation should succeed: {:?}",
            result.err()
        );
        let code = result.unwrap();
        assert!(code.lib.contains("pub mod"));
        assert!(
            code.instructions.contains("use borsh")
                || code.instructions.contains("pub enum Instruction")
        );
    }

    #[test]
    fn test_generate_idl_with_types() {
        let idl = Idl {
            address: None,
            version: None,
            name: Some("test_program".to_string()),
            metadata: None,
            // Include at least one instruction to avoid empty match arms
            instructions: vec![Instruction {
                name: "noop".to_string(),
                docs: None,
                discriminator: Some(vec![0, 0, 0, 0, 0, 0, 0, 0]),
                accounts: vec![],
                args: vec![],
            }],
            accounts: None,
            types: Some(vec![TypeDef {
                name: "TestStruct".to_string(),
                docs: None,
                ty: TypeDefType::Struct {
                    fields: StructFields::Named(vec![Field {
                        name: "value".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    }]),
                },
                serialization: None,
                repr: None,
            }]),
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate(&idl, "test_program");
        assert!(
            result.is_ok(),
            "Generation should succeed: {:?}",
            result.err()
        );
        let code = result.unwrap();
        assert!(code.types.contains("pub struct TestStruct"));
        assert!(code.types.contains("pub value: u64"));
    }

    #[test]
    fn test_generate_idl_with_discriminators() {
        let idl = Idl {
            address: None,
            version: None,
            name: Some("test_program".to_string()),
            metadata: None,
            // Include at least one instruction to avoid empty match arms
            instructions: vec![Instruction {
                name: "noop".to_string(),
                docs: None,
                discriminator: Some(vec![0, 0, 0, 0, 0, 0, 0, 0]),
                accounts: vec![],
                args: vec![],
            }],
            accounts: Some(vec![Account {
                name: "TestAccount".to_string(),
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                docs: None,
                ty: None,
            }]),
            types: Some(vec![TypeDef {
                name: "TestAccount".to_string(),
                docs: None,
                ty: TypeDefType::Struct {
                    fields: StructFields::Named(vec![Field {
                        name: "data".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    }]),
                },
                serialization: None,
                repr: None,
            }]),
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate(&idl, "test_program");
        assert!(
            result.is_ok(),
            "Generation should succeed: {:?}",
            result.err()
        );
        let code = result.unwrap();
        assert!(code.accounts.contains("DISCRIMINATOR"));
        assert!(code.accounts.contains("try_from_slice_with_discriminator"));
    }

    #[test]
    fn test_generate_idl_with_bytemuck_serialization() {
        let idl = Idl {
            address: None,
            version: None,
            name: Some("test_program".to_string()),
            metadata: None,
            // Include at least one instruction to avoid empty match arms
            instructions: vec![Instruction {
                name: "noop".to_string(),
                docs: None,
                discriminator: Some(vec![0, 0, 0, 0, 0, 0, 0, 0]),
                accounts: vec![],
                args: vec![],
            }],
            accounts: Some(vec![Account {
                name: "BytemuckAccount".to_string(),
                discriminator: Some(vec![10, 20, 30, 40, 50, 60, 70, 80]),
                docs: None,
                ty: None,
            }]),
            types: Some(vec![TypeDef {
                name: "BytemuckAccount".to_string(),
                docs: None,
                ty: TypeDefType::Struct {
                    fields: StructFields::Named(vec![Field {
                        name: "value".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    }]),
                },
                serialization: Some("bytemuck".to_string()),
                repr: Some(Repr {
                    kind: "C".to_string(),
                    packed: None,
                }),
            }]),
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate(&idl, "test_program");
        assert!(
            result.is_ok(),
            "Generation should succeed: {:?}",
            result.err()
        );
        let code = result.unwrap();
        assert!(code.accounts.contains("bytemuck::try_from_bytes"));
        assert!(code.accounts.contains("bytemuck::bytes_of"));
    }

    #[test]
    fn test_generate_complex_idl() {
        let idl = Idl {
            address: Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()),
            version: Some("1.0.0".to_string()),
            name: Some("token_program".to_string()),
            metadata: None,
            instructions: vec![Instruction {
                name: "transfer".to_string(),
                docs: Some(vec![
                    "Transfers tokens from one account to another".to_string()
                ]),
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                accounts: vec![
                    AccountArg {
                        name: "source".to_string(),
                        docs: None,
                        signer: true,
                        writable: true,
                        pda: None,
                        address: None,
                        optional: None,
                    },
                    AccountArg {
                        name: "destination".to_string(),
                        docs: None,
                        signer: false,
                        writable: true,
                        pda: None,
                        address: None,
                        optional: None,
                    },
                ],
                args: vec![Arg {
                    name: "amount".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                }],
            }],
            accounts: None,
            types: Some(vec![TypeDef {
                name: "TokenAccount".to_string(),
                docs: Some(vec!["Token account data".to_string()]),
                ty: TypeDefType::Struct {
                    fields: StructFields::Named(vec![
                        Field {
                            name: "mint".to_string(),
                            ty: IdlType::Simple("publicKey".to_string()),
                            docs: None,
                        },
                        Field {
                            name: "owner".to_string(),
                            ty: IdlType::Simple("publicKey".to_string()),
                            docs: None,
                        },
                        Field {
                            name: "amount".to_string(),
                            ty: IdlType::Simple("u64".to_string()),
                            docs: None,
                        },
                    ]),
                },
                serialization: None,
                repr: None,
            }]),
            errors: Some(vec![Error {
                code: 6000,
                name: "InsufficientFunds".to_string(),
                msg: Some("Insufficient funds for transfer".to_string()),
            }]),
            events: Some(vec![Event {
                name: "TransferEvent".to_string(),
                discriminator: Some(vec![255, 254, 253, 252, 251, 250, 249, 248]),
                fields: Some(vec![
                    EventField {
                        name: "from".to_string(),
                        ty: IdlType::Simple("publicKey".to_string()),
                        index: false,
                    },
                    EventField {
                        name: "to".to_string(),
                        ty: IdlType::Simple("publicKey".to_string()),
                        index: false,
                    },
                    EventField {
                        name: "amount".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        index: false,
                    },
                ]),
            }]),
            constants: None,
        };

        let result = generate(&idl, "token_program");
        assert!(result.is_ok());
        let code = result.unwrap();

        // Check all major components are present in their respective modules
        assert!(code.types.contains("pub struct TokenAccount"));
        assert!(code.instructions.contains("pub enum Instruction"));
        assert!(code.instructions.contains("Transfer"));
        assert!(code.instructions.contains("TransferIxArgs"));
        assert!(code.instructions.contains("TransferIxData"));
        assert!(code.instructions.contains("pub amount: u64"));
        assert!(code.errors.contains("pub enum ErrorCode"));
        assert!(code.errors.contains("InsufficientFunds"));
        assert!(code.events.contains("pub struct TransferEvent"));
    }

    // ============================================================================
    // Program ID Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_lib_with_program_id() {
        let idl = Idl {
            address: Some("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string()),
            version: Some("0.1.0".to_string()),
            name: Some("test_program".to_string()),
            metadata: None,
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let lib_code = generate_lib_module(&idl);
        assert!(lib_code.contains("solana_program::declare_id!"));
        assert!(lib_code.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));
    }

    #[test]
    fn test_generate_lib_without_program_id() {
        let idl = Idl {
            address: None,
            version: Some("0.1.0".to_string()),
            name: Some("test_program".to_string()),
            metadata: None,
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let lib_code = generate_lib_module(&idl);
        assert!(lib_code.contains("Program ID not specified"));
        assert!(lib_code.contains("YourProgramIdHere"));
    }

    #[test]
    fn test_generated_code_includes_program_id() {
        let idl = Idl {
            address: Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()),
            version: Some("1.0.0".to_string()),
            name: Some("token_program".to_string()),
            metadata: None,
            instructions: vec![Instruction {
                name: "noop".to_string(),
                docs: None,
                discriminator: Some(vec![0, 0, 0, 0, 0, 0, 0, 0]),
                accounts: vec![],
                args: vec![],
            }],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate(&idl, "token_program");
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code
            .lib
            .contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"));
    }

    // ============================================================================
    // Instruction IxData Pattern Tests
    // ============================================================================

    #[test]
    fn test_generate_ixdata_wrapper_no_args() {
        let instructions = vec![Instruction {
            name: "initialize".to_string(),
            docs: None,
            discriminator: Some(vec![175, 175, 109, 31, 13, 152, 155, 237]),
            accounts: vec![],
            args: vec![],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check for discriminator constant
        assert!(result_str.contains("INITIALIZE_IX_DISCM"));
        assert!(result_str.contains("175"));
        assert!(result_str.contains("237"));

        // Check for IxData struct
        assert!(result_str.contains("InitializeIxData"));
        assert!(result_str.contains("deserialize"));
        assert!(result_str.contains("serialize"));
        assert!(result_str.contains("try_to_vec"));
    }

    #[test]
    fn test_generate_ixdata_wrapper_with_args() {
        let instructions = vec![Instruction {
            name: "transfer".to_string(),
            docs: None,
            discriminator: Some(vec![163, 52, 200, 231, 140, 3, 69, 186]),
            accounts: vec![],
            args: vec![Arg {
                name: "amount".to_string(),
                ty: IdlType::Simple("u64".to_string()),
            }],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check for discriminator constant
        assert!(result_str.contains("TRANSFER_IX_DISCM"));

        // Check for IxData wrapper struct
        assert!(result_str.contains("TransferIxData"));
        assert!(result_str.contains("TransferIxArgs"));

        // Check for From implementation
        assert!(result_str.contains("From"));

        // Check for IxArgs struct
        assert!(result_str.contains("pub amount"));
        assert!(result_str.contains("u64"));
    }

    #[test]
    fn test_ixdata_discriminator_in_serialization() {
        let instructions = vec![Instruction {
            name: "buy".to_string(),
            docs: None,
            discriminator: Some(vec![102, 6, 61, 18, 1, 218, 235, 234]),
            accounts: vec![],
            args: vec![Arg {
                name: "amount".to_string(),
                ty: IdlType::Simple("u64".to_string()),
            }],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check that serialize method uses the discriminator constant
        assert!(result_str.contains("BUY_IX_DISCM"));
        assert!(result_str.contains("write_all"));
    }

    // ============================================================================
    // AccountMeta Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_keys_struct() {
        let instructions = vec![Instruction {
            name: "transfer".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![
                AccountArg {
                    name: "from".to_string(),
                    docs: None,
                    signer: true,
                    writable: true,
                    pda: None,
                    address: None,
                    optional: None,
                },
                AccountArg {
                    name: "to".to_string(),
                    docs: None,
                    signer: false,
                    writable: true,
                    pda: None,
                    address: None,
                    optional: None,
                },
            ],
            args: vec![],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check for Keys struct
        assert!(result_str.contains("TransferKeys"));
        assert!(result_str.contains("pub from : Pubkey"));
        assert!(result_str.contains("pub to : Pubkey"));

        // Check for accounts length constant
        assert!(result_str.contains("TRANSFER_IX_ACCOUNTS_LEN"));
        assert!(result_str.contains(": usize = 2"));
    }

    #[test]
    fn test_generate_account_meta_conversion() {
        let instructions = vec![Instruction {
            name: "initialize".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![
                AccountArg {
                    name: "admin".to_string(),
                    docs: None,
                    signer: true,
                    writable: false,
                    pda: None,
                    address: None,
                    optional: None,
                },
                AccountArg {
                    name: "config".to_string(),
                    docs: None,
                    signer: false,
                    writable: true,
                    pda: None,
                    address: None,
                    optional: None,
                },
                AccountArg {
                    name: "system_program".to_string(),
                    docs: None,
                    signer: false,
                    writable: false,
                    pda: None,
                    address: None,
                    optional: None,
                },
            ],
            args: vec![],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check for From implementation
        assert!(result_str.contains("impl From < InitializeKeys > for [AccountMeta"));

        // Check that is_signer and is_writable are set correctly
        assert!(result_str.contains("is_signer : true"));
        assert!(result_str.contains("is_signer : false"));
        assert!(result_str.contains("is_writable : true"));
        assert!(result_str.contains("is_writable : false"));
    }

    #[test]
    fn test_account_meta_flags_correctness() {
        let instructions = vec![Instruction {
            name: "test".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![
                AccountArg {
                    name: "signer_writable".to_string(),
                    docs: None,
                    signer: true,
                    writable: true,
                    pda: None,
                    address: None,
                    optional: None,
                },
                AccountArg {
                    name: "signer_readonly".to_string(),
                    docs: None,
                    signer: true,
                    writable: false,
                    pda: None,
                    address: None,
                    optional: None,
                },
                AccountArg {
                    name: "nonsigner_writable".to_string(),
                    docs: None,
                    signer: false,
                    writable: true,
                    pda: None,
                    address: None,
                    optional: None,
                },
                AccountArg {
                    name: "nonsigner_readonly".to_string(),
                    docs: None,
                    signer: false,
                    writable: false,
                    pda: None,
                    address: None,
                    optional: None,
                },
            ],
            args: vec![],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Verify all four combinations are represented
        assert!(result_str.contains("signer_writable"));
        assert!(result_str.contains("signer_readonly"));
        assert!(result_str.contains("nonsigner_writable"));
        assert!(result_str.contains("nonsigner_readonly"));
    }

    // ============================================================================
    // Instruction Builder Function Tests
    // ============================================================================

    #[test]
    fn test_generate_instruction_builder_no_args() {
        let instructions = vec![Instruction {
            name: "initialize".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![AccountArg {
                name: "config".to_string(),
                docs: None,
                signer: false,
                writable: true,
                pda: None,
                address: None,
                optional: None,
            }],
            args: vec![],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check for builder functions
        assert!(result_str.contains("pub fn initialize_ix"));
        assert!(result_str.contains("pub fn initialize_ix_with_program_id"));
        assert!(result_str.contains("keys : InitializeKeys"));
        assert!(result_str.contains("crate :: ID"));
    }

    #[test]
    fn test_generate_instruction_builder_with_args() {
        let instructions = vec![Instruction {
            name: "transfer".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![AccountArg {
                name: "from".to_string(),
                docs: None,
                signer: true,
                writable: true,
                pda: None,
                address: None,
                optional: None,
            }],
            args: vec![Arg {
                name: "amount".to_string(),
                ty: IdlType::Simple("u64".to_string()),
            }],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check for builder functions with args
        assert!(result_str.contains("pub fn transfer_ix"));
        assert!(result_str.contains("pub fn transfer_ix_with_program_id"));
        assert!(result_str.contains("keys : TransferKeys"));
        assert!(result_str.contains("args : TransferIxArgs"));
    }

    #[test]
    fn test_instruction_builder_returns_instruction() {
        let instructions = vec![Instruction {
            name: "swap".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![AccountArg {
                name: "user".to_string(),
                docs: None,
                signer: true,
                writable: false,
                pda: None,
                address: None,
                optional: None,
            }],
            args: vec![Arg {
                name: "amount".to_string(),
                ty: IdlType::Simple("u64".to_string()),
            }],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check that builder returns Instruction
        assert!(result_str.contains("-> std :: io :: Result"));
        assert!(result_str.contains("solana_program :: instruction :: Instruction"));
        assert!(result_str.contains("program_id"));
        assert!(result_str.contains("accounts"));
        assert!(result_str.contains("data"));
    }

    // ============================================================================
    // Edge Cases and Error Handling
    // ============================================================================

    #[test]
    fn test_empty_struct() {
        let type_def = TypeDef {
            name: "EmptyStruct".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![]),
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deeply_nested_types() {
        let deeply_nested = IdlType::Vec {
            vec: Box::new(IdlType::Option {
                option: Box::new(IdlType::Vec {
                    vec: Box::new(IdlType::Simple("u64".to_string())),
                }),
            }),
        };

        let result = map_idl_type(&deeply_nested);
        let result_str = result.to_string();
        // Token streams may have different whitespace, just check the structure
        assert!(
            result_str.contains("Vec")
                && result_str.contains("Option")
                && result_str.contains("u64"),
            "Result should contain deeply nested type: {}",
            result_str
        );
    }

    #[test]
    fn test_snake_case_conversion() {
        let type_def = TypeDef {
            name: "TestStruct".to_string(),
            docs: None,
            ty: TypeDefType::Struct {
                fields: StructFields::Named(vec![
                    Field {
                        name: "camelCase".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    },
                    Field {
                        name: "PascalCase".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    },
                    Field {
                        name: "snake_case".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    },
                ]),
            },
            serialization: None,
            repr: None,
        };

        let result = generate_type_def(&type_def).unwrap();
        let result_str = result.to_string();

        assert!(result_str.contains("camel_case"));
        assert!(result_str.contains("pascal_case"));
        assert!(result_str.contains("snake_case"));
    }

    #[test]
    fn test_instruction_deserialization_with_args() {
        let instructions = vec![Instruction {
            name: "test_instruction".to_string(),
            docs: None,
            discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            accounts: vec![],
            args: vec![Arg {
                name: "value".to_string(),
                ty: IdlType::Simple("u64".to_string()),
            }],
        }];

        let result = generate_instructions(&instructions, true).unwrap();
        let result_str = result.to_string();

        // Check that deserialization uses &mut buf
        assert!(result_str.contains("deserialize (& mut buf)"));
    }

    // ============================================================================
    // Event Parsing Helpers Tests
    // ============================================================================

    #[test]
    fn test_generate_event_parsing_helpers_empty() {
        let events = vec![];
        let result = generate_event_parsing_helpers(&events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_generate_event_parsing_helpers_with_events() {
        let events = vec![
            Event {
                name: "CreateEvent".to_string(),
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                fields: Some(vec![EventField {
                    name: "mint".to_string(),
                    ty: IdlType::Simple("pubkey".to_string()),
                    index: false,
                }]),
            },
            Event {
                name: "TradeEvent".to_string(),
                discriminator: Some(vec![9, 10, 11, 12, 13, 14, 15, 16]),
                fields: Some(vec![EventField {
                    name: "amount".to_string(),
                    ty: IdlType::Simple("u64".to_string()),
                    index: false,
                }]),
            },
        ];

        let result = generate_event_parsing_helpers(&events).unwrap();
        let result_str = result.to_string();

        // Check for ParsedEvent enum
        assert!(result_str.contains("enum ParsedEvent"));
        assert!(result_str.contains("CreateEvent") && result_str.contains("CreateEventEvent"));
        assert!(result_str.contains("TradeEvent") && result_str.contains("TradeEventEvent"));

        // Check for EventParseError
        assert!(result_str.contains("enum EventParseError"));
        assert!(result_str.contains("DataTooShort"));
        assert!(result_str.contains("UnknownDiscriminator"));
        assert!(result_str.contains("DeserializationError"));

        // Check for parse_event function
        assert!(result_str.contains("fn parse_event"));
        assert!(result_str.contains("parse_events_from_data"));

        // Check for discriminator matching
        assert!(result_str.contains("CREATE_EVENT_EVENT_DISCM"));
        assert!(result_str.contains("TRADE_EVENT_EVENT_DISCM"));
    }

    #[test]
    fn test_generate_event_parsing_helpers_no_discriminators() {
        let events = vec![Event {
            name: "NoDiscEvent".to_string(),
            discriminator: None,
            fields: Some(vec![EventField {
                name: "data".to_string(),
                ty: IdlType::Simple("u64".to_string()),
                index: false,
            }]),
        }];

        let result = generate_event_parsing_helpers(&events).unwrap();
        assert!(
            result.is_empty(),
            "Events without discriminators should not generate helpers"
        );
    }

    // ============================================================================
    // Account Validation Helpers Tests
    // ============================================================================

    #[test]
    fn test_generate_account_validation_helpers_no_program_id() {
        let idl = Idl {
            address: None,
            version: None,
            name: None,
            metadata: None,
            instructions: vec![],
            accounts: Some(vec![Account {
                name: "TestAccount".to_string(),
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                docs: None,
                ty: Some(TypeDefType::Struct {
                    fields: StructFields::Named(vec![]),
                }),
            }]),
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate_account_validation_helpers(&idl).unwrap();
        assert!(
            result.is_empty(),
            "Should not generate helpers without program ID"
        );
    }

    #[test]
    fn test_generate_account_validation_helpers_with_program_id() {
        let idl = Idl {
            address: Some("11111111111111111111111111111111".to_string()),
            version: None,
            name: None,
            metadata: None,
            instructions: vec![],
            accounts: Some(vec![Account {
                name: "TestAccount".to_string(),
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                docs: None,
                ty: Some(TypeDefType::Struct {
                    fields: StructFields::Named(vec![Field {
                        name: "value".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    }]),
                }),
            }]),
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate_account_validation_helpers(&idl).unwrap();
        let result_str = result.to_string();

        // Check for ValidationError enum
        assert!(result_str.contains("enum ValidationError"));
        assert!(result_str.contains("InvalidOwner"));
        assert!(result_str.contains("DataTooShort"));
        assert!(result_str.contains("InvalidDiscriminator"));
        assert!(result_str.contains("DeserializationError"));

        // Check for validation methods
        assert!(result_str.contains("impl TestAccount"));
        assert!(result_str.contains("fn validate_account_info"));
        assert!(result_str.contains("fn try_from_account_info"));
        assert!(result_str.contains("ID") || result_str.contains("crate :: ID"));
    }

    #[test]
    fn test_generate_account_validation_helpers_new_format() {
        let idl = Idl {
            address: Some("11111111111111111111111111111111".to_string()),
            version: None,
            name: None,
            metadata: None,
            instructions: vec![],
            accounts: Some(vec![Account {
                name: "PoolState".to_string(),
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                docs: None,
                ty: None, // New format - references type
            }]),
            types: Some(vec![TypeDef {
                name: "PoolState".to_string(),
                docs: None,
                ty: TypeDefType::Struct {
                    fields: StructFields::Named(vec![Field {
                        name: "amount".to_string(),
                        ty: IdlType::Simple("u64".to_string()),
                        docs: None,
                    }]),
                },
                serialization: None,
                repr: None,
            }]),
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate_account_validation_helpers(&idl).unwrap();
        let result_str = result.to_string();

        // Should generate validation for PoolState type
        assert!(result_str.contains("impl PoolState"));
        assert!(result_str.contains("fn validate_account_info"));
    }

    #[test]
    fn test_generate_account_validation_helpers_empty_accounts() {
        let idl = Idl {
            address: Some("11111111111111111111111111111111".to_string()),
            version: None,
            name: None,
            metadata: None,
            instructions: vec![],
            accounts: Some(vec![]),
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let result = generate_account_validation_helpers(&idl).unwrap();
        assert!(
            result.is_empty(),
            "Should not generate helpers for empty accounts"
        );
    }
}
