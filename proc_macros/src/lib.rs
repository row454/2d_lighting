use std::{any::type_name, str::FromStr};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree};
use syn::{parse_macro_input, Data, DeriveInput, Lit, ExprLit, TypeArray};
use quote::{quote, TokenStreamExt};

// impl Vertex {
//     const ATTRIBS: [wgpu::VertexAttribute; 2] =
//         wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

//     fn desc() -> wgpu::VertexBufferLayout<'static> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &Self::ATTRIBS,
//         }
//     }
// }
#[proc_macro_derive(Vertex, attributes(normalized))]
pub fn derive_vertex(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = input.ident;
    if let Data::Struct(data_struct) = input.data {
        let fields: Vec<_> = data_struct.fields.into_iter().collect();
        let len = fields.len();
        let mut attribs_inner = proc_macro2::TokenStream::new();
        for (i,field) in fields.into_iter().enumerate() {
            let mut type_name = String::new(); //String::from_str("wgpu::VertexFormat::").unwrap();
            match field.ty {
                syn::Type::Array(TypeArray {
                    elem,
                    len,
                    ..

                }) => {
                    
                    match *elem {
                        syn::Type::Path(path) => {
                            let ident = &path.path.segments.first().unwrap().ident;
                            type_name.push_str(
                                if field.attrs.iter().any(|attr| attr.path().is_ident("normalized")) {
                                    match ident.to_string().as_str() {
                                        "f32" | "f64," => panic!("floats can not be normalized!"),
                                        "u8" => "Unorm8",
                                        "i8" => "Snorm8",
                                        "u16" => "Unorm16",
                                        "i16" => "Snorm16",
                                        "u32" => "Unorm32",
                                        "i32" => "Snorm32",
                                        _ => panic!("invalid type identifier!")
                                    }
                                } else {
                                    match ident.to_string().as_str() {
                                        "f32" => "Float32",
                                        "f64" => "Float64",
                                        "u8" => "Uint8",
                                        "i8" => "Sint8",
                                        "u16" => "Uint16",
                                        "i16" => "Sint16",
                                        "u32" => "Uint32",
                                        "i32" => "Sint32",
                                        _ => panic!("invalid type identifier!")
                                    }
                            });
                                    
                        },
                        a => panic!("type not allowed!"),
                    }
                    match len {
                        syn::Expr::Lit(ExprLit {lit: Lit::Int(len), ..}) => {
                            type_name.push('x');
                            type_name.push_str(len.base10_digits());
                        },
                        _ => panic!("length must be an integer literal!")
                    }
                    
                },
                syn::Type::Path(path) => {
                    let ident = &path.path.segments.first().unwrap().ident;
                    type_name.push_str(match ident.to_string().as_str() {
                        "f32" => "Float32",
                        "f64" => "Float64",
                        "i32" => "Sint32",
                        "u32" => "Uint32",
                        _ => panic!("invalid type identifier!")
                    });
                },
                _ => panic!("all fields must be arrays or numbers!")
            }
            let ty: proc_macro2::TokenStream = type_name.parse().unwrap();
            let i = i as u32;
            attribs_inner.append_all(quote! {
                #i => #ty,
            })
            
        }
        
        let attribs = quote!{wgpu::vertex_attr_array![#attribs_inner]};
        quote!{
            impl Vertex for #ident {
                type Attribs = [wgpu::VertexAttribute; #len];
                const ATTRIBS: Self::Attribs = #attribs;
                fn desc() -> wgpu::VertexBufferLayout<'static> {
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &Self::ATTRIBS,
                    }
                }
            }
        }.into()
    } else {
        panic!("only valid on structs!")
    }
}