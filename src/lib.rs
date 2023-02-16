use proc_macro::TokenStream;
use quote::quote;

mod decl_keyboard_handler;
mod new_key_handler;

use decl_keyboard_handler::KeyHandlersSoA;
use syn::{parse_macro_input, TypeInfer, Token};

#[proc_macro]
pub fn decl_keyboard_handler(input: TokenStream) -> TokenStream {
    let KeyHandlersSoA {
        instance_name,
        ty_name,
        key_paths,
        key_handlers_exprs,
    } = parse_macro_input!(input as KeyHandlersSoA);
    let generics = (0..key_paths.len()).map(|i| quote::format_ident!("T{}", i));
    let [generics_clone0, generics_clone1, generics_clone2, generics_clone3] =
        std::array::from_fn(|_i| generics.clone());
    let field_ty_pairs = (0..key_paths.len()).map(|i| {
        let field = quote::format_ident!("key_handler{}", i);
        let ty = quote::format_ident!("T{}", i);
        quote!(#field: #ty)
    });
    let fields = (0..key_paths.len()).map(|i| quote::format_ident!("key_handler{}", i));
    // Iterators have to be cloned because they are consumed by the quote macro.
    // https://stackoverflow.com/questions/65603440/how-do-i-use-an-iterator-twice-inside-of-the-quote-macro
    let [fields_clone0, fields_clone1, fields_clone2] = std::array::from_fn(|_i| fields.clone());

    let keypress_impl = if cfg!(feature = "keypress") { 
        quote!(
            fn inner_handle_keypress(&mut self, event: &::web_sys::KeyboardEvent) {
                match event.code().as_str() {
                    #(#key_paths => self.#fields_clone0.handle_keypress()),*,
                    _ => (),
                }
            }
        )
    } else {
        quote!()
    };

    // TODO: eventually optimize match with alternatives
    quote!(
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
            fn inner_handle_keydown(&mut self, event: &::web_sys::KeyboardEvent) {
                match event.code().as_str() {
                    #(#key_paths => self.#fields.handle_keydown()),*,
                    _ => (),
                }
            }
            
            fn inner_handle_keyup(&mut self, event: &::web_sys::KeyboardEvent) {
                match event.code().as_str() {
                    #(#key_paths => self.#fields_clone2.handle_keyup()),*,
                    _ => (),
                }
            }

            #keypress_impl
        }

        let mut #instance_name = #ty_name {
            #( #fields_clone1: #key_handlers_exprs ),*
        };
    )
    .into()
}

#[proc_macro]
pub fn new_key_handler(input: TokenStream) -> TokenStream {
    use new_key_handler::Args;

    let Args {
        path,
        state,
        fs,
    } = parse_macro_input!(input as Args);

    #[cfg(feature = "keypress")]
    let [f1,f2,f3] = fs;
    #[cfg(not(feature = "keypress"))]
    let [f1,f2] = fs;
    
    // 1 inferred type for state and 2 or 3 for handlers 
    let inferred_tys: [TypeInfer; if cfg!(feature = "keypress") {
        4
    } else {
        3
    }] = std::array::from_fn(|_|
        TypeInfer { underscore_token: <Token![_]>::default()}
    );

    #[cfg(feature = "keypress")]
    let args = quote!(#state, #f1, #f2, #f3);
    #[cfg(not(feature = "keypress"))]
    let args = quote!(#state, #f1, #f2);
    
    quote!(
        ::wasm_keyboard::implementors::KeyHandler::<
            { ::core::convert::identity::<::wasm_keyboard::uievents_code::KeyboardEventCode>(#path) as u8 },
            #( #inferred_tys ),*
        >::new(#args)
    ).into()
}
