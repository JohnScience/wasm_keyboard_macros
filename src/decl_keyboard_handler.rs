use std::marker::PhantomData;

use strum::EnumCount;
use syn::{bracketed, parse::Parse, punctuated::Punctuated, token::FatArrow, Expr, Token};
use uievents_code::KeyboardEventCode;

/// Struct of Arrays (SOA) for key handlers.
pub(super) struct KeyHandlersSoA {
    pub(super) instance_name: syn::Ident,
    pub(super) ty_name: syn::Ident,
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

impl Parse for KeyHandlersSoA {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let instance_name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty_name = input.parse()?;
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
        Ok(KeyHandlersSoA {
            instance_name,
            ty_name,
            key_paths,
            key_handlers_exprs,
        })
    }
}
