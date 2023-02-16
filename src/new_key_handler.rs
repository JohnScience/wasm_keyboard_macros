use syn::{parse::Parse, Token};

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
        Ok(Args {
            path,
            state,
            fs,
        })
    }
}
