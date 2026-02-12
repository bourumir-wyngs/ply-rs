//!
//! Procedural macros for the ply-rs-bw crate.
//!
//! The primary user-facing derive is `#[derive(PlyAccess)]`, which generates
//! implementations for both `PropertyAccess` and `PropertySchema]` for a struct.
//! Fields can be annotated with `#[ply(name = "...")]` to bind them to specific
//! PLY property names. Optional fields (i.e., `Option<T>`) are treated as optional
//! properties in the header validation; all other fields are required.
//!
//! Additionally, `#[derive(FromPly)]` can be used on a container struct to map
//! PLY element names to `Vec<T>` fields (`T: PlyAccess`). This enables loading a
//! whole file with a single call to `Container::read_ply(&mut reader)`.
//!
//! Examples
//! --------
//! Define element types and a mesh container:
//!
//! ```ignore
//! use ply_rs_bw::{PlyAccess, FromPly};
//!
//! #[derive(Debug, Default, PlyAccess)]
//! struct Vertex {
//!     #[ply(name = "x")] x: f32,
//!     #[ply(name = "y")] y: f32,
//!     #[ply(name = "z")] z: f32,
//! }
//!
//! #[derive(Debug, Default, PlyAccess)]
//! struct Face {
//!     #[ply(name = "vertex_indices")] indices: Vec<u32>,
//! }
//!
//! #[derive(Debug, FromPly)]
//! struct Mesh {
//!     #[ply(name = "vertex")] vertices: Vec<Vertex>,
//!     #[ply(name = "face")] faces: Vec<Face>,
//! }
//! ```
//!
//! Then read a file:
//!
//! ```ignore
//! let mut file = std::fs::File::open("mesh.ply")?;
//! let mesh = Mesh::read_ply(&mut file)?;
//! ```
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type, PathArguments, GenericArgument};
use proc_macro_crate::{crate_name, FoundCrate};

fn get_crate_name() -> proc_macro2::TokenStream {
    let found_crate = crate_name("ply-rs-bw");

    match found_crate {
        Ok(FoundCrate::Itself) => quote!(::ply_rs_bw),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::ply_rs_bw),
    }
}

struct PlyAttr {
    name: String,
    count_type: Option<String>,
    explicit_type: Option<String>,
}

/// Parses the `#[ply(...)]` attributes and returns the PLY property name and optional count type.
fn parse_ply_attr(field: &syn::Field) -> Result<PlyAttr, syn::Error> {
    let mut attr_data = PlyAttr {
        name: field.ident.as_ref().unwrap().to_string(),
        count_type: None,
        explicit_type: None,
    };

    for attr in &field.attrs {
        if attr.path().is_ident("ply") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    attr_data.name = s.value();
                    Ok(())
                } else if meta.path.is_ident("count") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    attr_data.count_type = Some(s.value());
                    Ok(())
                } else if meta.path.is_ident("type") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    attr_data.explicit_type = Some(s.value());
                    Ok(())
                } else {
                    Err(meta.error(format!("unknown ply attribute: {}", meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default())))
                }
            })?;
        }
    }

    Ok(attr_data)
}

/// Parses the `#[ply(name = "...")]` attribute and returns the PLY property name.
fn parse_ply_name(field: &syn::Field) -> Result<String, syn::Error> {
    Ok(parse_ply_attr(field)?.name)
}

