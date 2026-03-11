//! Derive macros for TBF (Tauq Binary Format)
//!
//! This crate provides `#[derive(TbfEncode)]` and `#[derive(TbfDecode)]` for
//! compile-time schema generation, enabling optimal binary serialization.
//!
//! # Example
//!
//! ```ignore
//! use tbf_derive::{TbfEncode, TbfDecode};
//!
//! #[derive(TbfEncode, TbfDecode)]
//! struct User {
//!     id: u32,
//!     name: String,
//!     age: u32,
//! }
//!
//! // Serialize directly to bytes (no type tags, minimal overhead)
//! let user = User { id: 1, name: "Alice".into(), age: 30 };
//! let bytes = user.tbf_encode();
//!
//! // Deserialize back
//! let decoded = User::tbf_decode(&bytes).unwrap();
//! ```

use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Data, Fields, Type, Ident};

/// Derive macro for TBF encoding
///
/// Generates an optimized `tbf_encode()` method that serializes the struct
/// without type tags, using compile-time schema information.
#[proc_macro_derive(TbfEncode, attributes(tbf))]
pub fn derive_tbf_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => {
                    let mut active_fields = Vec::new();
                    for f in &fields.named {
                        let is_skipped = f.attrs.iter().any(|attr| {
                            attr.path().is_ident("tbf") && 
                            attr.parse_args::<syn::Ident>().map_or(false, |ident| ident == "skip")
                        });
                        if !is_skipped {
                            active_fields.push(f);
                        }
                    }

                    let field_count = active_fields.len();
                    let field_names: Vec<_> = active_fields.iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();
                    let field_types: Vec<_> = active_fields.iter()
                        .map(|f| &f.ty)
                        .collect();

                    // Generate field encoding
                    let field_encoders = field_names.iter().zip(field_types.iter())
                        .map(|(name, ty)| {
                            generate_field_encoder(name, ty)
                        });

                    // Generate schema info method
                    let field_name_strs: Vec<_> = field_names.iter()
                        .map(|n| n.to_string())
                        .collect();

                    quote! {
                        impl #impl_generics TbfEncode for #name #ty_generics #where_clause {
                            fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut tauq::tbf::StringDictionary) {
                                #(#field_encoders)*
                            }

                            fn tbf_schema() -> tauq::tbf::Schema {
                                let mut schema = tauq::tbf::Schema::new(stringify!(#name));
                                #(
                                    schema.add_field(#field_name_strs, <#field_types as TbfEncode>::tbf_schema_type());
                                )*
                                schema
                            }

                            fn tbf_field_count() -> usize {
                                #field_count
                            }
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let field_count = fields.unnamed.len();
                    let field_indices: Vec<_> = (0..field_count).collect();
                    let field_types: Vec<_> = fields.unnamed.iter()
                        .map(|f| &f.ty)
                        .collect();

                    let field_encoders = field_indices.iter().zip(field_types.iter())
                        .map(|(idx, ty)| {
                            let idx = syn::Index::from(*idx);
                            generate_tuple_field_encoder(&idx, ty)
                        });

                    quote! {
                        impl #impl_generics TbfEncode for #name #ty_generics #where_clause {
                            fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut tauq::tbf::StringDictionary) {
                                #(#field_encoders)*
                            }

                            fn tbf_schema() -> tauq::tbf::Schema {
                                tauq::tbf::Schema::new(stringify!(#name))
                            }

                            fn tbf_field_count() -> usize {
                                #field_count
                            }
                        }
                    }
                }
                Fields::Unit => {
                    quote! {
                        impl #impl_generics TbfEncode for #name #ty_generics #where_clause {
                            fn tbf_encode_to(&self, _buf: &mut Vec<u8>, _dict: &mut tauq::tbf::StringDictionary) {
                                // Unit struct - no data to encode
                            }

                            fn tbf_schema() -> tauq::tbf::Schema {
                                tauq::tbf::Schema::new(stringify!(#name))
                            }

                            fn tbf_field_count() -> usize {
                                0
                            }
                        }
                    }
                }
            }
        }
        Data::Enum(data) => {
            let variant_encoders = data.variants.iter().enumerate().map(|(idx, variant)| {
                let variant_name = &variant.ident;
                let idx = idx as u32;

                match &variant.fields {
                    Fields::Unit => {
                        quote! {
                            #name::#variant_name => {
                                tauq::tbf::encode_varint(#idx as u64, buf);
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let field_names: Vec<_> = (0..fields.unnamed.len())
                            .map(|i| format_ident!("f{}", i))
                            .collect();
                        let field_types: Vec<_> = fields.unnamed.iter()
                            .map(|f| &f.ty)
                            .collect();

                        let encoders = field_names.iter().zip(field_types.iter())
                            .map(|(name, ty)| generate_value_encoder(name, ty));

                        quote! {
                            #name::#variant_name(#(#field_names),*) => {
                                tauq::tbf::encode_varint(#idx as u64, buf);
                                #(#encoders)*
                            }
                        }
                    }
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named.iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let field_types: Vec<_> = fields.named.iter()
                            .map(|f| &f.ty)
                            .collect();

                        let encoders = field_names.iter().zip(field_types.iter())
                            .map(|(name, ty)| generate_value_encoder(name, ty));

                        quote! {
                            #name::#variant_name { #(#field_names),* } => {
                                tauq::tbf::encode_varint(#idx as u64, buf);
                                #(#encoders)*
                            }
                        }
                    }
                }
            });

            quote! {
                impl #impl_generics TbfEncode for #name #ty_generics #where_clause {
                    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut tauq::tbf::StringDictionary) {
                        match self {
                            #(#variant_encoders)*
                        }
                    }

                    fn tbf_schema() -> tauq::tbf::Schema {
                        tauq::tbf::Schema::new(stringify!(#name))
                    }

                    fn tbf_field_count() -> usize {
                        0
                    }
                }
            }
        }
        Data::Union(_) => {
            panic!("TbfEncode cannot be derived for unions");
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for TBF decoding
///
/// Generates an optimized `tbf_decode()` method that deserializes the struct
/// without type tags, using compile-time schema information.
#[proc_macro_derive(TbfDecode, attributes(tbf))]
pub fn derive_tbf_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => {
                    let mut active_field_names = Vec::new();
                    let mut active_field_types = Vec::new();
                    let mut all_field_names = Vec::new();
                    let mut skipped_field_names = Vec::new();

                    for f in &fields.named {
                        let name = f.ident.as_ref().unwrap();
                        all_field_names.push(name);

                        let is_skipped = f.attrs.iter().any(|attr| {
                            attr.path().is_ident("tbf") &&
                            attr.parse_args::<syn::Ident>().map_or(false, |ident| ident == "skip")
                        });

                        if is_skipped {
                            skipped_field_names.push(name);
                        } else {
                            active_field_names.push(name);
                            active_field_types.push(&f.ty);
                        }
                    }

                    let field_decoders = active_field_names.iter().zip(active_field_types.iter())
                        .map(|(name, ty)| {
                            generate_field_decoder(name, ty)
                        });

                    let default_assignments = skipped_field_names.iter().map(|name| {
                        quote! { let #name = Default::default(); }
                    });

                    quote! {
                        impl #impl_generics TbfDecode for #name #ty_generics #where_clause {
                            fn tbf_decode_from(
                                buf: &[u8],
                                pos: &mut usize,
                                dict: &tauq::tbf::BorrowedDictionary
                            ) -> Result<Self, tauq::TauqError> {
                                #(#field_decoders)*
                                #(#default_assignments)*

                                Ok(Self {
                                    #(#all_field_names),*
                                })
                            }
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let field_count = fields.unnamed.len();
                    let field_names: Vec<_> = (0..field_count)
                        .map(|i| format_ident!("f{}", i))
                        .collect();
                    let field_types: Vec<_> = fields.unnamed.iter()
                        .map(|f| &f.ty)
                        .collect();

                    let field_decoders = field_names.iter().zip(field_types.iter())
                        .map(|(name, ty)| generate_value_decoder(name, ty));

                    quote! {
                        impl #impl_generics TbfDecode for #name #ty_generics #where_clause {
                            fn tbf_decode_from(
                                buf: &[u8],
                                pos: &mut usize,
                                dict: &tauq::tbf::BorrowedDictionary
                            ) -> Result<Self, tauq::TauqError> {
                                #(#field_decoders)*

                                Ok(Self(#(#field_names),*))
                            }
                        }
                    }
                }
                Fields::Unit => {
                    quote! {
                        impl #impl_generics TbfDecode for #name #ty_generics #where_clause {
                            fn tbf_decode_from(
                                _buf: &[u8],
                                _pos: &mut usize,
                                _dict: &tauq::tbf::BorrowedDictionary
                            ) -> Result<Self, tauq::TauqError> {
                                Ok(Self)
                            }
                        }
                    }
                }
            }
        }
        Data::Enum(data) => {
            let variant_decoders = data.variants.iter().enumerate().map(|(idx, variant)| {
                let variant_name = &variant.ident;
                let idx = idx as u32;

                match &variant.fields {
                    Fields::Unit => {
                        quote! {
                            #idx => Ok(#name::#variant_name),
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let field_names: Vec<_> = (0..fields.unnamed.len())
                            .map(|i| format_ident!("f{}", i))
                            .collect();
                        let field_types: Vec<_> = fields.unnamed.iter()
                            .map(|f| &f.ty)
                            .collect();

                        let decoders = field_names.iter().zip(field_types.iter())
                            .map(|(name, ty)| generate_value_decoder(name, ty));

                        quote! {
                            #idx => {
                                #(#decoders)*
                                Ok(#name::#variant_name(#(#field_names),*))
                            }
                        }
                    }
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named.iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let field_types: Vec<_> = fields.named.iter()
                            .map(|f| &f.ty)
                            .collect();

                        let decoders = field_names.iter().zip(field_types.iter())
                            .map(|(name, ty)| generate_field_decoder(name, ty));

                        quote! {
                            #idx => {
                                #(#decoders)*
                                Ok(#name::#variant_name { #(#field_names),* })
                            }
                        }
                    }
                }
            });

            quote! {
                impl #impl_generics TbfDecode for #name #ty_generics #where_clause {
                    fn tbf_decode_from(
                        buf: &[u8],
                        pos: &mut usize,
                        dict: &tauq::tbf::BorrowedDictionary
                    ) -> Result<Self, tauq::TauqError> {
                        let (variant_idx, len) = tauq::tbf::decode_varint(&buf[*pos..])?;
                        *pos += len;

                        match variant_idx as u32 {
                            #(#variant_decoders)*
                            _ => Err(tauq::TauqError::Interpret(
                                tauq::error::InterpretError::new("Invalid enum variant")
                            )),
                        }
                    }
                }
            }
        }
        Data::Union(_) => {
            panic!("TbfDecode cannot be derived for unions");
        }
    };

    TokenStream::from(expanded)
}

