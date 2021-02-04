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
use syn::{parse_macro_input, Error, ItemTrait};

use crate::facet_crate_name;

pub fn facet(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = parse_macro_input!(attr as syn::parse::Nothing);
    let facet = parse_macro_input!(item as ItemTrait);

    match gen_attribute(facet) {
        Ok(output) => output,
        Err(e) => e.to_compile_error(),
    }
    .into()
}

fn gen_attribute(facet: ItemTrait) -> Result<TokenStream, Error> {
    let facet_crate = format_ident!("{}", facet_crate_name());
    let vis = &facet.vis;
    let name = &facet.ident;
    let snake_name = snakify_pascal_case(name.to_string());
    let trait_ref_name = format_ident!("{}Ref", name);
    let trait_ref_method = format_ident!("{}", snake_name, span = name.span());
    let trait_arc_name = format_ident!("{}Arc", name);
    let trait_arc_method = format_ident!("{}_arc", snake_name, span = name.span());
    let arc_trait_name = format_ident!("Arc{}", name);
    let send_sync = quote!(::std::marker::Send + ::std::marker::Sync);

    Ok(quote! {
        #facet

        #vis trait #trait_ref_name {
            fn #trait_ref_method(&self) -> &(dyn #name + #send_sync);
        }

        impl<T: ::#facet_crate::FacetRef<dyn #name + #send_sync>> #trait_ref_name for T {
            #[inline]
            fn #trait_ref_method(&self) -> &(dyn #name + #send_sync) {
                self.facet_ref()
            }
        }

        #vis trait #trait_arc_name: #trait_ref_name {
            fn #trait_arc_method(&self) -> ::std::sync::Arc<dyn #name + #send_sync>;
        }

        impl<T: ::#facet_crate::FacetArc<dyn #name + #send_sync> + ::#facet_crate::FacetRef<dyn #name + #send_sync>> #trait_arc_name for T {
            #[inline]
            fn #trait_arc_method(&self) -> ::std::sync::Arc<dyn #name + #send_sync> {
                self.facet_arc()
            }
        }

        #vis type #arc_trait_name = ::std::sync::Arc<dyn #name + #send_sync>;
    })
}

/// Converts a Pascal case name like `SomeTraitName` to snake case like
/// `some_trait_name`.
fn snakify_pascal_case(pascal: impl AsRef<str>) -> String {
    let mut snake = String::new();
    for ch in pascal.as_ref().chars() {
        if ch.is_uppercase() {
            if !snake.is_empty() {
                snake.push('_');
            }
            snake.extend(ch.to_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}
