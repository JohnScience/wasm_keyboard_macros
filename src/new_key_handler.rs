use quote::quote;
use syn::{parse::Parse, Token, TypeInfer};

pub(super) struct Args {
    pub(super) path: syn::Path,
    pub(super) state: syn::Expr,
    pub(super) fs: [syn::Expr; if cfg!(feature = "keypress") { 3 } else { 2 }],
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;
        input.parse::<Token![,]>()?;
        let state = input.parse()?;
        input.parse::<Token![,]>()?;
        let f1 = input.parse()?;
        input.parse::<Token![,]>()?;
        let f2 = input.parse()?;
        #[cfg(feature = "keypress")]
        let f3 = {
            input.parse::<Token![,]>()?;
            input.parse()?
        };
        #[cfg(feature = "keypress")]
        let fs = [f1, f2, f3];
        #[cfg(not(feature = "keypress"))]
        let fs = [f1, f2];
        Ok(Args { path, state, fs })
    }
}

impl Args {
    pub(super) fn extend_with_key_handler_expr(&self, ts: &mut proc_macro2::TokenStream) {
        let &Self {
            ref path,
            ref state,
            ref fs,
        } = self;
        #[cfg(feature = "keypress")]
        let [f1, f2, f3] = fs;
        #[cfg(not(feature = "keypress"))]
        let [f1, f2] = fs;

        // 1 inferred type for state and 2 or 3 for handlers
        let inferred_tys: [TypeInfer; if cfg!(feature = "keypress") { 4 } else { 3 }] =
            std::array::from_fn(|_| TypeInfer {
                underscore_token: <Token![_]>::default(),
            });

        #[cfg(feature = "keypress")]
        let args = quote!(#state, #f1, #f2, #f3);
        #[cfg(not(feature = "keypress"))]
        let args = quote!(#state, #f1, #f2);

        ts.extend(quote!(
        ::wasm_keyboard::implementors::KeyHandler::<
            { ::core::convert::identity::<::wasm_keyboard::uievents_code::KeyboardEventCode>(#path) as u8 },
            #( #inferred_tys ),*
        >::new(#args)
    ));
    }
}