/// Generate encoder for a named struct field
fn generate_field_encoder(name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    // Check for common types and generate optimized encoders
    let ty_str = quote!(#ty).to_string();

    match ty_str.as_str() {
        "u8" => quote! { buf.push(self.#name); },
        "u16" | "u32" | "u64" | "usize" => quote! {
            tauq::tbf::encode_varint(self.#name as u64, buf);
        },
        "i8" => quote! { buf.push(self.#name as u8); },
        "i16" | "i32" | "i64" | "isize" => quote! {
            tauq::tbf::encode_signed_varint(self.#name as i64, buf);
        },
        "f32" => quote! { buf.extend_from_slice(&self.#name.to_le_bytes()); },
        "f64" => quote! { buf.extend_from_slice(&self.#name.to_le_bytes()); },
        "bool" => quote! { buf.push(if self.#name { 1 } else { 0 }); },
        "String" => quote! {
            let idx = dict.intern(&self.#name);
            tauq::tbf::encode_varint(idx as u64, buf);
        },
        _ => {
            // For other types, use the TbfEncode trait
            quote! {
                self.#name.tbf_encode_to(buf, dict);
            }
        }
    }
}

/// Generate encoder for a tuple struct field
fn generate_tuple_field_encoder(idx: &syn::Index, ty: &Type) -> proc_macro2::TokenStream {
    let ty_str = quote!(#ty).to_string();

    match ty_str.as_str() {
        "u8" => quote! { buf.push(self.#idx); },
        "u16" | "u32" | "u64" | "usize" => quote! {
            tauq::tbf::encode_varint(self.#idx as u64, buf);
        },
        "i8" => quote! { buf.push(self.#idx as u8); },
        "i16" | "i32" | "i64" | "isize" => quote! {
            tauq::tbf::encode_signed_varint(self.#idx as i64, buf);
        },
        "f32" => quote! { buf.extend_from_slice(&self.#idx.to_le_bytes()); },
        "f64" => quote! { buf.extend_from_slice(&self.#idx.to_le_bytes()); },
        "bool" => quote! { buf.push(if self.#idx { 1 } else { 0 }); },
        "String" => quote! {
            let idx = dict.intern(&self.#idx);
            tauq::tbf::encode_varint(idx as u64, buf);
        },
        _ => {
            quote! {
                self.#idx.tbf_encode_to(buf, dict);
            }
        }
    }
}

/// Generate encoder for a value (used in enums)
fn generate_value_encoder(name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    let ty_str = quote!(#ty).to_string();

    match ty_str.as_str() {
        "u8" => quote! { buf.push(*#name); },
        "u16" | "u32" | "u64" | "usize" => quote! {
            tauq::tbf::encode_varint(*#name as u64, buf);
        },
        "i8" => quote! { buf.push(*#name as u8); },
        "i16" | "i32" | "i64" | "isize" => quote! {
            tauq::tbf::encode_signed_varint(*#name as i64, buf);
        },
        "f32" => quote! { buf.extend_from_slice(&#name.to_le_bytes()); },
        "f64" => quote! { buf.extend_from_slice(&#name.to_le_bytes()); },
        "bool" => quote! { buf.push(if *#name { 1 } else { 0 }); },
        "String" => quote! {
            let idx = dict.intern(#name);
            tauq::tbf::encode_varint(idx as u64, buf);
        },
        _ => {
            quote! {
                #name.tbf_encode_to(buf, dict);
            }
        }
    }
}

/// Generate decoder for a named struct field
fn generate_field_decoder(name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    let ty_str = quote!(#ty).to_string();

    match ty_str.as_str() {
        "u8" => quote! {
            if *pos >= buf.len() {
                return Err(tauq::TauqError::Interpret(
                    tauq::error::InterpretError::new("Unexpected EOF")
                ));
            }
            let #name = buf[*pos];
            *pos += 1;
        },
        "u16" => quote! {
            let (v, len) = tauq::tbf::decode_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v as u16;
        },
        "u32" => quote! {
            let (v, len) = tauq::tbf::decode_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v as u32;
        },
        "u64" => quote! {
            let (v, len) = tauq::tbf::decode_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v;
        },
        "usize" => quote! {
            let (v, len) = tauq::tbf::decode_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v as usize;
        },
        "i8" => quote! {
            if *pos >= buf.len() {
                return Err(tauq::TauqError::Interpret(
                    tauq::error::InterpretError::new("Unexpected EOF")
                ));
            }
            let #name = buf[*pos] as i8;
            *pos += 1;
        },
        "i16" => quote! {
            let (v, len) = tauq::tbf::decode_signed_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v as i16;
        },
        "i32" => quote! {
            let (v, len) = tauq::tbf::decode_signed_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v as i32;
        },
        "i64" => quote! {
            let (v, len) = tauq::tbf::decode_signed_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v;
        },
        "isize" => quote! {
            let (v, len) = tauq::tbf::decode_signed_varint(&buf[*pos..])?;
            *pos += len;
            let #name = v as isize;
        },
        "f32" => quote! {
            if *pos + 4 > buf.len() {
                return Err(tauq::TauqError::Interpret(
                    tauq::error::InterpretError::new("Unexpected EOF")
                ));
            }
            let bytes: [u8; 4] = buf[*pos..*pos + 4].try_into().unwrap();
            *pos += 4;
            let #name = f32::from_le_bytes(bytes);
        },
        "f64" => quote! {
            if *pos + 8 > buf.len() {
                return Err(tauq::TauqError::Interpret(
                    tauq::error::InterpretError::new("Unexpected EOF")
                ));
            }
            let bytes: [u8; 8] = buf[*pos..*pos + 8].try_into().unwrap();
            *pos += 8;
            let #name = f64::from_le_bytes(bytes);
        },
        "bool" => quote! {
            if *pos >= buf.len() {
                return Err(tauq::TauqError::Interpret(
                    tauq::error::InterpretError::new("Unexpected EOF")
                ));
            }
            let #name = buf[*pos] != 0;
            *pos += 1;
        },
        "String" => quote! {
            let (idx, len) = tauq::tbf::decode_varint(&buf[*pos..])?;
            *pos += len;
            let #name = dict.get(idx as u32)
                .ok_or_else(|| tauq::TauqError::Interpret(
                    tauq::error::InterpretError::new("Invalid string index")
                ))?
                .to_string();
        },
        _ => {
            quote! {
                let #name = <#ty as TbfDecode>::tbf_decode_from(buf, pos, dict)?;
            }
        }
    }
}

/// Generate decoder for a value (used in enums and tuples)
fn generate_value_decoder(name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    generate_field_decoder(name, ty)
}
