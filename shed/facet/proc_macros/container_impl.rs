/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::ToTokens;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Attribute;
use syn::Error;
use syn::Expr;
use syn::Fields;
use syn::Ident;
use syn::Index;
use syn::ItemStruct;
use syn::Token;
use syn::Type;
use syn::TypeParamBound;

use crate::facet_crate_name;
use crate::snakify_pascal_case;

#[derive(Debug, Copy, Clone)]
enum ContainerType {
    Named,
    Unnamed,
}

impl ContainerType {
    fn gen_init(&self, body: TokenStream) -> TokenStream {
        match self {
            ContainerType::Named => {
                quote! {
                    Self {
                        #body
                    }
                }
            }
            ContainerType::Unnamed => {
                quote! {
                    Self( #body )
                }
            }
        }
    }
}

#[derive(Debug)]
struct ContainerMembers {
    container_type: ContainerType,
    field_idents: Vec<Ident>,
    field_inits: Vec<Expr>,
    facet_idents: Vec<Ident>,
    facet_types: Vec<Type>,
    delegate_idents: Vec<Ident>,
    delegate_types: Vec<Type>,
    delegate_facets: Vec<Vec<Type>>,
}

impl ContainerMembers {
    fn extract(container: &mut ItemStruct) -> Result<Self, Error> {
        let mut field_idents = Vec::new();
        let mut field_inits = Vec::new();
        let mut facet_idents = Vec::new();
        let mut facet_types = Vec::new();
        let mut delegate_idents = Vec::new();
        let mut delegate_types = Vec::new();
        let mut delegate_facets = Vec::new();
        let container_type = match &mut container.fields {
            Fields::Named(named_fields) => {
                for field in named_fields.named.iter_mut() {
                    let mut attr_found = false;
                    let mut new_attrs = Vec::new();
                    for attr in field.attrs.drain(..) {
                        if attr.path.is_ident("init") {
                            if attr_found {
                                return Err(Error::new(
                                    attr.span(),
                                    concat!(
                                        "facet::container field must have exactly one ",
                                        "of 'init', 'facet' or 'delegate', found multiple"
                                    ),
                                ));
                            }
                            attr_found = true;
                            let expr: Expr = attr.parse_args()?;
                            field_idents
                                .push(field.ident.clone().expect("named field must have a name"));
                            field_inits.push(expr);
                        } else if attr.path.is_ident("facet") {
                            if attr_found {
                                return Err(Error::new(
                                    attr.span(),
                                    concat!(
                                        "facet::container field must have exactly one ",
                                        "of 'init', 'facet' or 'delegate', found multiple"
                                    ),
                                ));
                            }
                            attr_found = true;
                            let mut facet_type = field.ty.clone();
                            if let Type::TraitObject(obj) = &mut facet_type {
                                obj.bounds.push(syn::parse2(quote!(::std::marker::Send))?);
                                obj.bounds.push(syn::parse2(quote!(::std::marker::Sync))?);
                                obj.bounds.push(syn::parse2(quote!('static))?);
                            }
                            field.ty = syn::parse2(quote!(::std::sync::Arc<#facet_type>))?;
                            facet_idents
                                .push(field.ident.clone().expect("named field must have a name"));
                            facet_types.push(facet_type);
                        } else if attr.path.is_ident("delegate") {
                            if attr_found {
                                return Err(Error::new(
                                    attr.span(),
                                    concat!(
                                        "facet::container field must have exactly one ",
                                        "of 'init', 'facet' or 'delegate', found multiple"
                                    ),
                                ));
                            }
                            attr_found = true;
                            let delegate_type = field.ty.clone();
                            let facets = extract_delegate_facets(&attr)?;
                            delegate_idents
                                .push(field.ident.clone().expect("named field must have a name"));
                            delegate_types.push(delegate_type);
                            delegate_facets.push(facets);
                        } else {
                            new_attrs.push(attr);
                        }
                    }
                    if !attr_found {
                        return Err(Error::new(
                            field.span(),
                            concat!(
                                "facet::container field must have exactly one ",
                                "of 'init', 'facet' or 'delegate', found none"
                            ),
                        ));
                    }
                    field.attrs = new_attrs;
                }
                ContainerType::Named
            }
            Fields::Unnamed(unnamed_fields) => {
                for (index, field) in unnamed_fields.unnamed.iter_mut().enumerate() {
                    let mut facet_type = field.ty.clone();
                    if let Type::TraitObject(obj) = &mut facet_type {
                        obj.bounds.push(syn::parse2(quote!(::std::marker::Send))?);
                        obj.bounds.push(syn::parse2(quote!(::std::marker::Sync))?);
                        obj.bounds.push(syn::parse2(quote!('static))?);
                    }
                    field.ty = syn::parse2(quote!(::std::sync::Arc<#facet_type>))?;
                    facet_idents.push(format_ident!("_field{}", index));
                    facet_types.push(facet_type);
                }
                ContainerType::Unnamed
            }
            _ => {
                return Err(Error::new(
                    container.ident.span(),
                    "facet::container requires a struct with named fields",
                ));
            }
        };

        Ok(ContainerMembers {
            container_type,
            field_idents,
            field_inits,
            facet_idents,
            facet_types,
            delegate_idents,
            delegate_types,
            delegate_facets,
        })
    }
}

pub fn container(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = parse_macro_input!(attr as syn::parse::Nothing);
    let container = parse_macro_input!(item as ItemStruct);

    match gen_container(container) {
        Ok(output) => output,
        Err(e) => e.to_compile_error(),
    }
    .into()
}

fn gen_container(mut container: ItemStruct) -> Result<TokenStream, Error> {
    let facet_crate = format_ident!("{}", facet_crate_name());
    let members = ContainerMembers::extract(&mut container)?;
    let container_name = &container.ident;

    let attr_impls = gen_attr_impls(&facet_crate, container_name, &members);
    let buildable_impl = gen_buildable_impl(&facet_crate, container_name, &members);
    let async_buildable_impl = gen_async_buildable_impl(&facet_crate, container_name, &members);
    let from_impl = gen_from_impl(container_name, &members);

    Ok(quote! {
        #container

        #( #attr_impls )*

        #buildable_impl

        #async_buildable_impl

        #from_impl
    })
}

fn gen_buildable_impl(
    facet_crate: &Ident,
    container_name: &Ident,
    members: &ContainerMembers,
) -> TokenStream {
    let facet_idents = &members.facet_idents;
    let facet_types = &members.facet_types;
    let field_idents = &members.field_idents;
    let field_inits = &members.field_inits;
    let delegate_idents = &members.delegate_idents;
    let delegate_types = &members.delegate_types;

    let container_init = members.container_type.gen_init(quote! {
        #( #delegate_idents, )*
        #( #field_idents, )*
        #( #facet_idents, )*
    });

    quote! {
        impl<B> ::#facet_crate::Buildable<B> for #container_name
        where B: ::std::marker::Send + ::std::marker::Sync
            #( + ::#facet_crate::Builder<::std::sync::Arc<#facet_types>> )*,
            #( #delegate_types: ::#facet_crate::Buildable<B>, )*
        {
           fn build(builder: &mut B) -> ::std::result::Result<Self, ::#facet_crate::FactoryError> {

                // Build each delegate.
                #(
                    let #delegate_idents =
                        <#delegate_types as ::#facet_crate::Buildable<B>>::build(builder)?;
                )*

                // Build each facet.
                #(
                    let #facet_idents =
                        <B as ::#facet_crate::Builder<
                            ::std::sync::Arc<#facet_types>
                        >>::build(builder)?;
                )*

                // Initialize the other fields.
                #(
                    let #field_idents = #field_inits;
                )*

                Ok(#container_init)
           }
        }
    }
}

fn gen_async_buildable_impl(
    facet_crate: &Ident,
    container_name: &Ident,
    members: &ContainerMembers,
) -> TokenStream {
    let facet_idents = &members.facet_idents;
    let facet_types = &members.facet_types;
    let field_idents = &members.field_idents;
    let field_inits = &members.field_inits;
    let delegate_idents = &members.delegate_idents;
    let delegate_types = &members.delegate_types;

    let container_init = members.container_type.gen_init(quote! {
            #( #delegate_idents, )*
            #( #field_idents, )*
            #( #facet_idents, )*
    });
    // Desugared async-trait so that the builder lifetime can be specified.
    quote! {
        impl<'builder, B> ::#facet_crate::AsyncBuildable<'builder, B> for #container_name
        where B: ::std::marker::Send + ::std::marker::Sync + ::#facet_crate::AsyncBuilder
            #( + ::#facet_crate::AsyncBuilderFor<::std::sync::Arc<#facet_types>> )*
            + 'builder,
            #( #delegate_types: ::#facet_crate::AsyncBuildable<'builder, B>, )*
        {
            fn build_async(mut builder: B) -> ::std::pin::Pin<::std::boxed::Box<
                dyn std::future::Future<
                    Output = ::std::result::Result<Self, ::#facet_crate::FactoryError>
                > + ::std::marker::Send + 'builder
            >>
            {
                let build = async move {
                    // Mark needed facets as needed.
                    Self::mark_needed(&mut builder);

                    // Build the needed facets.
                    <B as ::#facet_crate::AsyncBuilder>::build_needed(&mut builder).await?;

                    // Build ourself.
                    Ok(Self::construct(&builder))

                };
                ::std::boxed::Box::pin(build)
           }

           fn mark_needed(builder: &mut B) {
                // Mark facets we need as as needed.
                #(
                    <B as ::#facet_crate::AsyncBuilderFor<
                        ::std::sync::Arc<#facet_types>
                    >>::need(builder);
                )*

                // Mark facets our delegates need as needed.
                #(
                    <#delegate_types as ::#facet_crate::AsyncBuildable<'builder, B>>
                        ::mark_needed(builder);
                )*
           }

            fn construct(builder: &B) -> Self {
                // Build delegates.
                #(
                    let #delegate_idents =
                        <#delegate_types as ::#facet_crate::AsyncBuildable<'builder, B>>
                            ::construct(builder);
                )*

                // Get the facets out of the builder.
                #(
                    let #facet_idents =
                        <B as ::#facet_crate::AsyncBuilderFor<
                            ::std::sync::Arc<#facet_types>
                        >>::get(builder);
                )*

                // Initialize other fields.
                #(
                    let #field_idents = #field_inits;
                )*

                #container_init
            }
        }

    }
}

fn gen_attr_impls(
    facet_crate: &Ident,
    container_name: &Ident,
    members: &ContainerMembers,
) -> Vec<TokenStream> {
    let mut output = Vec::new();
    let facet_idents = &members.facet_idents;
    let facet_types = &members.facet_types;
    let delegate_idents = &members.delegate_idents;
    let delegate_facets = &members.delegate_facets;

    for (index, (facet_ident, facet_type)) in facet_idents.iter().zip(facet_types).enumerate() {
        let field = match members.container_type {
            ContainerType::Named => {
                quote!(#facet_ident)
            }
            ContainerType::Unnamed => {
                let index = Index::from(index);
                quote!(#index)
            }
        };
        output.push(quote! {
            impl ::#facet_crate::FacetRef<#facet_type> for #container_name {
                #[inline]
                fn facet_ref(&self) -> &(#facet_type)
                {
                    self.#field.as_ref()
                }
            }

            impl ::#facet_crate::FacetRef<#facet_type> for &#container_name {
                #[inline]
                fn facet_ref(&self) -> &(#facet_type)
                {
                    (*self).#field.as_ref()
                }
            }

            impl ::#facet_crate::FacetArc<#facet_type> for #container_name {
                #[inline]
                fn facet_arc(&self) -> ::std::sync::Arc<#facet_type>
                {
                    self.#field.clone()
                }
            }

            impl ::#facet_crate::FacetArc<#facet_type> for &#container_name {
                #[inline]
                fn facet_arc(&self) -> ::std::sync::Arc<#facet_type>
                {
                    (*self).#field.clone()
                }
            }

        });
    }

    for (delegate_ident, delegate_facet) in delegate_idents.iter().zip(delegate_facets) {
        output.push(quote! {
            #(
                impl ::#facet_crate::FacetRef<#delegate_facet> for #container_name {
                    #[inline]
                    fn facet_ref(&self) -> &(#delegate_facet) {
                        self.#delegate_ident.facet_ref()
                    }
                }

                impl ::#facet_crate::FacetRef<#delegate_facet> for &#container_name {
                    #[inline]
                    fn facet_ref(&self) -> &(#delegate_facet) {
                        self.#delegate_ident.facet_ref()
                    }
                }

                impl ::#facet_crate::FacetArc<#delegate_facet> for #container_name {
                    #[inline]
                    fn facet_arc(&self) -> ::std::sync::Arc<#delegate_facet> {
                        self.#delegate_ident.facet_arc()
                    }
                }

                impl ::#facet_crate::FacetArc<#delegate_facet> for &#container_name {
                    #[inline]
                    fn facet_arc(&self) -> ::std::sync::Arc<#delegate_facet> {
                        self.#delegate_ident.facet_arc()
                    }
                }
            )*
        });
    }

    output
}

