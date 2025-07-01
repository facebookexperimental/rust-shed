/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Error;
use syn::FnArg;
use syn::GenericArgument;
use syn::Ident;
use syn::ImplItem;
use syn::ItemImpl;
use syn::Pat;
use syn::PatType;
use syn::PathArguments;
use syn::ReturnType;
use syn::Signature;
use syn::Token;
use syn::Type;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;

use crate::facet_crate_name;
use crate::util::Asyncness;
use crate::util::Fallibility;

pub fn factory(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let params = parse_macro_input!(attr as Params);
    let factory = parse_macro_input!(item as ItemImpl);

    gen_factory(params, factory)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn gen_factory(params: Params, mut factory_impl: ItemImpl) -> Result<TokenStream, Error> {
    let factory_ty = extract_type_ident(&factory_impl.self_ty)?;

    let facets = Facets::extract_from_impl(&params, &mut factory_impl)?;

    let factory_builder = gen_factory_builder(&params, &factory_ty, &facets)?;

    Ok(quote! {
        #factory_impl

        #factory_builder
    })
}

fn gen_factory_builder(
    params: &Params,
    factory_ty: &Ident,
    facets: &Facets,
) -> Result<TokenStream, Error> {
    let facet_idents = &facets.facet_idents;
    let facet_params = &facets.facet_params;

    let facet_crate = format_ident!("{}", facet_crate_name());
    let builder_ident = format_ident!("{}Builder", factory_ty);

    let is_async = Asyncness::any(facets.facet_asyncnesses.iter());

    let facet_params_map = facet_idents
        .iter()
        .zip(facet_params)
        .collect::<BTreeMap<_, _>>();

    for facet_ident in facet_idents {
        check_no_cycles(facet_ident, &facet_params_map)?;
    }

    let builder = match is_async {
        Asyncness::Synchronous => {
            gen_sync_factory_builder(&facet_crate, factory_ty, &builder_ident, params, facets)?
        }
        Asyncness::Asynchronous => {
            gen_async_factory_builder(&facet_crate, factory_ty, &builder_ident, params, facets)?
        }
    };

    Ok(builder)
}

fn gen_sync_factory_builder(
    facet_crate: &Ident,
    factory_ty: &Ident,
    builder_ident: &Ident,
    params: &Params,
    facets: &Facets,
) -> Result<TokenStream, Error> {
    let builder_facets_ident = format_ident!("{}BuilderFacets", factory_ty);
    let param_idents = &params.param_idents;
    let param_types = &params.param_types;
    let facet_idents = &facets.facet_idents;
    let facet_types = &facets.facet_types;
    let facet_types_map = facet_idents
        .iter()
        .zip(facet_types)
        .collect::<BTreeMap<_, _>>();

    let mut builder_impls = Vec::new();

    for (facet_ident, facet_type, fallibility, asyncness, facet_params) in facets.iter() {
        let mut call_params = Vec::new();
        let mut make_facets = Vec::new();

        for facet_param in facet_params {
            match facet_param {
                FactoryParam::Facet(ident) => {
                    let param_type = facet_types_map
                        .get(ident)
                        .ok_or_else(|| Error::new(ident.span(), "unrecognised facet name"))?;
                    make_facets.push(quote! {
                        let #ident: #param_type = self.build()?;
                    });
                    call_params.push(quote!(&#ident));
                }
                FactoryParam::Param(ident) => {
                    call_params.push(quote!(&self.facets.#ident));
                }
            }
        }

        if asyncness == Asyncness::Asynchronous {
            panic!("should not generate sync builder for async factory");
        }
        let maybe_map_err = fallibility.maybe(quote! {
            .map_err(|e| ::#facet_crate::FactoryError::FacetBuildFailed {
                name: stringify!(#facet_ident),
                source: e.into(),
            })?
        });

        builder_impls.push(quote! {

            impl ::#facet_crate::Builder<#facet_type> for #builder_ident<'_> {

                fn build<'builder>(&'builder mut self) -> ::std::result::Result<
                    #facet_type,
                    ::#facet_crate::FactoryError,
                >  {
                    if let Some(facet) = self.facets.#facet_ident.as_ref() {
                        return Ok(facet.clone());
                    }
                    use ::#facet_crate::Builder as _;
                    #( #make_facets )*
                    let #facet_ident =
                        self.factory.#facet_ident( #( #call_params ),* )
                            #maybe_map_err;
                    debug_assert!(self.facets.#facet_ident.is_none());
                    self.facets.#facet_ident = Some(#facet_ident.clone());
                    Ok(#facet_ident)
                }
            }

        })
    }

    let builder = quote! {
        #[doc(hidden)]
        pub struct #builder_facets_ident {
            #(
                #param_idents: #param_types,
            )*
            #(
                #facet_idents: ::std::option::Option<#facet_types>,
            )*
        }

        impl #builder_facets_ident {
            #[doc(hidden)]
            pub fn new( #( #param_idents: #param_types, )* ) -> Self {
                Self {
                    #( #param_idents, )*
                    #(
                        #facet_idents: ::std::default::Default::default(),
                    )*
                }
            }
        }

        #(
            #builder_impls
        )*

        #[doc(hidden)]
        pub struct #builder_ident<'factory> {
            factory: &'factory #factory_ty,
            facets: #builder_facets_ident,
        }

        impl #factory_ty {
            /// Build an instance of a container from this factory.
            pub fn build<'factory, T>(
                &'factory self,
                #( #param_idents: #param_types ),*
            ) -> ::std::result::Result<T, ::#facet_crate::FactoryError>
            where
                T: ::#facet_crate::Buildable<#builder_ident<'factory>>,
            {
                let mut builder = #builder_ident {
                    factory: &self,
                    facets: #builder_facets_ident::new(#( #param_idents, )*),
                };
                T::build(&mut builder)
            }
        }
    };

    Ok(builder)
}

