/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Error, Expr, Fields, Ident, ItemStruct, Type};

use crate::facet_crate_name;

#[derive(Debug)]
struct ContainerMembers {
    field_idents: Vec<Ident>,
    field_inits: Vec<Expr>,
    facet_idents: Vec<Ident>,
    facet_types: Vec<Type>,
}

impl ContainerMembers {
    fn extract(container: &mut ItemStruct) -> Result<Self, Error> {
        let mut field_idents = Vec::new();
        let mut field_inits = Vec::new();
        let mut facet_idents = Vec::new();
        let mut facet_types = Vec::new();
        let send_sync = quote!(::std::marker::Send + ::std::marker::Sync);
        match &mut container.fields {
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
                                        "of 'init' or 'facet', found multiple"
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
                                        "of 'init' or 'facet', found multiple"
                                    ),
                                ));
                            }
                            attr_found = true;
                            let facet_type = field.ty.clone();
                            field.ty = syn::parse2(
                                quote!(::std::sync::Arc<dyn #facet_type + #send_sync>),
                            )?;
                            facet_idents
                                .push(field.ident.clone().expect("named field must have a name"));
                            facet_types.push(facet_type);
                        } else {
                            new_attrs.push(attr);
                        }
                    }
                    if !attr_found {
                        return Err(Error::new(
                            field.span(),
                            concat!(
                                "facet::container field must have exactly one ",
                                "of 'init' or 'attr', found neither"
                            ),
                        ));
                    }
                    field.attrs = new_attrs;
                }
            }
            _ => {
                return Err(Error::new(
                    container.ident.span(),
                    "facet::container requires a struct with named fields",
                ));
            }
        }

        Ok(ContainerMembers {
            field_idents,
            field_inits,
            facet_idents,
            facet_types,
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
    let facet_idents = &members.facet_idents;
    let facet_types = &members.facet_types;

    let attr_impls = gen_attr_impls(&facet_crate, container_name, facet_idents, facet_types)?;

    let buildable_impl = gen_buildable_impl(&facet_crate, &container_name, &members)?;
    let async_buildable_impl = gen_async_buildable_impl(&facet_crate, &container_name, &members)?;

    Ok(quote! {
        #container

        #( #attr_impls )*

        #buildable_impl

        #async_buildable_impl
    })
}

fn gen_buildable_impl(
    facet_crate: &Ident,
    container_name: &Ident,
    members: &ContainerMembers,
) -> Result<TokenStream, Error> {
    let facet_idents = &members.facet_idents;
    let facet_types = &members.facet_types;
    let field_idents = &members.field_idents;
    let field_inits = &members.field_inits;

    let send_sync = quote!(::std::marker::Send + ::std::marker::Sync);

    Ok(quote! {
        impl<B> ::#facet_crate::Buildable<B> for #container_name
        where B: #send_sync
            #( + ::#facet_crate::Builder<::std::sync::Arc<dyn #facet_types + #send_sync>> )*
        {
           fn build(mut builder: B) -> ::std::result::Result<Self, ::#facet_crate::FactoryError> {

                // Build each facet.
                #(
                    let #facet_idents =
                        <B as ::#facet_crate::Builder<
                            ::std::sync::Arc<dyn #facet_types + #send_sync>
                        >>::build(&mut builder)?;
                )*

                // Initialize the other fields.
                #(
                    let #field_idents = #field_inits;
                )*

                Ok(Self {
                    #( #field_idents, )*
                    #( #facet_idents, )*
                })
           }
        }
    })
}

fn gen_async_buildable_impl(
    facet_crate: &Ident,
    container_name: &Ident,
    members: &ContainerMembers,
) -> Result<TokenStream, Error> {
    let facet_idents = &members.facet_idents;
    let facet_types = &members.facet_types;
    let field_idents = &members.field_idents;
    let field_inits = &members.field_inits;

    let send_sync = quote!(::std::marker::Send + ::std::marker::Sync);

    // Desugared async-trait so that the builder lifetime can be specified.
    Ok(quote! {
        impl<'builder, B> ::#facet_crate::AsyncBuildable<'builder, B> for #container_name
        where B: ::std::marker::Send + ::std::marker::Sync + ::#facet_crate::AsyncBuilder
            #( + ::#facet_crate::AsyncBuilderFor<::std::sync::Arc<dyn #facet_types + #send_sync>> )*
            + 'builder
        {
            fn build_async(mut builder: B) -> ::std::pin::Pin<::std::boxed::Box<
                dyn std::future::Future<
                    Output = ::std::result::Result<Self, ::#facet_crate::FactoryError>
                > + ::std::marker::Send + 'builder
            >>
            {
                let build = async move {
                    // Mark facets we need as as needed.
                    #(
                        <B as ::#facet_crate::AsyncBuilderFor<
                            ::std::sync::Arc<dyn #facet_types + #send_sync>
                        >>::need(&mut builder);
                    )*

                    // Build the needed facets.
                    <B as ::#facet_crate::AsyncBuilder>::build_needed(&mut builder).await?;

                    // Get the facets out of the builder.
                    #(
                        let #facet_idents =
                            <B as ::#facet_crate::AsyncBuilderFor<
                                ::std::sync::Arc<dyn #facet_types + #send_sync>
                            >>::get(&builder);
                    )*

                    // Initialize other fields.
                    #(
                        let #field_idents = #field_inits;
                    )*

                    Ok(Self {
                        #( #field_idents, )*
                        #( #facet_idents, )*
                    })
                };
                ::std::boxed::Box::pin(build)
           }
        }
    })
}

fn gen_attr_impls(
    facet_crate: &Ident,
    container_name: &Ident,
    facet_idents: &[Ident],
    facet_types: &[Type],
) -> Result<Vec<TokenStream>, Error> {
    let mut output = Vec::new();

    let send_sync = quote!(::std::marker::Send + ::std::marker::Sync);

    for (facet_ident, facet_type) in facet_idents.iter().zip(facet_types) {
        output.push(quote! {
            impl ::#facet_crate::FacetRef<dyn #facet_type + #send_sync> for #container_name {
                #[inline]
                fn facet_ref(&self) -> &(dyn #facet_type + #send_sync + 'static)
                {
                    self.#facet_ident.as_ref()
                }
            }

            impl ::#facet_crate::FacetRef<dyn #facet_type + #send_sync> for &#container_name {
                #[inline]
                fn facet_ref(&self) -> &(dyn #facet_type + #send_sync + 'static)
                {
                    (*self).#facet_ident.as_ref()
                }
            }

            impl ::#facet_crate::FacetArc<dyn #facet_type + #send_sync> for #container_name {
                #[inline]
                fn facet_arc(&self) -> ::std::sync::Arc<dyn #facet_type + #send_sync + 'static>
                {
                    self.#facet_ident.clone()
                }
            }

            impl ::#facet_crate::FacetArc<dyn #facet_type + #send_sync> for &#container_name {
                #[inline]
                fn facet_arc(&self) -> ::std::sync::Arc<dyn #facet_type + #send_sync + 'static>
                {
                    (*self).#facet_ident.clone()
                }
            }
        });
    }
    Ok(output)
}