/// Procedural macro to derive the `PropertyAccess` trait.
///
/// This macro generates the `set_property` method, which maps PLY property names
/// to struct fields and handles type conversions.
///
/// Supported attributes:
/// - `#[ply(name = "property_name")]`: Maps the field to a specific PLY property name.
///
/// Note: Optional fields (`Option<T>`) are only supported when reading PLY files.
/// When writing, all properties must have a value, so `Option<T>` is not allowed.
#[proc_macro_derive(PropertyAccess, attributes(ply))]
pub fn derive_property_access(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "PropertyAccess only supports named fields").to_compile_error()),
        },
        _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "PropertyAccess only supports structs").to_compile_error()),
    };

    let mut set_arms = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for field in fields {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let ply_name = match parse_ply_name(field) {
            Ok(name) => name,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };

        if !seen_names.insert(ply_name.clone()) {
            return TokenStream::from(syn::Error::new_spanned(field, format!("duplicate ply property name: {}", ply_name)).to_compile_error());
        }

        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());

        let is_opt = is_option(field_type);
        
        let conversion = if let Some(inner_type) = is_opt.as_ref() {
             generate_conversion(inner_type)
        } else {
             generate_conversion(field_type)
        };

        let conversion = match conversion {
            Ok(tokens) => tokens,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };

        let arm = if is_opt.is_some() {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        let mut _val = val;
                        unsafe {
                            self.#field_name = Some(std::ptr::read(&mut _val as *mut _ as *const _));
                            std::mem::forget(_val);
                        }
                    }
                }
            }
        } else {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        let mut _val = val;
                        unsafe {
                            self.#field_name = std::ptr::read(&mut _val as *mut _ as *const _);
                            std::mem::forget(_val);
                        }
                    }
                }
            }
        };
        set_arms.push(arm);

    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ply_rs = get_crate_name();
    let expanded = quote! {
        impl #impl_generics #ply_rs::ply::PropertyAccess for #name #ty_generics #where_clause {
            fn new() -> Self {
                Default::default()
            }

            fn set_property(&mut self, key: &str, property: #ply_rs::ply::Property) {
                match key {
                    #( #set_arms )*
                    _ => {},
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Procedural macro to derive the `PropertySchema` trait.
///
/// This macro generates the `schema` method, which describes the expected properties
/// of the struct and whether they are required or optional (based on `Option<T>`).
#[proc_macro_derive(PropertySchema, attributes(ply))]
pub fn derive_property_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "PropertySchema only supports named fields").to_compile_error()),
        },
        _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "PropertySchema only supports structs").to_compile_error()),
    };

    let mut schema_entries = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for field in fields {
        let ply_name = match parse_ply_name(field) {
            Ok(name) => name,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };

        if !seen_names.insert(ply_name.clone()) {
            return TokenStream::from(syn::Error::new_spanned(field, format!("duplicate ply property name: {}", ply_name)).to_compile_error());
        }

        let ply_rs = get_crate_name();
        let requiredness = if is_option(&field.ty).is_some() {
            quote! { #ply_rs::ply::Requiredness::Optional }
        } else {
            quote! { #ply_rs::ply::Requiredness::Required }
        };

        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());
        schema_entries.push(quote! {
            (#ply_name_lit.to_string(), #requiredness)
        });
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ply_rs = get_crate_name();
    let expanded = quote! {
        impl #impl_generics #ply_rs::ply::PropertySchema for #name #ty_generics #where_clause {
            fn schema() -> Vec<(String, #ply_rs::ply::Requiredness)> {
                vec![
                    #( #schema_entries ),*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}

/// Procedural macro to derive the `PlyAccess` trait.
///
/// This is a convenience macro that derives both `PropertyAccess` and `PropertySchema`.
/// It is the primary macro for defining PLY element structures.
///
/// Note: Optional fields (`Option<T>`) are only supported when reading PLY files.
/// When writing, all properties must have a value, so `Option<T>` is not allowed.
#[proc_macro_derive(PlyAccess, attributes(ply))]
pub fn derive_ply_access(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Reuse logic from PropertyAccess derive
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "PlyAccess only supports named fields").to_compile_error()),
        },
        _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "PlyAccess only supports structs").to_compile_error()),
    };

    let mut set_arms = Vec::new();
    let mut schema_entries = Vec::new();
    let mut type_schema_entries = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    // Getters
    let mut get_char_arms = Vec::new();
    let mut get_uchar_arms = Vec::new();
    let mut get_short_arms = Vec::new();
    let mut get_ushort_arms = Vec::new();
    let mut get_int_arms = Vec::new();
    let mut get_uint_arms = Vec::new();
    let mut get_float_arms = Vec::new();
    let mut get_double_arms = Vec::new();

    // List getters
    let mut get_list_char_arms = Vec::new();
    let mut get_list_uchar_arms = Vec::new();
    let mut get_list_short_arms = Vec::new();
    let mut get_list_ushort_arms = Vec::new();
    let mut get_list_int_arms = Vec::new();
    let mut get_list_uint_arms = Vec::new();
    let mut get_list_float_arms = Vec::new();
    let mut get_list_double_arms = Vec::new();

    for field in fields {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let ply_attr = match parse_ply_attr(field) {
            Ok(attr) => attr,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };
        let ply_name = ply_attr.name;

        if !seen_names.insert(ply_name.clone()) {
            return TokenStream::from(syn::Error::new_spanned(field, format!("duplicate ply property name: {}", ply_name)).to_compile_error());
        }

        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());

        let is_opt = is_option(field_type);
        let conversion_type = if let Some(inner) = is_opt.as_ref() { inner } else { field_type };

        // Support explicit type override even for generic fields
        let conversion = if let Some(et) = ply_attr.explicit_type.as_deref() {
            let ply_rs = get_crate_name();
            let scalar_type_from_str = |s: &str| -> Option<proc_macro2::TokenStream> {
                match s {
                    "char" | "i8" => Some(quote! { i8 }),
                    "uchar" | "u8" => Some(quote! { u8 }),
                    "short" | "i16" => Some(quote! { i16 }),
                    "ushort" | "u16" => Some(quote! { u16 }),
                    "int" | "i32" => Some(quote! { i32 }),
                    "uint" | "u32" => Some(quote! { u32 }),
                    "float" | "f32" => Some(quote! { f32 }),
                    "double" | "f64" => Some(quote! { f64 }),
                    _ => None,
                }
            };
            if let Some(cast_ty) = scalar_type_from_str(et) {
                let (scalar_variants, _) = scalar_match_and_cast_tokens_with_ty(&cast_ty, &ply_rs);
                Ok(quote! {
                    match property {
                        #(#scalar_variants)*
                        _ => None,
                    }.map(|v: #cast_ty| v)
                })
            } else {
                 generate_conversion(conversion_type)
            }
        } else {
             generate_conversion(conversion_type)
        };

        let conversion = match conversion {
            Ok(tokens) => tokens,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };

        let arm = if is_opt.is_some() {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        let mut _val = val;
                        unsafe {
                            self.#field_name = Some(std::ptr::read(&mut _val as *mut _ as *const _));
                            std::mem::forget(_val);
                        }
                    }
                }
            }
        } else {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        let mut _val = val;
                        unsafe {
                            self.#field_name = std::ptr::read(&mut _val as *mut _ as *const _);
                            std::mem::forget(_val);
                        }
                    }
                }
            }
        };
        set_arms.push(arm);

        let ply_rs = get_crate_name();
        let requiredness = if is_opt.is_some() {
            quote! { #ply_rs::ply::Requiredness::Optional }
        } else {
            quote! { #ply_rs::ply::Requiredness::Required }
        };
        schema_entries.push(quote! {
            (#ply_name_lit.to_string(), #requiredness)
        });

        let prop_type_token = match get_property_type_tokens(field_type, ply_attr.count_type.as_deref(), ply_attr.explicit_type.as_deref(), Some(field)) {
            Ok(tokens) => tokens,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };
        type_schema_entries.push(quote! {
             (#ply_name_lit.to_string(), #prop_type_token)
        });

        // Getter logic
        let effective_kind = if let Some(et) = ply_attr.explicit_type.as_deref() {
            let scalar_type_from_str_kind = |s: &str| -> Option<ScalarKind> {
                match s {
                    "char" | "i8" => Some(ScalarKind::I8),
                    "uchar" | "u8" => Some(ScalarKind::U8),
                    "short" | "i16" => Some(ScalarKind::I16),
                    "ushort" | "u16" => Some(ScalarKind::U16),
                    "int" | "i32" => Some(ScalarKind::I32),
                    "uint" | "u32" => Some(ScalarKind::U32),
                    "float" | "f32" => Some(ScalarKind::F32),
                    "double" | "f64" => Some(ScalarKind::F64),
                    _ => None,
                }
            };
            scalar_type_from_str_kind(et)
        } else {
            scalar_ident(conversion_type)
        };

        if let Some(inner_vec_type) = is_vec(conversion_type) {
             // List type
             let inner_kind = if let Some(et) = ply_attr.explicit_type.as_deref() {
                let scalar_type_from_str_kind = |s: &str| -> Option<ScalarKind> {
                    match s {
                        "char" | "i8" => Some(ScalarKind::I8),
                        "uchar" | "u8" => Some(ScalarKind::U8),
                        "short" | "i16" => Some(ScalarKind::I16),
                        "ushort" | "u16" => Some(ScalarKind::U16),
                        "int" | "i32" => Some(ScalarKind::I32),
                        "uint" | "u32" => Some(ScalarKind::U32),
                        "float" | "f32" => Some(ScalarKind::F32),
                        "double" | "f64" => Some(ScalarKind::F64),
                        _ => None,
                    }
                };
                scalar_type_from_str_kind(et)
             } else {
                scalar_ident(inner_vec_type)
             };

             if let Some(kind) = inner_kind {
                 use ScalarKind::*;
                 let (_, cast_ty) = scalar_type_tokens(&kind, &ply_rs);
                 let field_access_list = if is_opt.is_some() {
                      quote! { self.#field_name.as_deref().map(|v| v as &[#cast_ty]) }
                 } else {
                      quote! { Some(self.#field_name.as_slice() as &[#cast_ty]) }
                 };
                 let arm = quote! { key if key == #ply_name_lit => #field_access_list, };
                 match kind {
                    I8 => get_list_char_arms.push(arm),
                    U8 => get_list_uchar_arms.push(arm),
                    I16 => get_list_short_arms.push(arm),
                    U16 => get_list_ushort_arms.push(arm),
                    I32 => get_list_int_arms.push(arm),
                    U32 => get_list_uint_arms.push(arm),
                    F32 => get_list_float_arms.push(arm),
                    F64 => get_list_double_arms.push(arm),
                 }
             }
        } else if let Some(kind) = effective_kind {
             // Scalar type
             use ScalarKind::*;
             let (_, cast_ty) = scalar_type_tokens(&kind, &ply_rs);
             let field_access_scalar = if is_opt.is_some() {
                  quote! { self.#field_name.map(|v| {
                      let mut _v = v;
                      unsafe {
                          let _casted: #cast_ty = std::ptr::read(&mut _v as *mut _ as *const _);
                          std::mem::forget(_v);
                          _casted
                      }
                  }) }
             } else {
                  quote! { Some({
                      let mut _v = self.#field_name;
                      unsafe {
                          let _casted: #cast_ty = std::ptr::read(&mut _v as *mut _ as *const _);
                          std::mem::forget(_v);
                          _casted
                      }
                  }) }
             };
             let arm = quote! { key if key == #ply_name_lit => #field_access_scalar, };
             match kind {
                I8 => get_char_arms.push(arm),
                U8 => get_uchar_arms.push(arm),
                I16 => get_short_arms.push(arm),
                U16 => get_ushort_arms.push(arm),
                I32 => get_int_arms.push(arm),
                U32 => get_uint_arms.push(arm),
                F32 => get_float_arms.push(arm),
                F64 => get_double_arms.push(arm),
             }
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ply_rs = get_crate_name();
    let expanded = quote! {
        impl #impl_generics #ply_rs::ply::PropertyAccess for #name #ty_generics #where_clause {
            fn new() -> Self { Default::default() }
            fn set_property(&mut self, key: &str, property: #ply_rs::ply::Property) {
                match key {
                    #( #set_arms )*
                    _ => {},
                }
            }
            fn get_char(&self, key: &str) -> Option<i8> { match key { #( #get_char_arms )* _ => None } }
            fn get_uchar(&self, key: &str) -> Option<u8> { match key { #( #get_uchar_arms )* _ => None } }
            fn get_short(&self, key: &str) -> Option<i16> { match key { #( #get_short_arms )* _ => None } }
            fn get_ushort(&self, key: &str) -> Option<u16> { match key { #( #get_ushort_arms )* _ => None } }
            fn get_int(&self, key: &str) -> Option<i32> { match key { #( #get_int_arms )* _ => None } }
            fn get_uint(&self, key: &str) -> Option<u32> { match key { #( #get_uint_arms )* _ => None } }
            fn get_float(&self, key: &str) -> Option<f32> { match key { #( #get_float_arms )* _ => None } }
            fn get_double(&self, key: &str) -> Option<f64> { match key { #( #get_double_arms )* _ => None } }
            
            fn get_list_char(&self, key: &str) -> Option<&[i8]> { match key { #( #get_list_char_arms )* _ => None } }
            fn get_list_uchar(&self, key: &str) -> Option<&[u8]> { match key { #( #get_list_uchar_arms )* _ => None } }
            fn get_list_short(&self, key: &str) -> Option<&[i16]> { match key { #( #get_list_short_arms )* _ => None } }
            fn get_list_ushort(&self, key: &str) -> Option<&[u16]> { match key { #( #get_list_ushort_arms )* _ => None } }
            fn get_list_int(&self, key: &str) -> Option<&[i32]> { match key { #( #get_list_int_arms )* _ => None } }
            fn get_list_uint(&self, key: &str) -> Option<&[u32]> { match key { #( #get_list_uint_arms )* _ => None } }
            fn get_list_float(&self, key: &str) -> Option<&[f32]> { match key { #( #get_list_float_arms )* _ => None } }
            fn get_list_double(&self, key: &str) -> Option<&[f64]> { match key { #( #get_list_double_arms )* _ => None } }
        }
        impl #impl_generics #ply_rs::ply::PropertySchema for #name #ty_generics #where_clause {
            fn schema() -> Vec<(String, #ply_rs::ply::Requiredness)> {
                vec![ #( #schema_entries ),* ]
            }
        }
        impl #impl_generics #ply_rs::ply::PropertyTypeSchema for #name #ty_generics #where_clause {
            fn property_type_schema() -> Vec<(String, #ply_rs::ply::PropertyType)> {
                vec![ #( #type_schema_entries ),* ]
            }
        }
    };

    TokenStream::from(expanded)
}

/// Procedural macro to derive the `FromPly` trait.
///
/// This macro allows a struct to be read directly from a PLY file by mapping
/// element names to `Vec<T>` fields.
#[proc_macro_derive(FromPly, attributes(ply))]
pub fn derive_from_ply(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "FromPly only supports named fields").to_compile_error()),
        },
        _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "FromPly only supports structs").to_compile_error()),
    };

    let mut field_names = Vec::new();
    let mut inner_tys = Vec::new();
    let mut ply_names = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let ply_name = match parse_ply_name(field) {
            Ok(name) => name,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };

        if !seen_names.insert(ply_name.clone()) {
            return TokenStream::from(syn::Error::new_spanned(field, format!("duplicate ply element name: {}", ply_name)).to_compile_error());
        }

        let inner_ty = match is_vec(field_type) {
            Some(ty) => ty,
            None => return TokenStream::from(syn::Error::new_spanned(field_type, "FromPly currently only supports Vec<T> fields").to_compile_error()),
        };

        field_names.push(field_name);
        inner_tys.push(inner_ty);
        ply_names.push(ply_name);
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ply_rs = get_crate_name();
    let expanded = quote! {
        impl #impl_generics #ply_rs::parser::FromPly for #name #ty_generics #where_clause {
            fn read_ply<_T_READER: std::io::Read>(reader: &mut _T_READER) -> std::io::Result<Self> {
                let mut reader = std::io::BufReader::new(reader);
                // We need a parser to read the header. Any element type will do.
                let parser = #ply_rs::parser::Parser::<#ply_rs::ply::DefaultElement>::new();
                let header = parser.read_header(&mut reader)?;

                #(
                    let mut #field_names = Vec::new();
                )*

                for (name, element_def) in &header.elements {
                    match name.as_str() {
                        #(
                            #ply_names => {
                                let p = #ply_rs::parser::Parser::<#inner_tys>::new();
                                #field_names = p.read_payload_for_element(&mut reader, element_def, &header)?;
                            }
                        )*
                        _ => {
                             // skip unknown elements
                             let p = #ply_rs::parser::Parser::<#ply_rs::ply::DefaultElement>::new();
                             let _ = p.read_payload_for_element(&mut reader, element_def, &header)?;
                        }
                    }
                }

                Ok(#name {
                    #( #field_names, )*
                })
            }
        }
    };

    TokenStream::from(expanded)
}

/// Checks if a type is `Option<T>` and returns the inner type `T`.
fn is_option(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// Generates the conversion logic from a `Property` to a specific Rust type.
///
/// Handles both scalar types and `Vec<T>` for list properties.
fn generate_conversion(ty: &Type) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ply_rs = get_crate_name();

    // Recognize scalars and Vec<scalar>
    if let Some(inner) = is_vec(ty) {
        let elem = scalar_ident(inner);
        if let Some(elem_ty) = elem {
            let (list_variants, cast_ty) = list_match_and_cast_tokens(&elem_ty, &ply_rs);
            return Ok(quote! {
                match property {
                    #(#list_variants)*
                    _ => None,
                }.map(|v: Vec<#cast_ty>| v)
            });
        }
    }

    if let Some(s) = scalar_ident(ty) {
        let (scalar_variants, cast_ty) = scalar_match_and_cast_tokens(&s, &ply_rs);
        return Ok(quote! {
            match property {
                #(#scalar_variants)*
                _ => None,
            }.map(|v: #cast_ty| v)
        });
    }

    // Fallback: not recognized
    Err(syn::Error::new_spanned(ty, "Unsupported field type for PlyAccess. Supported types: i8, u8, i16, u16, i32, u32, f32, f64, and Vec<T> of these."))
}

/// Checks if a type is `Vec<T>` and returns the inner type `T`.
fn is_vec(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

enum ScalarKind { I8, U8, I16, U16, I32, U32, F32, F64 }

/// Identifies supported scalar types.
fn scalar_ident(ty: &Type) -> Option<ScalarKind> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if !seg.arguments.is_empty() {
                return None;
            }
            return match seg.ident.to_string().as_str() {
                "i8" => Some(ScalarKind::I8),
                "u8" => Some(ScalarKind::U8),
                "i16" => Some(ScalarKind::I16),
                "u16" => Some(ScalarKind::U16),
                "i32" => Some(ScalarKind::I32),
                "u32" => Some(ScalarKind::U32),
                "f32" => Some(ScalarKind::F32),
                "f64" => Some(ScalarKind::F64),
                _ => None,
            };
        }
    }
    None
}

/// Generates match arms and casting logic for scalar properties.
fn scalar_match_and_cast_tokens(kind: &ScalarKind, ply_rs: &proc_macro2::TokenStream) -> (Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
    use ScalarKind::*;
    let cast_ty = match kind {
        I8 => quote!{ i8 },
        U8 => quote!{ u8 },
        I16 => quote!{ i16 },
        U16 => quote!{ u16 },
        I32 => quote!{ i32 },
        U32 => quote!{ u32 },
        F32 => quote!{ f32 },
        F64 => quote!{ f64 },
    };
    scalar_match_and_cast_tokens_with_ty(&cast_ty, ply_rs)
}

fn scalar_match_and_cast_tokens_with_ty(cast_ty: &proc_macro2::TokenStream, ply_rs: &proc_macro2::TokenStream) -> (Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
    let arms = vec![
        quote!{ #ply_rs::ply::Property::Char(v) => Some(v as #cast_ty), },
        quote!{ #ply_rs::ply::Property::UChar(v) => Some(v as #cast_ty), },
        quote!{ #ply_rs::ply::Property::Short(v) => Some(v as #cast_ty), },
        quote!{ #ply_rs::ply::Property::UShort(v) => Some(v as #cast_ty), },
        quote!{ #ply_rs::ply::Property::Int(v) => Some(v as #cast_ty), },
        quote!{ #ply_rs::ply::Property::UInt(v) => Some(v as #cast_ty), },
        quote!{ #ply_rs::ply::Property::Float(v) => Some(v as #cast_ty), },
        quote!{ #ply_rs::ply::Property::Double(v) => Some(v as #cast_ty), },
    ];
    (arms, cast_ty.clone())
}

/// Generates match arms and casting logic for list properties.
fn list_match_and_cast_tokens(kind: &ScalarKind, ply_rs: &proc_macro2::TokenStream) -> (Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
    use ScalarKind::*;
    let cast_ty = match kind {
        I8 => quote!{ i8 },
        U8 => quote!{ u8 },
        I16 => quote!{ i16 },
        U16 => quote!{ u16 },
        I32 => quote!{ i32 },
        U32 => quote!{ u32 },
        F32 => quote!{ f32 },
        F64 => quote!{ f64 },
    };
    let arms = vec![
        quote!{ #ply_rs::ply::Property::ListChar(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ #ply_rs::ply::Property::ListUChar(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ #ply_rs::ply::Property::ListShort(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ #ply_rs::ply::Property::ListUShort(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ #ply_rs::ply::Property::ListInt(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ #ply_rs::ply::Property::ListUInt(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ #ply_rs::ply::Property::ListFloat(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ #ply_rs::ply::Property::ListDouble(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
    ];
    (arms, cast_ty)
}

/// Procedural macro to derive the `ToPly` trait.
///
/// This macro allows a struct to be written directly to a PLY file by mapping
/// `Vec<T>` fields to PLY elements.
#[proc_macro_derive(ToPly, attributes(ply))]
pub fn derive_to_ply(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "ToPly only supports named fields").to_compile_error()),
        },
        _ => return TokenStream::from(syn::Error::new_spanned(&input.ident, "ToPly only supports structs").to_compile_error()),
    };

    let mut element_defs = Vec::new();
    let mut payload_writes = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for field in fields {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let ply_name = match parse_ply_name(field) {
            Ok(name) => name,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };

        if !seen_names.insert(ply_name.clone()) {
            return TokenStream::from(syn::Error::new_spanned(field, format!("duplicate ply element name: {}", ply_name)).to_compile_error());
        }

        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());
        let inner_ty = match is_vec(field_type) {
            Some(ty) => ty,
            None => return TokenStream::from(syn::Error::new_spanned(field_type, "ToPly fields must be Vec<T>").to_compile_error()),
        };

        let ply_rs = get_crate_name();
        element_defs.push(quote! {
            {
                let mut element = #ply_rs::ply::ElementDef::new(#ply_name_lit.to_string());
                element.count = self.#field_name.len();
                let props = <#inner_ty as #ply_rs::ply::PropertyTypeSchema>::property_type_schema();
                for (name, ty) in props {
                    #ply_rs::ply::Addable::add(&mut element.properties, #ply_rs::ply::PropertyDef::new(name, ty));
                }
                #ply_rs::ply::Addable::add(&mut header.elements, element);
            }
        });

        payload_writes.push(quote! {
            {
                let element_def = header.elements.get(#ply_name_lit).expect("Element definition missing");
                let w = #ply_rs::writer::Writer::<#inner_ty>::new();
                written += w.write_payload_of_element(writer, &self.#field_name, element_def, &header)?;
            }
        });
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ply_rs = get_crate_name();
    let expanded = quote! {
        impl #impl_generics #ply_rs::writer::ToPly for #name #ty_generics #where_clause {
            fn write_ply<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
                let mut header = #ply_rs::ply::Header::new();
                header.encoding = #ply_rs::ply::Encoding::Ascii; // Defaulting to Ascii
                
                #( #element_defs )*
                
                let w = #ply_rs::writer::Writer::<#ply_rs::ply::DefaultElement>::new();
                let mut written = w.write_header(writer, &header)?;
                
                #( #payload_writes )*
                
                Ok(written)
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_property_type_tokens(ty: &Type, count_type: Option<&str>, explicit_type: Option<&str>, field_span: Option<&syn::Field>) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ply_rs = get_crate_name();

    let scalar_type_from_str = |s: &str| -> Option<proc_macro2::TokenStream> {
        match s {
            "char" | "i8" => Some(quote! { #ply_rs::ply::ScalarType::Char }),
            "uchar" | "u8" => Some(quote! { #ply_rs::ply::ScalarType::UChar }),
            "short" | "i16" => Some(quote! { #ply_rs::ply::ScalarType::Short }),
            "ushort" | "u16" => Some(quote! { #ply_rs::ply::ScalarType::UShort }),
            "int" | "i32" => Some(quote! { #ply_rs::ply::ScalarType::Int }),
            "uint" | "u32" => Some(quote! { #ply_rs::ply::ScalarType::UInt }),
            "float" | "f32" => Some(quote! { #ply_rs::ply::ScalarType::Float }),
            "double" | "f64" => Some(quote! { #ply_rs::ply::ScalarType::Double }),
            _ => None,
        }
    };

    if let Some(inner) = is_vec(ty) {
        let count_scalar_type = if let Some(ct) = count_type {
            scalar_type_from_str(ct).ok_or_else(|| {
                let span = field_span.map(|f| syn::spanned::Spanned::span(f)).unwrap_or_else(|| syn::spanned::Spanned::span(ty));
                syn::Error::new(span, format!("Unsupported count type: {}. Use one of: i8, u8, i16, u16, i32, u32, char, uchar, short, ushort, int, uint", ct))
            })?
        } else {
            quote! { #ply_rs::ply::ScalarType::UChar }
        };

        let elem_scalar_type = if let Some(et) = explicit_type {
            scalar_type_from_str(et).ok_or_else(|| {
                let span = field_span.map(|f| syn::spanned::Spanned::span(f)).unwrap_or_else(|| syn::spanned::Spanned::span(ty));
                syn::Error::new(span, format!("Unsupported explicit type: {}. Use one of: i8, u8, i16, u16, i32, u32, f32, f64, char, uchar, short, ushort, int, uint, float, double", et))
            })?
        } else if let Some(kind) = scalar_ident(inner) {
            let (scalar_type_token, _) = scalar_type_tokens(&kind, &ply_rs);
            scalar_type_token
        } else {
            return Err(syn::Error::new_spanned(inner, "Unsupported field type for PlyAccess. Supported types: i8, u8, i16, u16, i32, u32, f32, f64, and Vec<T> of these."));
        };

        return Ok(quote! {
            #ply_rs::ply::PropertyType::List(#count_scalar_type, #elem_scalar_type)
        });
    }

    if let Some(et) = explicit_type {
        let scalar_type_token = scalar_type_from_str(et).ok_or_else(|| {
            let span = field_span.map(|f| syn::spanned::Spanned::span(f)).unwrap_or_else(|| syn::spanned::Spanned::span(ty));
            syn::Error::new(span, format!("Unsupported explicit type: {}. Use one of: i8, u8, i16, u16, i32, u32, f32, f64, char, uchar, short, ushort, int, uint, float, double", et))
        })?;
        return Ok(quote! {
            #ply_rs::ply::PropertyType::Scalar(#scalar_type_token)
        });
    }

    if let Some(kind) = scalar_ident(ty) {
        let (scalar_type_token, _) = scalar_type_tokens(&kind, &ply_rs);
        return Ok(quote! {
            #ply_rs::ply::PropertyType::Scalar(#scalar_type_token)
        });
    }

    if is_option(ty).is_some() {
        return Err(syn::Error::new_spanned(ty, "optional properties are only supported by the reader"));
    }
    Err(syn::Error::new_spanned(ty, "Unsupported field type for PlyAccess. Supported types: i8, u8, i16, u16, i32, u32, f32, f64, and Vec<T> of these."))
}

fn scalar_type_tokens(kind: &ScalarKind, ply_rs: &proc_macro2::TokenStream) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    use ScalarKind::*;
    match kind {
        I8 => (quote!{ #ply_rs::ply::ScalarType::Char }, quote!{ i8 }),
        U8 => (quote!{ #ply_rs::ply::ScalarType::UChar }, quote!{ u8 }),
        I16 => (quote!{ #ply_rs::ply::ScalarType::Short }, quote!{ i16 }),
        U16 => (quote!{ #ply_rs::ply::ScalarType::UShort }, quote!{ u16 }),
        I32 => (quote!{ #ply_rs::ply::ScalarType::Int }, quote!{ i32 }),
        U32 => (quote!{ #ply_rs::ply::ScalarType::UInt }, quote!{ u32 }),
        F32 => (quote!{ #ply_rs::ply::ScalarType::Float }, quote!{ f32 }),
        F64 => (quote!{ #ply_rs::ply::ScalarType::Double }, quote!{ f64 }),
    }
}
