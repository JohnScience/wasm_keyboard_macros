use proc_macro::TokenStream;

mod new_key_handler;
mod start_keyboard_handler;

use start_keyboard_handler::Args;
use syn::parse_macro_input;

pub(crate) const EVENT_COUNT: usize = if cfg!(feature = "keypress") { 3 } else { 2 };
pub(crate) const EVENTS: [&str; EVENT_COUNT] = [
    "keydown",
    "keyup",
    #[cfg(feature = "keypress")]
    "keypress",
];

#[proc_macro]
pub fn start_keyboard_handler(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Args);
    let mut ts = proc_macro2::TokenStream::new();
    args.extend_with_startup_code(&mut ts);
    ts.into()
}

#[proc_macro]
pub fn new_key_handler(input: TokenStream) -> TokenStream {
    use new_key_handler::Args;

    let args = parse_macro_input!(input as Args);

    let mut ts = proc_macro2::TokenStream::new();
    args.extend_with_key_handler_expr(&mut ts);
    ts.into()
}
