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

/// Procedural macro to derive the `PropertyAccess` trait.
///
/// This macro generates the `set_property` method, which maps PLY property names
/// to struct fields and handles type conversions.
///
/// Supported attributes:
/// - `#[ply(name = "property_name")]`: Maps the field to a specific PLY property name.
#[proc_macro_derive(PropertyAccess, attributes(ply))]
pub fn derive_property_access(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("PropertyAccess only supports named fields"),
        },
        _ => panic!("PropertyAccess only supports structs"),
    };

    let mut set_arms = Vec::new();

    for field in fields {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let mut ply_name = field_name.as_ref().unwrap().to_string();

        for attr in &field.attrs {
            if attr.path().is_ident("ply") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        ply_name = s.value();
                    }
                    Ok(())
                });
            }
        }

        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());

        let is_opt = is_option(field_type);
        
        let conversion = if let Some(inner_type) = is_opt.as_ref() {
             generate_conversion(inner_type)
        } else {
             generate_conversion(field_type)
        };

        let arm = if is_opt.is_some() {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        self.#field_name = Some(val);
                    }
                }
            }
        } else {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        self.#field_name = val;
                    }
                }
            }
        };
        set_arms.push(arm);

    }

    let expanded = quote! {
        impl ply_rs_bw::ply::PropertyAccess for #name where #name: Default {
            fn new() -> Self {
                Default::default()
            }

            fn set_property(&mut self, key: &str, property: ply_rs_bw::ply::Property) {
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
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("PropertySchema only supports named fields"),
        },
        _ => panic!("PropertySchema only supports structs"),
    };

    let mut schema_entries = Vec::new();

    for field in fields {
        let field_name = &field.ident;
        let mut ply_name = field_name.as_ref().unwrap().to_string();

        for attr in &field.attrs {
            if attr.path().is_ident("ply") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        ply_name = s.value();
                    }
                    Ok(())
                });
            }
        }

        let requiredness = if is_option(&field.ty).is_some() {
            quote! { ply_rs_bw::ply::Requiredness::Optional }
        } else {
            quote! { ply_rs_bw::ply::Requiredness::Required }
        };

        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());
        schema_entries.push(quote! {
            (#ply_name_lit.to_string(), #requiredness)
        });
    }

    let expanded = quote! {
        impl ply_rs_bw::ply::PropertySchema for #name {
            fn schema() -> Vec<(String, ply_rs_bw::ply::Requiredness)> {
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
#[proc_macro_derive(PlyAccess, attributes(ply))]
pub fn derive_ply_access(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Reuse logic from PropertyAccess derive
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("PlyAccess only supports named fields"),
        },
        _ => panic!("PlyAccess only supports structs"),
    };

    let mut set_arms = Vec::new();
    let mut schema_entries = Vec::new();
    let mut type_schema_entries = Vec::new();

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
        let mut ply_name = field_name.as_ref().unwrap().to_string();

        for attr in &field.attrs {
            if attr.path().is_ident("ply") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        ply_name = s.value();
                    }
                    Ok(())
                });
            }
        }

        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());

        let is_opt = is_option(field_type);
        let conversion_type = if let Some(inner) = is_opt.as_ref() { inner } else { field_type };
        let conversion = generate_conversion(conversion_type);

        let arm = if is_opt.is_some() {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        self.#field_name = Some(val);
                    }
                }
            }
        } else {
            quote! {
                #ply_name_lit => {
                    if let Some(val) = #conversion {
                        self.#field_name = val;
                    }
                }
            }
        };
        set_arms.push(arm);

        let requiredness = if is_opt.is_some() {
            quote! { ply_rs_bw::ply::Requiredness::Optional }
        } else {
            quote! { ply_rs_bw::ply::Requiredness::Required }
        };
        schema_entries.push(quote! {
            (#ply_name_lit.to_string(), #requiredness)
        });

        let prop_type_token = get_property_type_tokens(conversion_type);
        type_schema_entries.push(quote! {
             (#ply_name_lit.to_string(), #prop_type_token)
        });

        // Getter logic
        let field_access_scalar = if is_opt.is_some() {
             quote! { self.#field_name }
        } else {
             quote! { Some(self.#field_name) }
        };
        
        let field_access_list = if is_opt.is_some() {
             quote! { self.#field_name.as_deref() }
        } else {
             quote! { Some(self.#field_name.as_slice()) }
        };

        if let Some(inner) = is_vec(conversion_type) {
             // List type
             if let Some(kind) = scalar_ident(inner) {
                 use ScalarKind::*;
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
        } else if let Some(kind) = scalar_ident(conversion_type) {
             // Scalar type
             use ScalarKind::*;
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

    let expanded = quote! {
        impl ply_rs_bw::ply::PropertyAccess for #name where #name: Default {
            fn new() -> Self { Default::default() }
            fn set_property(&mut self, key: &str, property: ply_rs_bw::ply::Property) {
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
        impl ply_rs_bw::ply::PropertySchema for #name {
            fn schema() -> Vec<(String, ply_rs_bw::ply::Requiredness)> {
                vec![ #( #schema_entries ),* ]
            }
        }
        impl ply_rs_bw::ply::PropertyTypeSchema for #name {
            fn property_type_schema() -> Vec<(String, ply_rs_bw::ply::PropertyType)> {
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
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("FromPly only supports named fields"),
        },
        _ => panic!("FromPly only supports structs"),
    };

    let mut field_names = Vec::new();
    let mut inner_tys = Vec::new();
    let mut ply_names = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let mut ply_name = field_name.to_string();

        let inner_ty = is_vec(field_type).expect("FromPly currently only supports Vec<T> fields");

        for attr in &field.attrs {
            if attr.path().is_ident("ply") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        ply_name = s.value();
                    }
                    Ok(())
                });
            }
        }

        field_names.push(field_name);
        inner_tys.push(inner_ty);
        ply_names.push(ply_name);
    }

    let expanded = quote! {
        impl ply_rs_bw::parser::FromPly for #name {
            fn read_ply<T: std::io::Read>(reader: &mut T) -> std::io::Result<Self> {
                let mut reader = std::io::BufReader::new(reader);
                // We need a parser to read the header. Any element type will do.
                let parser = ply_rs_bw::parser::Parser::<ply_rs_bw::ply::DefaultElement>::new();
                let header = parser.read_header(&mut reader)?;

                #(
                    let mut #field_names = Vec::new();
                )*

                for (name, element_def) in &header.elements {
                    match name.as_str() {
                        #(
                            #ply_names => {
                                let p = ply_rs_bw::parser::Parser::<#inner_tys>::new();
                                #field_names = p.read_payload_for_element(&mut reader, element_def, &header)?;
                            }
                        )*
                        _ => {
                             // skip unknown elements
                             let p = ply_rs_bw::parser::Parser::<ply_rs_bw::ply::DefaultElement>::new();
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
fn generate_conversion(ty: &Type) -> proc_macro2::TokenStream {
    // Recognize scalars and Vec<scalar>
    if let Some(inner) = is_vec(ty) {
        let elem = scalar_ident(inner);
        if let Some(elem_ty) = elem {
            let (list_variants, _cast) = list_match_and_cast_tokens(&elem_ty);
            return quote! {
                match property {
                    #(#list_variants)*
                    _ => None,
                }
            };
        }
    }

    if let Some(s) = scalar_ident(ty) {
        let (scalar_variants, _cast) = scalar_match_and_cast_tokens(&s);
        return quote! {
            match property {
                #(#scalar_variants)*
                _ => None,
            }
        };
    }

    // Fallback: not recognized
    quote! { None }
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
fn scalar_match_and_cast_tokens(kind: &ScalarKind) -> (Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
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
        quote!{ ply_rs_bw::ply::Property::Char(v) => Some(v as #cast_ty), },
        quote!{ ply_rs_bw::ply::Property::UChar(v) => Some(v as #cast_ty), },
        quote!{ ply_rs_bw::ply::Property::Short(v) => Some(v as #cast_ty), },
        quote!{ ply_rs_bw::ply::Property::UShort(v) => Some(v as #cast_ty), },
        quote!{ ply_rs_bw::ply::Property::Int(v) => Some(v as #cast_ty), },
        quote!{ ply_rs_bw::ply::Property::UInt(v) => Some(v as #cast_ty), },
        quote!{ ply_rs_bw::ply::Property::Float(v) => Some(v as #cast_ty), },
        quote!{ ply_rs_bw::ply::Property::Double(v) => Some(v as #cast_ty), },
    ];
    (arms, cast_ty)
}

/// Generates match arms and casting logic for list properties.
fn list_match_and_cast_tokens(kind: &ScalarKind) -> (Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
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
        quote!{ ply_rs_bw::ply::Property::ListChar(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ ply_rs_bw::ply::Property::ListUChar(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ ply_rs_bw::ply::Property::ListShort(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ ply_rs_bw::ply::Property::ListUShort(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ ply_rs_bw::ply::Property::ListInt(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ ply_rs_bw::ply::Property::ListUInt(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ ply_rs_bw::ply::Property::ListFloat(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
        quote!{ ply_rs_bw::ply::Property::ListDouble(v) => Some(v.into_iter().map(|x| x as #cast_ty).collect()), },
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
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("ToPly only supports named fields"),
        },
        _ => panic!("ToPly only supports structs"),
    };

    let mut element_defs = Vec::new();
    let mut payload_writes = Vec::new();

    for field in fields {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let mut ply_name = field_name.as_ref().unwrap().to_string();

        for attr in &field.attrs {
            if attr.path().is_ident("ply") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        if let Ok(value) = meta.value() {
                            if let Ok(s) = value.parse::<syn::LitStr>() {
                                ply_name = s.value();
                            }
                        }
                    }
                    Ok(())
                });
            }
        }
        
        let ply_name_lit = syn::LitStr::new(&ply_name, proc_macro2::Span::call_site());
        let inner_ty = is_vec(field_type).expect("ToPly fields must be Vec<T>");

        element_defs.push(quote! {
            {
                let mut element = ply_rs_bw::ply::ElementDef::new(#ply_name_lit.to_string());
                element.count = self.#field_name.len();
                let props = <#inner_ty as ply_rs_bw::ply::PropertyTypeSchema>::property_type_schema();
                for (name, ty) in props {
                    ply_rs_bw::ply::Addable::add(&mut element.properties, ply_rs_bw::ply::PropertyDef::new(name, ty));
                }
                ply_rs_bw::ply::Addable::add(&mut header.elements, element);
            }
        });

        payload_writes.push(quote! {
            {
                let element_def = header.elements.get(#ply_name_lit).expect("Element definition missing");
                let w = ply_rs_bw::writer::Writer::<#inner_ty>::new();
                w.write_payload_of_element(writer, &self.#field_name, element_def, &header)?;
            }
        });
    }

    let expanded = quote! {
        impl ply_rs_bw::writer::ToPly for #name {
            fn write_ply<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
                let mut header = ply_rs_bw::ply::Header::new();
                header.encoding = ply_rs_bw::ply::Encoding::Ascii; // Defaulting to Ascii
                
                #( #element_defs )*
                
                let w = ply_rs_bw::writer::Writer::<ply_rs_bw::ply::DefaultElement>::new();
                let mut written = w.write_header(writer, &header)?;
                
                #( #payload_writes )*
                
                Ok(written)
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_property_type_tokens(ty: &Type) -> proc_macro2::TokenStream {
    if let Some(inner) = is_vec(ty) {
         if let Some(kind) = scalar_ident(inner) {
             let (scalar_type_token, _) = scalar_type_tokens(&kind);
             return quote! {
                 ply_rs_bw::ply::PropertyType::List(ply_rs_bw::ply::ScalarType::UChar, #scalar_type_token)
             };
         }
    }
    if let Some(kind) = scalar_ident(ty) {
         let (scalar_type_token, _) = scalar_type_tokens(&kind);
         return quote! {
             ply_rs_bw::ply::PropertyType::Scalar(#scalar_type_token)
         };
    }
    if let Some(inner) = is_option(ty) {
        return get_property_type_tokens(inner);
    }
    quote! { ply_rs_bw::ply::PropertyType::Scalar(ply_rs_bw::ply::ScalarType::Int) }
}

fn scalar_type_tokens(kind: &ScalarKind) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    use ScalarKind::*;
    match kind {
        I8 => (quote!{ ply_rs_bw::ply::ScalarType::Char }, quote!{ i8 }),
        U8 => (quote!{ ply_rs_bw::ply::ScalarType::UChar }, quote!{ u8 }),
        I16 => (quote!{ ply_rs_bw::ply::ScalarType::Short }, quote!{ i16 }),
        U16 => (quote!{ ply_rs_bw::ply::ScalarType::UShort }, quote!{ u16 }),
        I32 => (quote!{ ply_rs_bw::ply::ScalarType::Int }, quote!{ i32 }),
        U32 => (quote!{ ply_rs_bw::ply::ScalarType::UInt }, quote!{ u32 }),
        F32 => (quote!{ ply_rs_bw::ply::ScalarType::Float }, quote!{ f32 }),
        F64 => (quote!{ ply_rs_bw::ply::ScalarType::Double }, quote!{ f64 }),
    }
}
