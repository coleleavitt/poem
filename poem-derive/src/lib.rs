//! Macros for poem

#![doc(html_favicon_url = "https://raw.githubusercontent.com/poem-web/poem/master/favicon.ico")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/poem-web/poem/master/logo.png")]
#![forbid(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod utils;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, GenericArgument, GenericParam, ItemFn, Member, PathArguments, Result, Type, parse_macro_input};

/// Wrap an asynchronous function as an `Endpoint`.
///
/// # Example
///
/// ```ignore
/// #[handler]
/// async fn example() {
/// }
/// ```
#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut internal = false;

    let arg_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("internal") {
            internal = true;
        }
        Ok(())
    });
    parse_macro_input!(args with arg_parser);

    match generate_handler(internal, input) {
        Ok(stream) => stream,
        Err(err) => err.into_compile_error().into(),
    }
}

fn generate_handler(internal: bool, input: TokenStream) -> Result<TokenStream> {
    let crate_name = utils::get_crate_name(internal);
    let item_fn = syn::parse::<ItemFn>(input)?;
    
    // Create a new generics with __S prepended to the existing generic params
    let mut combined_generics = item_fn.sig.generics.clone();
    combined_generics.params.insert(0, syn::parse_quote!(__S));
    let (combined_impl_generics, _, _) = combined_generics.split_for_impl();
    
    let (_, type_generics, _) = item_fn.sig.generics.split_for_impl();
    
    // Get just the where predicates without the 'where' keyword
    let where_predicates = item_fn.sig.generics.where_clause.as_ref().map(|wc| {
        let predicates = &wc.predicates;
        quote! { #predicates }
    }).unwrap_or_else(|| quote! {});
    
    let vis = &item_fn.vis;
    let docs = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect::<Vec<_>>();
    let ident = &item_fn.sig.ident;
    let call_await = if item_fn.sig.asyncness.is_some() {
        Some(quote::quote!(.await))
    } else {
        None
    };

    let def_struct = if !item_fn.sig.generics.params.is_empty() {
        let iter = item_fn
            .sig
            .generics
            .params
            .iter()
            .filter_map(|param| match param {
                GenericParam::Type(ty) => Some(ty),
                _ => None,
            })
            .enumerate()
            .map(|(idx, ty)| {
                let ident = format_ident!("_mark{}", idx);
                let ty_ident = &ty.ident;
                (ident, ty_ident)
            });

        let struct_members = iter.clone().map(|(ident, ty_ident)| {
            quote! { #ident: ::std::marker::PhantomData<#ty_ident> }
        });

        let default_members = iter.clone().map(|(ident, _ty_ident)| {
            quote! { #ident: ::std::marker::PhantomData }
        });

        quote! {
            #vis struct #ident #type_generics { #(#struct_members),*}
            impl #type_generics ::std::default::Default for #ident #type_generics {
                fn default() -> Self {
                    Self { #(#default_members),* }
                }
            }
        }
    } else {
        quote! { #vis struct #ident; }
    };

    let mut extractors = Vec::new();
    let mut args = Vec::new();
    let mut state_bounds = Vec::new();
    
    for (idx, input) in item_fn.sig.inputs.clone().into_iter().enumerate() {
        if let FnArg::Typed(pat) = input {
            let ty = &pat.ty;
            let id = quote::format_ident!("p{}", idx);
            args.push(id.clone());
            extractors.push(quote! {
                let #id = <#ty as #crate_name::FromRequest<'_, __S>>::from_request(&req, &mut body, state).await?;
            });
            
            // Check if this type is State<T> and extract T to add FromRef<__S> bound
            if let Some(inner_ty) = extract_state_inner_type(ty) {
                state_bounds.push(quote! {
                    #inner_ty: #crate_name::web::FromRef<__S>
                });
            }
        }
    }

    // Generate where clause bounds for State<T> extractors
    let state_bounds_clause = if state_bounds.is_empty() {
        quote! {}
    } else {
        quote! { #(#state_bounds,)* }
    };

    let expanded = quote! {
        #(#docs)*
        #[allow(non_camel_case_types)]
        #def_struct

        impl #combined_impl_generics #crate_name::Endpoint<__S> for #ident #type_generics
        where
            __S: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + 'static,
            #state_bounds_clause
            #where_predicates
        {
            type Output = #crate_name::Response;

            #[allow(unused_mut)]
            async fn call(&self, mut req: #crate_name::Request, state: &__S) -> #crate_name::Result<Self::Output> {
                let (req, mut body) = req.split();
                #(#extractors)*
                #item_fn
                let res = #ident(#(#args),*)#call_await;
                let res = #crate_name::error::IntoResult::into_result(res);
                std::result::Result::map(res, #crate_name::IntoResponse::into_response)
            }
        }
    };

    Ok(expanded.into())
}

/// Extracts the inner type `T` from a `State<T>` type.
/// Returns `Some(T)` if the type is `State<T>` or `web::State<T>` or similar,
/// otherwise returns `None`.
fn extract_state_inner_type(ty: &Type) -> Option<Type> {
    let path = match ty {
        Type::Path(path) => path,
        _ => return None,
    };
    
    // Check if the last segment is "State"
    let last_segment = path.path.segments.last()?;
    if last_segment.ident != "State" {
        return None;
    }
    
    // Extract the generic argument
    let args = match &last_segment.arguments {
        PathArguments::AngleBracketed(args) => args,
        _ => return None,
    };
    
    // Get the first type argument
    let first_arg = args.args.first()?;
    match first_arg {
        GenericArgument::Type(inner_ty) => Some(inner_ty.clone()),
        _ => None,
    }
}

#[doc(hidden)]
#[proc_macro]
pub fn generate_implement_middlewares(_: TokenStream) -> TokenStream {
    let mut impls = Vec::new();

    for i in 2..=16 {
        let idents = (0..i)
            .map(|i| format_ident!("T{}", i + 1))
            .collect::<Vec<_>>();
        let output_type = idents.last().unwrap();
        let first_ident = idents.first().unwrap();
        let mut where_clauses = vec![quote! { #first_ident: Middleware<E, S> }];
        let mut transforms = Vec::new();

        for k in 1..i {
            let prev_ident = &idents[k - 1];
            let current_ident = &idents[k];
            where_clauses.push(quote! { #current_ident: Middleware<#prev_ident::Output, S> });
        }

        for k in 0..i {
            let n = Member::from(k);
            transforms.push(quote! { let ep = self.#n.transform(ep); });
        }

        let expanded = quote! {
            impl<E, S, #(#idents),*> Middleware<E, S> for (#(#idents),*)
                where
                    E: Endpoint<S>,
                    S: Send + Sync,
                    #(#where_clauses,)*
            {
                type Output = #output_type::Output;

                fn transform(&self, ep: E) -> Self::Output {
                    #(#transforms)*
                    ep
                }
            }
        };

        impls.push(expanded);
    }

    quote!(#(#impls)*).into()
}