fn gen_async_factory_builder(
    facet_crate: &Ident,
    factory_ty: &Ident,
    builder_ident: &Ident,
    params: &Params,
    facets: &Facets,
) -> Result<TokenStream, Error> {
    let builder_facets_ident = format_ident!("{}BuilderFacets", factory_ty);
    let builder_facets_needed_ident = format_ident!("{}BuilderFacetsNeeded", factory_ty);
    let builder_params_ident = format_ident!("{}BuilderParams", factory_ty);

    let param_idents = &params.param_idents;
    let param_types = &params.param_types;
    let facet_idents = &facets.facet_idents;
    let facet_types = &facets.facet_types;
    let facet_types_map = facet_idents
        .iter()
        .zip(facet_types)
        .collect::<BTreeMap<_, _>>();

    let mut heads: BTreeSet<_> = facet_idents.iter().collect();
    let mut facet_build_futs = BTreeMap::new();
    let mut facet_build_graph = BTreeMap::new();
    let mut builder_impls = Vec::new();
    let mut build_facets = Vec::new();
    let mut store_facets = Vec::new();

    for (facet_ident, facet_type, fallibility, asyncness, facet_params) in facets.iter() {
        let mut dependent_facets = Vec::new();
        let mut mark_facets_needed = Vec::new();
        let mut call_params = Vec::new();
        let mut deps = Vec::new();

        for facet_param in facet_params {
            match facet_param {
                FactoryParam::Facet(ident) => {
                    let param_type = facet_types_map
                        .get(ident)
                        .ok_or_else(|| Error::new(ident.span(), "unrecognised facet name"))?;
                    mark_facets_needed.push(quote! {
                        ::#facet_crate::AsyncBuilderFor::<#param_type>::need(self);
                    });
                    dependent_facets.push(ident);
                    call_params.push(quote!(#ident.as_ref().unwrap()));
                    heads.remove(&ident);
                    deps.push(ident);
                }
                FactoryParam::Param(ident) => {
                    call_params.push(quote!(&__self_params.#ident));
                }
            }
        }

        let maybe_dot_await_factory = asyncness.maybe(quote!(.await));
        let maybe_map_err = fallibility.maybe(quote! {
            .map_err(|e| ::#facet_crate::AsyncFactoryError::from(
                ::#facet_crate::FactoryError::FacetBuildFailed {
                    name: stringify!(#facet_ident),
                    source: e.into(),
                }))?
        });

        facet_build_graph.insert(facet_ident, deps);
        builder_impls.push(quote! {

            impl ::#facet_crate::AsyncBuilderFor<#facet_type> for #builder_ident<'_> {

                fn need(&mut self) {
                    self.needed.#facet_ident = true;
                    #( #mark_facets_needed )*
                }

                fn get(&self) -> #facet_type {
                    // The proc macro should have arranged for all needed
                    // facets to have been marked as needed and thus built. It
                    // is invalid for this to be called if the facet wasn't
                    // built.
                    self.facets.#facet_ident.clone().expect(
                        concat!(
                            "bug in #[facet::factory]: facet '",
                            stringify!(#facet_ident),
                            "' was not marked as needed",
                        )
                    )
                }
            }

        });

        let get_dependent_facets = if dependent_facets.is_empty() {
            quote!()
        } else {
            quote! {
                let ( #( #dependent_facets, )* ) =
                    ::#facet_crate::futures::try_join!(
                        #( #dependent_facets.clone(), )*
                    )?;
            }
        };

        facet_build_futs.insert(
            facet_ident,
            quote! {
                let #facet_ident = async {
                    if __self_needed.#facet_ident {
                        #get_dependent_facets
                        Ok::<_, ::#facet_crate::AsyncFactoryError>(Some(
                            __self_factory.#facet_ident( #( #call_params, )* )
                                #maybe_dot_await_factory
                                #maybe_map_err
                        ))
                    } else {
                        Ok::<_, ::#facet_crate::AsyncFactoryError>(None)
                    }
                }.shared();
            },
        );

        store_facets.push(quote! {
            __self_facets.#facet_ident = #facet_ident;
        });
    }

    // Group facets into based on their depth from the heads of the dependency
    // graph.  This will be used to order construction of the facets in
    // topological order.
    let mut ident_depths = BTreeMap::new();
    let mut queue: VecDeque<_> = heads.into_iter().map(|head| (head, 0)).collect();
    let mut max_depth = 0;
    while let Some((ident, depth)) = queue.pop_front() {
        ident_depths.insert(ident, depth);
        max_depth = depth;
        for dep in facet_build_graph.get(&ident).unwrap().iter() {
            queue.push_back((dep, depth + 1));
        }
    }
    let mut levels = vec![vec![]; max_depth + 1];
    for (ident, depth) in ident_depths.into_iter() {
        levels[depth].push(ident);
    }

    for idents in levels.into_iter().rev() {
        for ident in idents.iter() {
            build_facets.push(facet_build_futs.remove(ident).unwrap());
        }
    }

    let builder = quote! {
        #[doc(hidden)]
        pub struct #builder_params_ident {
            #(
                #param_idents: #param_types,
            )*
        }

        #[doc(hidden)]
        #[derive(Default)]
        pub struct #builder_facets_ident {
            #(
                #facet_idents: ::std::option::Option<#facet_types>,
            )*
        }

        #[doc(hidden)]
        #[derive(Default)]
        pub struct #builder_facets_needed_ident {
            #(
                #facet_idents: bool,
            )*
        }

        impl #builder_params_ident {
            #[doc(hidden)]
            pub fn new( #( #param_idents: #param_types, )* ) -> Self {
                Self {
                    #( #param_idents, )*
                }
            }
        }

        #[::#facet_crate::async_trait::async_trait]
        impl ::#facet_crate::AsyncBuilder for #builder_ident<'_> {
            async fn build_needed(
                &mut self
            ) -> ::std::result::Result<(), ::#facet_crate::FactoryError> {
                use ::#facet_crate::futures::future::FutureExt;
                let __self_facets = &mut self.facets;
                let __self_needed = &self.needed;
                let __self_params = &self.params;
                let __self_factory = self.factory;
                #( #build_facets )*
                let ( #( #facet_idents, )* ) =
                    ::#facet_crate::futures::try_join!( #( #facet_idents.clone(), )* )
                    .map_err(|e| e.factory_error())?;
                #( #store_facets )*
                Ok(())
            }
        }

        #(
            #builder_impls
        )*

        #[doc(hidden)]
        pub struct #builder_ident<'factory> {
            factory: &'factory #factory_ty,
            params: #builder_params_ident,
            facets: #builder_facets_ident,
            needed: #builder_facets_needed_ident,
        }

        impl #factory_ty {
            /// Build an instance of a container from this factory.
            pub async fn build<'factory, 'builder, T>(
                &'factory self,
                #( #param_idents: #param_types ),*
            ) -> ::std::result::Result<T, ::#facet_crate::FactoryError>
            where
                T: ::#facet_crate::AsyncBuildable<'builder, #builder_ident<'factory>>,
            {
                let builder = #builder_ident {
                    factory: &self,
                    params: #builder_params_ident::new(#( #param_idents, )*),
                    facets: #builder_facets_ident::default(),
                    needed: #builder_facets_needed_ident::default(),
                };
                T::build_async(builder).await
            }
        }
    };

    Ok(builder)
}