fn gen_from_impl(container_name: &Ident, members: &ContainerMembers) -> Option<TokenStream> {
    if members.field_idents.is_empty()
        && members.delegate_idents.is_empty()
        && !members.facet_idents.is_empty()
    {
        let facet_idents = &members.facet_idents;
        let facet_types = members
            .facet_types
            .iter()
            .map(|ty| {
                // This is janky. For each type, we turn it to text that's hopefuly the right type,
                // then parse that back into an ident
                let token_stream = match ty {
                    Type::TraitObject(obj) => obj
                        .bounds
                        .iter()
                        .filter_map(|bound| {
                            if let TypeParamBound::Trait(bound) = bound {
                                Some(bound.path.clone())
                            } else {
                                None
                            }
                        })
                        .next()?
                        .to_token_stream(),
                    _ => ty.to_token_stream(),
                };
                syn::parse2(token_stream).ok()
            })
            .collect::<Option<Vec<Ident>>>()?;

        let part_params = facet_idents
            .iter()
            .zip(members.facet_types.iter())
            .map(|(ident, ty)| quote! {#ident: ::std::sync::Arc<#ty>});
        let snake_name = snakify_pascal_case(container_name.to_string());

        let macro_name = format_ident!("{}_from_container", snake_name);
        let facet_copies = facet_types.iter().map(|facet_type| {
            let facet_type = snakify_pascal_case(facet_type.to_string());
            let facet_type = format_ident!("{}_arc", facet_type);
            quote! {
                $other.#facet_type()
            }
        });

        let container_init = members.container_type.gen_init(quote! {
            #(#facet_idents),*
        });

        Some(quote! {
            impl #container_name {
                pub fn from_parts(#(#part_params),*) -> Self {
                    #container_init
                }
            }
            #[allow(unused)]
            macro_rules! #macro_name {
                ($other:expr) => {
                    #container_name::from_parts(
                        #( #facet_copies ),*
                    )
                }
            }
        })
    } else {
        None
    }
}

fn extract_delegate_facets(attr: &Attribute) -> Result<Vec<Type>, Error> {
    let mut facets = Vec::new();
    let args: Punctuated<Type, Token![,]> = attr.parse_args_with(Punctuated::parse_terminated)?;
    for mut arg in args {
        if let Type::TraitObject(obj) = &mut arg {
            obj.bounds.push(syn::parse2(quote!(::std::marker::Send))?);
            obj.bounds.push(syn::parse2(quote!(::std::marker::Sync))?);
            obj.bounds.push(syn::parse2(quote!('static))?);
        }
        facets.push(arg);
    }
    Ok(facets)
}
