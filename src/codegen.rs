use crate::idl::{ArrayType, *};
use anyhow::Result;
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_str;

pub fn generate(idl: &Idl, module_name: &str) -> Result<String> {
    let mut tokens = TokenStream::new();

    // Generate module header
    let _module_ident = format_ident!("{}", module_name);

    // Generate account discriminators
    // Note: In new format IDLs, accounts reference types that are generated separately
    // We'll add discriminator impl blocks for accounts that match type names
    let mut account_discriminators = std::collections::HashMap::new();
    if let Some(accounts) = &idl.accounts {
        for account in accounts {
            if let Some(disc) = &account.discriminator {
                account_discriminators.insert(account.name.clone(), disc.clone());
            }
            // Only generate if account has type definition (old format)
            if account.ty.is_some() {
                tokens.extend(generate_account(account)?);
            }
        }
    }

    // Generate types (including those referenced by accounts)
    if let Some(types) = &idl.types {
        for ty in types {
            let mut type_tokens = generate_type_def(ty)?;
            
            // Add discriminator methods if there's a matching account discriminator
            if let Some(disc) = account_discriminators.get(&ty.name) {
                let name = format_ident!("{}", ty.name);
                let disc_bytes = disc.iter().map(|b| quote! { #b });
                
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
            
            tokens.extend(type_tokens);
        }
    }

    // Generate instruction structs and enums
    tokens.extend(generate_instructions(&idl.instructions)?);

    // Generate errors
    if let Some(errors) = &idl.errors {
        tokens.extend(generate_errors(errors)?);
    }

    // Generate events
    if let Some(events) = &idl.events {
        for event in events {
            tokens.extend(generate_event(event)?);
        }
    }

    // Format the code
    // Note: We use outer attributes (#[]) instead of inner (#![]) so the generated
    // code can be included with include!() in modules
    let code = quote! {
        use borsh::{BorshDeserialize, BorshSerialize};
        use bytemuck::{Pod, Zeroable};
        use solana_program::pubkey::Pubkey;

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
    let is_packed = ty
        .repr
        .as_ref()
        .and_then(|r| r.packed)
        .unwrap_or(false);

    let repr_attr = if use_bytemuck && is_packed {
        quote! { #[repr(C, packed)] }
    } else if use_bytemuck {
        quote! { #[repr(C)] }
    } else {
        quote! {}
    };

    match &ty.ty {
        TypeDefType::Struct { fields } => {
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
                Ok(quote! {
                    #docs
                    #repr_attr
                    #[derive(Debug, Clone, Copy, PartialEq)]
                    pub struct #name {
                        #(#field_tokens),*
                    }

                    unsafe impl bytemuck::Pod for #name {}
                    unsafe impl bytemuck::Zeroable for #name {}
                })
            } else {
                Ok(quote! {
                    #docs
                    #repr_attr
                    #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                    pub struct #name {
                        #(#field_tokens),*
                    }
                })
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
                Ok(quote! {
                    #docs
                    #repr_attr
                    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                    pub enum #name {
                        #(#variant_tokens),*
                    }

                    unsafe impl bytemuck::Pod for #name {}
                    unsafe impl bytemuck::Zeroable for #name {}
                })
            } else {
                Ok(quote! {
                    #docs
                    #repr_attr
                    #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
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

fn generate_instructions(instructions: &[Instruction]) -> Result<TokenStream> {
    let mut tokens = TokenStream::new();

    // Generate instruction enum
    let instruction_variants: Vec<_> = instructions
        .iter()
        .enumerate()
        .map(|(_idx, ix)| {
            let variant_name = format_ident!("{}", ix.name.to_pascal_case());
            if ix.args.is_empty() {
                quote! { #variant_name }
            } else {
                let args_struct = format_ident!("{}Args", ix.name.to_pascal_case());
                quote! { #variant_name(#args_struct) }
            }
        })
        .collect();

    // Generate discriminator match arms for serialization
    let serialize_arms: Vec<_> = instructions
        .iter()
        .enumerate()
        .map(|(idx, ix)| {
            let variant_name = format_ident!("{}", ix.name.to_pascal_case());
            let discriminator = if let Some(disc) = &ix.discriminator {
                let disc_bytes = disc.iter().map(|b| quote! { #b });
                quote! { [#(#disc_bytes),*] }
            } else {
                // Use index as discriminator if not provided (old format)
                let idx_bytes = (idx as u64).to_le_bytes();
                let bytes = idx_bytes.iter().map(|b| quote! { #b });
                quote! { [#(#bytes),*] }
            };
            
            if ix.args.is_empty() {
                quote! {
                    Self::#variant_name => {
                        writer.write_all(&#discriminator)?;
                        Ok(())
                    }
                }
            } else {
                quote! {
                    Self::#variant_name(args) => {
                        writer.write_all(&#discriminator)?;
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
                // Use index as discriminator if not provided (old format)
                (idx as u64).to_le_bytes().to_vec()
            };
            
            let disc_pattern = discriminator_bytes.iter().map(|b| quote! { #b });
            
            if ix.args.is_empty() {
                quote! {
                    [#(#disc_pattern),*] => Ok(Self::#variant_name)
                }
            } else {
                let args_struct = format_ident!("{}Args", ix.name.to_pascal_case());
                quote! {
                    [#(#disc_pattern),*] => {
                        let args = #args_struct::deserialize(buf)?;
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
            let args_struct = format_ident!("{}Args", ix.name.to_pascal_case());
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
                pub struct #args_struct {
                    #(#field_tokens),*
                }
            });
        }

        // Generate accounts struct for each instruction
        let accounts_struct = format_ident!("{}Accounts", ix.name.to_pascal_case());
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

        tokens.extend(quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #accounts_struct {
                #(#account_fields),*
            }
        });
    }

    Ok(tokens)
}

fn generate_errors(errors: &[Error]) -> Result<TokenStream> {
    let error_variants: Vec<_> = errors
        .iter()
        .map(|e| {
            let variant_name = format_ident!("{}", e.name.to_pascal_case());
            let msg = e.msg.as_deref().unwrap_or(&e.name);
            let msg_doc = format!("Error: {}", msg);
            quote! {
                #[doc = #msg_doc]
                #variant_name
            }
        })
        .collect();

    Ok(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ProgramError {
            #(#error_variants),*
        }
    })
}

fn generate_event(event: &Event) -> Result<TokenStream> {
    // If event has fields, generate a struct for it
    if let Some(fields) = &event.fields {
        let name = format_ident!("{}", event.name);
        let docs = format!("Event: {}", event.name);
        
        let field_tokens: Vec<_> = fields
            .iter()
            .map(|f| {
                let field_name = format_ident!("{}", f.name.to_snake_case());
                let field_type = map_idl_type(&f.ty);
                quote! {
                    pub #field_name: #field_type
                }
            })
            .collect();

        let discriminator_code = if let Some(disc) = &event.discriminator {
            let disc_bytes = disc.iter().map(|b| quote! { #b });
            quote! {
                impl #name {
                    pub const DISCRIMINATOR: [u8; 8] = [#(#disc_bytes),*];
                    
                    pub fn try_from_slice_with_discriminator(data: &[u8]) -> std::result::Result<Self, std::io::Error> {
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
                        Self::try_from_slice(&data[8..])
                    }
                }
            }
        } else {
            quote! {}
        };

        Ok(quote! {
            #[doc = #docs]
            #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
            pub struct #name {
                #(#field_tokens),*
            }
            
            #discriminator_code
        })
    } else {
        // Events without fields are just references - they're defined in types
        Ok(TokenStream::new())
    }
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

