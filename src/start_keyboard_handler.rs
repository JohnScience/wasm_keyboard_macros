use std::marker::PhantomData;

use crate::EVENTS;
use quote::quote;
use strum::EnumCount;
use syn::{bracketed, parse::Parse, punctuated::Punctuated, token::FatArrow, Expr, Token};
use uievents_code::KeyboardEventCode;

use super::EVENT_COUNT;

pub(super) struct Args {
    pub(super) instance_name: syn::Ident,
    pub(super) ty_name: syn::Ident,
    pub(super) target: syn::Ident,
    pub(super) key_paths: Vec<syn::Path>,
    pub(super) key_handlers_exprs: Vec<Expr>,
}

struct KeyHandler {
    key_path: syn::Path,
    // Fat arrow is not used in the struct but it is used in the parse function.
    fat_arrow: PhantomData<FatArrow>,
    key_handler_expr: Expr,
}

impl Parse for KeyHandler {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key_path = input.parse()?;
        let fat_arrow = match input.parse::<Token![=>]>() {
            Ok(_fat_arrow) => PhantomData,
            Err(e) => return Err(e),
        };
        let expr = input.parse()?;

        Ok(KeyHandler {
            key_path,
            fat_arrow,
            key_handler_expr: expr,
        })
    }
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let instance_name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty_name = input.parse()?;
        input.parse::<Token![,]>()?;
        let target = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        bracketed!(content in input);
        let mut key_paths = Vec::<syn::Path>::with_capacity(KeyboardEventCode::COUNT);
        let mut key_handlers_exprs = Vec::<Expr>::with_capacity(KeyboardEventCode::COUNT);
        for KeyHandler {
            key_path,
            fat_arrow: _,
            key_handler_expr,
        } in Punctuated::<KeyHandler, Token![,]>::parse_terminated(&content)?
        {
            key_paths.push(key_path);
            key_handlers_exprs.push(key_handler_expr);
        }
        match key_paths.len() {
            0 => Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "No key handlers were provided.",
            )),
            _ => Ok(()),
        }?;
        Ok(Args {
            instance_name,
            ty_name,
            target,
            key_paths,
            key_handlers_exprs,
        })
    }
}

impl Args {
    fn ith_field(i: usize) -> syn::Ident {
        quote::format_ident!("key_handler{}", i)
    }

    fn fields(&self) -> impl Iterator<Item = syn::Ident> + Clone {
        (0..self.key_paths.len()).map(Args::ith_field)
    }

    /// Extend the token stream with the [items] that are needed for the keyboard handler,
    /// such as the struct definition and the impl.
    ///
    /// [items]: https://doc.rust-lang.org/reference/items.html
    fn extend_with_items(&self, ts: &mut proc_macro2::TokenStream) {
        let &Self {
            ref ty_name,
            ref key_paths,
            ..
        } = self;

        let generics = (0..key_paths.len()).map(|i| quote::format_ident!("T{}", i));
        let [generics_clone0, generics_clone1, generics_clone2, generics_clone3] =
            std::array::from_fn(|_i| generics.clone());
        let field_ty_pairs = (0..key_paths.len()).map(|i| {
            let field = Args::ith_field(i);
            let ty = quote::format_ident!("T{}", i);
            quote!(#field: #ty)
        });
        let fields = self.fields();

        let method_decls: [proc_macro2::TokenStream; EVENT_COUNT] = EVENTS.map(|event| {
            let keyboard_handler_impl_method = quote::format_ident!("inner_handle_{event}");
            let key_handler_impl_method = quote::format_ident!("handle_{event}");
            let fields = fields.clone();
            // TODO: eventually optimize match with alternatives
            quote!(
                fn #keyboard_handler_impl_method(&self, event: &::web_sys::KeyboardEvent) {
                    match event.code().as_str() {
                        #(#key_paths => self.#fields.#key_handler_impl_method()),*,
                        _ => (),
                    }
                }
            )
        });

        ts.extend(quote!(
        struct #ty_name<#(#generics),*>
        where
            #(#generics_clone0: ::wasm_keyboard::KeyHandler),*
        {
            #(#field_ty_pairs),*
        }

        impl<#(#generics_clone1),*> #ty_name<#(#generics_clone2),*>
        where
            #(#generics_clone3: ::wasm_keyboard::KeyHandler),*
        {
            #(#method_decls)*
        }));
    }

    fn extend_with_var_binding(&self, ts: &mut proc_macro2::TokenStream) {
        let &Self {
            ref instance_name,
            ref ty_name,
            ref key_handlers_exprs,
            ..
        } = self;

        let fields = self.fields();

        ts.extend(quote!(
            let #instance_name = ::std::rc::Rc::new(#ty_name {
                #( #fields: #key_handlers_exprs ),*
            });
        ))
    }

    fn extend_with_code_adding_listeners(&self, ts: &mut proc_macro2::TokenStream) {
        let &Self {
            ref instance_name,
            ref target,
            ..
        } = self;

        for block in EVENTS.map(|event| {
            let keyboard_handler_impl_method = quote::format_ident!("inner_handle_{event}");
            quote!(
                {
                    let #instance_name = #instance_name.clone();
                    let __handler = ::wasm_bindgen::closure::Closure::<dyn ::core::ops::FnMut(_)>::new::<_>(
                        move |event: KeyboardEvent| #instance_name.#keyboard_handler_impl_method(&event),
                    );
                    #target
                        .add_event_listener_with_callback(
                            #event,
                            ::wasm_bindgen::JsCast::unchecked_ref(__handler.as_ref()),
                        )
                        .unwrap();
                    ::wasm_bindgen::closure::Closure::forget(__handler);
                }
            )
        }) {
            ts.extend(block);
        }
    }

    pub(super) fn extend_with_startup_code(&self, ts: &mut proc_macro2::TokenStream) {
        self.extend_with_items(ts);
        self.extend_with_var_binding(ts);
        self.extend_with_code_adding_listeners(ts);
    }
}
