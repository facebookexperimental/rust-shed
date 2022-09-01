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
use syn::parse_macro_input;
use syn::spanned::Spanned;
use syn::Error;
use syn::Item;

use crate::facet_crate_name;
use crate::snakify_pascal_case;

pub fn facet(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = parse_macro_input!(attr as syn::parse::Nothing);
    let facet = parse_macro_input!(item as Item);

    match gen_attribute(facet) {
        Ok(output) => output,
        Err(e) => e.to_compile_error(),
    }
    .into()
}

fn gen_attribute(facet: Item) -> Result<TokenStream, Error> {
    let vis;
    let name;
    let facet_ty;

    match &facet {
        Item::Trait(facet) => {
            vis = &facet.vis;
            name = &facet.ident;
            facet_ty = quote!(dyn #name + ::std::marker::Send + ::std::marker::Sync + 'static);
        }
        Item::Struct(facet) => {
            vis = &facet.vis;
            name = &facet.ident;
            facet_ty = quote!(#name);
        }
        Item::Enum(facet) => {
            vis = &facet.vis;
            name = &facet.ident;
            facet_ty = quote!(#name);
        }
        _ => return Err(Error::new(facet.span(), "expected trait, struct or enum")),
    }

    let facet_crate = format_ident!("{}", facet_crate_name());
    let snake_name = snakify_pascal_case(name.to_string());
    let trait_ref_name = format_ident!("{}Ref", name);
    let trait_ref_method = format_ident!("{}", snake_name, span = name.span());
    let trait_arc_name = format_ident!("{}Arc", name);
    let trait_arc_method = format_ident!("{}_arc", snake_name, span = name.span());
    let arc_trait_name = format_ident!("Arc{}", name);

    Ok(quote! {
        #facet

        /// Access #name by reference from a facet container.
        #vis trait #trait_ref_name {
            /// Access #name by reference from a facet container.
            fn #trait_ref_method(&self) -> &(#facet_ty);
        }

        impl<T: ::#facet_crate::FacetRef<#facet_ty>> #trait_ref_name for T {
            #[inline]
            fn #trait_ref_method(&self) -> &(#facet_ty) {
                self.facet_ref()
            }
        }

        /// Access a cloneable reference to #name from a facet container.
        #vis trait #trait_arc_name: #trait_ref_name {
            /// Access a cloneable reference to #name from a facet container.
            fn #trait_arc_method(&self) -> ::std::sync::Arc<#facet_ty>;
        }

        impl<T: ::#facet_crate::FacetArc<#facet_ty> + ::#facet_crate::FacetRef<#facet_ty>> #trait_arc_name for T {
            #[inline]
            fn #trait_arc_method(&self) -> ::std::sync::Arc<#facet_ty> {
                self.facet_arc()
            }
        }

        /// Cloneable container for #name.
        #vis type #arc_trait_name = ::std::sync::Arc<#facet_ty>;
    })
}
