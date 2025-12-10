use crate::idl::{ArrayType, *};
use anyhow::Result;
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate(idl: &Idl, module_name: &str) -> Result<String> {
    let mut tokens = TokenStream::new();

    // Generate module header
    let _module_ident = format_ident!("{}", module_name);

    // Generate types
    if let Some(types) = &idl.types {
        for ty in types {
            tokens.extend(generate_type_def(ty)?);
        }
    }

    // Generate account structs
    if let Some(accounts) = &idl.accounts {
        for account in accounts {
            tokens.extend(generate_account(account)?);
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
    let code = quote! {
        #![allow(clippy::all)]
        #![allow(dead_code)]

        use borsh::{BorshDeserialize, BorshSerialize};
        use solana_program::pubkey::Pubkey;

        #tokens
    };

    Ok(code.to_string())
}

fn generate_type_def(ty: &TypeDef) -> Result<TokenStream> {
    let name = format_ident!("{}", ty.name);
    let docs = generate_docs(ty.docs.as_ref());

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

            Ok(quote! {
                #docs
                #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                pub struct #name {
                    #(#field_tokens),*
                }
            })
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

            Ok(quote! {
                #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
                pub enum #name {
                    #(#variant_tokens),*
                }
            })
        }
    }
}

fn generate_account(_account: &Account) -> Result<TokenStream> {
    // Accounts in this IDL format are just references - they're defined in types
    // So we don't generate anything here, the type is already generated
    Ok(TokenStream::new())
}

fn generate_instructions(instructions: &[Instruction]) -> Result<TokenStream> {
    let mut tokens = TokenStream::new();

    // Generate instruction enum
    let instruction_variants: Vec<_> = instructions
        .iter()
        .map(|ix| {
            let variant_name = format_ident!("{}", ix.name.to_pascal_case());
            if ix.args.is_empty() {
                quote! { #variant_name }
            } else {
                let args_struct = format_ident!("{}Args", ix.name.to_pascal_case());
                quote! { #variant_name(#args_struct) }
            }
        })
        .collect();

    tokens.extend(quote! {
        #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq)]
        pub enum Instruction {
            #(#instruction_variants),*
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
                        let field_type = map_arg_type(&arg.ty);
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

fn generate_event(_event: &Event) -> Result<TokenStream> {
    // Events in this IDL format are just references - they're defined in types
    // So we don't generate anything here, the type is already generated
    Ok(TokenStream::new())
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
        IdlType::Array { array } => {
            match array {
                ArrayType::Tuple((inner, size)) => {
                    let inner_ty = map_idl_type(inner);
                    quote! { [#inner_ty; #size] }
                }
            }
        }
        IdlType::Defined { defined } => {
            let ident = format_ident!("{}", defined.name);
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

fn map_arg_type(ty: &str) -> TokenStream {
    match ty {
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
            let ident = format_ident!("{}", ty);
            quote! { #ident }
        }
    }
}