#[derive(Debug)]
struct Params {
    param_idents: Vec<Ident>,
    param_types: Vec<Type>,
}

impl Parse for Params {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut param_idents = Vec::new();
        let mut param_types = Vec::new();
        for arg in Punctuated::<FnArg, Token![,]>::parse_terminated(input)? {
            match arg {
                FnArg::Typed(pat_type) => match *pat_type.pat {
                    Pat::Ident(pat_ident) => {
                        param_idents.push(pat_ident.ident);
                        param_types.push(*pat_type.ty);
                    }
                    _ => return Err(Error::new_spanned(&pat_type.pat, "expected 'ident: Type'")),
                },
                FnArg::Receiver(r) => {
                    return Err(Error::new_spanned(
                        r,
                        "receivers not supported in factory parameters",
                    ));
                }
            }
        }
        Ok(Params {
            param_idents,
            param_types,
        })
    }
}

struct Facets {
    facet_idents: Vec<Ident>,
    facet_types: Vec<Type>,
    facet_fallibilities: Vec<Fallibility>,
    facet_asyncnesses: Vec<Asyncness>,
    facet_params: Vec<Vec<FactoryParam>>,
}

impl Facets {
    fn iter(
        &self,
    ) -> impl Iterator<Item = (&Ident, &Type, Fallibility, Asyncness, &[FactoryParam])> {
        self.facet_idents
            .iter()
            .zip(self.facet_types.iter())
            .zip(self.facet_fallibilities.iter())
            .zip(self.facet_asyncnesses.iter())
            .zip(self.facet_params.iter())
            .map(|((((ident, ty), fall), asy), params)| (ident, ty, *fall, *asy, params.as_slice()))
    }

    fn extract_from_impl(params: &Params, factory: &mut ItemImpl) -> Result<Self, Error> {
        let mut facet_idents = Vec::new();
        let mut facet_types = Vec::new();
        let mut facet_fallibilities = Vec::new();
        let mut facet_asyncnesses = Vec::new();
        let mut facet_params = Vec::new();
        for item in &mut factory.items {
            if let ImplItem::Fn(method) = item {
                let method_params = Self::extract_facet_params(params, &method.sig)?;
                let (facet_ty, fallibility) = Self::extract_facet_return_type(&mut method.sig)?;
                facet_idents.push(method.sig.ident.clone());
                facet_types.push(facet_ty);
                facet_fallibilities.push(fallibility);
                facet_asyncnesses.push(method.sig.asyncness.into());
                facet_params.push(method_params);
            }
        }
        Ok(Facets {
            facet_idents,
            facet_types,
            facet_fallibilities,
            facet_asyncnesses,
            facet_params,
        })
    }

    fn extract_facet_params(params: &Params, sig: &Signature) -> Result<Vec<FactoryParam>, Error> {
        let mut method_params = Vec::new();
        for input in &sig.inputs {
            match input {
                FnArg::Receiver(_) => {}
                FnArg::Typed(pat_type) => {
                    method_params.push(FactoryParam::parse(params, pat_type)?);
                }
            }
        }
        Ok(method_params)
    }

    fn extract_facet_return_type(sig: &mut Signature) -> Result<(Type, Fallibility), Error> {
        if let ReturnType::Type(_, ty) = &mut sig.output {
            if let Type::Path(type_path) = &mut **ty {
                if let Some(segment) = type_path.path.segments.last_mut() {
                    match &mut segment.arguments {
                        PathArguments::None => {
                            // The type path should be directly to the facet.
                            let facet_ty = (**ty).clone();
                            return Ok((facet_ty, Fallibility::Infallible));
                        }
                        PathArguments::AngleBracketed(arguments) => {
                            if let Some(GenericArgument::Type(first_ty)) =
                                arguments.args.first_mut()
                            {
                                // This type should be directly to the facet.
                                let facet_ty = first_ty.clone();
                                return Ok((facet_ty, Fallibility::Fallible));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(Error::new_spanned(
            sig,
            concat!(
                "invalid return type ",
                "(note: factory methods must return either an ArcFacet alias or ",
                "a Result<ArcFacet, _>)",
            ),
        ))
    }
}

#[derive(Debug)]
enum FactoryParam {
    Param(Ident),
    Facet(Ident),
}

impl FactoryParam {
    fn parse(params: &Params, pat_type: &PatType) -> Result<Self, Error> {
        let ident = match &*pat_type.pat {
            Pat::Ident(pat_ident) => strip_leading_underscore(&pat_ident.ident),
            _ => return Err(Error::new_spanned(pat_type, "expected 'ident: Type'")),
        };
        match &*pat_type.ty {
            Type::Reference(_) => {
                if params.param_idents.contains(&ident) {
                    Ok(FactoryParam::Param(ident))
                } else {
                    Ok(FactoryParam::Facet(ident))
                }
            }
            _ => Err(Error::new_spanned(
                pat_type,
                concat!(
                    "factory methods must take a reference to a factory parameter ",
                    "or a reference to a facet"
                ),
            )),
        }
    }
}

fn extract_type_ident(ty: &Type) -> Result<Ident, Error> {
    if let Type::Path(type_path) = ty {
        if let Some(ident) = type_path.path.get_ident() {
            return Ok(ident.clone());
        }
    }
    Err(Error::new_spanned(
        ty,
        "facet::factory impl must be for a local concrete type",
    ))
}

fn strip_leading_underscore(ident: &Ident) -> Ident {
    let ident_string = ident.to_string();
    match ident_string.strip_prefix('_') {
        Some(stripped) => Ident::new(stripped, ident.span()),
        None => ident.clone(),
    }
}

fn check_no_cycles(
    top_ident: &Ident,
    ident_map: &BTreeMap<&Ident, &Vec<FactoryParam>>,
) -> Result<(), Error> {
    // A map from seen idents to a vector of the route to them from top_ident.
    let mut seen = BTreeMap::new();
    // A queue of idents to expand and the routes to them so far.
    let mut queue = VecDeque::new();
    queue.push_back((top_ident, vec![]));
    while let Some((ident, route)) = queue.pop_front() {
        if let Some(params) = ident_map.get(&ident) {
            for param in *params {
                if let FactoryParam::Facet(param_ident) = param {
                    seen.entry(param_ident).or_insert_with(|| {
                        let mut param_route = route.clone();
                        param_route.push(param_ident);
                        queue.push_back((param_ident, param_route));
                        route.clone()
                    });
                }
            }
        }
    }
    if let Some(route) = seen.get(&top_ident) {
        let via = if route.is_empty() {
            String::from("directly")
        } else {
            let route = route.iter().map(ToString::to_string).collect::<Vec<_>>();
            format!("via {}", route.join(" -> "))
        };
        return Err(Error::new(
            top_ident.span(),
            format!("cyclic facet dependency: {top_ident} depends on itself {via}"),
        ));
    }
    Ok(())
}
