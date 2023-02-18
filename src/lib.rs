#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod key_handlers;
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

pub(crate) const KEYDOWN_ID: usize = 0;
pub(crate) const KEYUP_ID: usize = 1;
#[cfg(feature = "keypress")]
pub(crate) const KEYPRESS_ID: usize = 2;

macro_rules! arguments_doc {
    () => {
r#"# Macro input

Both [`new_primitive_key_handler!`] and [`new_simplified_key_handler!`] expect
the same [token stream](https://doc.rust-lang.org/reference/procedural-macros.html#function-like-procedural-macros)
structure — a comma-separated list of arguments adhering to the rules below.

The first argument must be a [path](https://docs.rs/syn/latest/syn/struct.Path.html)
corresponding to a constant of type [`KeyboardEventCode`](https://docs.rs/uievents-code/latest/uievents_code/enum.KeyboardEventCode.html),
e.g. `KeyboardEventCode::KeyA`.

The second argument must be a `state = `-prefixed [expression](https://docs.rs/syn/latest/syn/enum.Expr.html)
that should evaluate to the initial state of the key handler, e.g. `state = ()`.

Each of the arguments 3,4 and, with `keypress` feature, 5 must be a `<keydown|keyup|keypress> = `-prefixed
[block](https://docs.rs/syn/latest/syn/struct.Block.html) where the last
[statement](https://docs.rs/syn/latest/syn/enum.Stmt.html) must be a
[closure expression](https://docs.rs/syn/latest/syn/struct.ExprClosure.html). Extra
statements before the closure are allowed in order to write some closure prelude code. For example,
the following code snippet is a valid input for the third (but not fourth) argument:

```rust,ignore
keydown = {
    let body = body.clone();
    let document = document.clone();
    move |_state| {
        let val = document.create_element("p").unwrap();
        val.set_inner_html("W pressed down!");
        body.append_child(&val).unwrap();
    }
},
```

## Notes

Notice that unlike [`start_keywise_keyboard_handler!`], both [`new_primitive_key_handler!`] and
[`new_simplified_key_handler!`] expect a path of type [`KeyboardEventCode`](https://docs.rs/uievents-code/latest/uievents_code/enum.KeyboardEventCode.html)
(such as `KeyboardEventCode::KeyA`) and not a path of type `&'static str` (such as 
[`KEY_A`](https://docs.rs/uievents-code/latest/uievents_code/writing_system/constant.KEY_A.html)).

"#
    };
}

macro_rules! simplified_key_handling_example_doc {
    () => {
        r#"
Code snippet below creates a key handler for `W` key that will
append a paragraph to the body of the document when the key is pressed
initially and then another paragraph when the key is released. Then
a keywise keyboard handler is created from the key handler and this
keywise keyboard handler is started.

```rust,ignore
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_keyboard::{
    macros::{new_simplified_key_handler, start_keywise_keyboard_handler},
    uievents_code::{KeyboardEventCode, KEY_W},
};
use web_sys::KeyboardEvent;

// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
fn main() -> Result<(), JsValue> {
    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = Rc::new(window.document().expect("should have a document on window"));
    let body = Rc::new(document.body().expect("document should have a body"));

    let w_handler = new_simplified_key_handler!(
        KeyboardEventCode::KeyW,
        state = (),
        keydown = {
            let body = body.clone();
            let document = document.clone();
            move |_state| {
                let val = document.create_element("p").unwrap();
                val.set_inner_html("W pressed down!");
                body.append_child(&val).unwrap();
            }
        },
        keyup = {
            let body = body.clone();
            let document = document.clone();
            move |_state| {
                let val = document.create_element("p").unwrap();
                val.set_inner_html("W released!");
                body.append_child(&val).unwrap();
            }
        }
    );

    start_keywise_keyboard_handler!(kh: Kh, document, [KEY_W => w_handler]);

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    Ok(())
}
```
"#
    };
}

/// This macro creates a new "keywise" keyboard handler and runs its start-up code.
///
/// To be more precise, due to the absence of [variadic generics], this macro declares a new generic type
/// for the "keywise keyboard handler", creates an instance of the parameterized type, and
/// adds event listeners to the target, which often will be an instance of [`web_sys::Document`].
///
/// Keywise keyboard handler is a keyboard handler that is created from a list of key handlers,
/// each with its own state.
///
/// Individual key handlers can be created using [`new_primitive_key_handler!`] or[`new_simplified_key_handler!`].
///
/// # Example
///
#[doc = simplified_key_handling_example_doc!()]
///
/// ## Notes
/// 
/// Notice that even though both [`new_primitive_key_handler!`] and
/// [`new_simplified_key_handler!`] expect a path of type [`KeyboardEventCode`](https://docs.rs/uievents-code/latest/uievents_code/enum.KeyboardEventCode.html)
/// (such as 
/// [`KeyboardEventCode::KeyA`](https://docs.rs/uievents-code/latest/uievents_code/enum.KeyboardEventCode.html#variant.KeyA)),
/// [`start_keywise_keyboard_handler!`] expects a path of type `&'static str` (such as 
/// [`KEY_A`](https://docs.rs/uievents-code/latest/uievents_code/writing_system/constant.KEY_A.html)).
///
/// [`web_sys::Document`]: https://docs.rs/web-sys/latest/web_sys/struct.Document.html
/// [variadic generics]: https://github.com/rust-lang/rust/issues/10124
#[proc_macro]
pub fn start_keywise_keyboard_handler(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Args);
    let mut ts = proc_macro2::TokenStream::new();
    args.extend_with_startup_code(&mut ts);
    ts.into()
}

/// This macro creates a new "primitive" [key \[event\] handler].
///
/// A primitive key handler is a key handler that assumes the responsibility
/// to account for multiple consequent triggers of [`keydown`] event when
/// the key is long-pressed.
///
/// For simpler use cases, consider using [`new_simplified_key_handler!`].
///
/// One notable use of key handlers is creating and starting a keywise keyboard handler, e.g.
/// using [`start_keywise_keyboard_handler!`].
///
/// # Example
///
/// Code snippet below creates a key handler for `W` key that will
/// append a paragraph to the body of the document when the key is pressed
/// initially, each period of time after holding, and then another
/// paragraph when the key is released.
///
/// ```rust,ignore
/// use std::rc::Rc;
///
/// use wasm_bindgen::prelude::*;
/// use wasm_keyboard::{
///     macros::{new_primitive_key_handler, start_keywise_keyboard_handler},
///     uievents_code::{KeyboardEventCode, KEY_W},
/// };
/// use web_sys::KeyboardEvent;
///
/// // Called when the wasm module is instantiated
/// #[wasm_bindgen(start)]
/// fn main() -> Result<(), JsValue> {
///     // Use `web_sys`'s global `window` function to get a handle on the global
///     // window object.
///     let window = web_sys::window().expect("no global `window` exists");
///     let document = Rc::new(window.document().expect("should have a document on window"));
///     let body = Rc::new(document.body().expect("document should have a body"));
///
///     let w_handler = new_primitive_key_handler!(
///         KeyboardEventCode::KeyW,
///         state = (),
///         keydown = {
///             let body = body.clone();
///             let document = document.clone();
///             move |_state| {
///                 let val = document.create_element("p").unwrap();
///                 val.set_inner_html("W pressed down!");
///                 body.append_child(&val).unwrap();
///             }
///         },
///         keyup = {
///             let body = body.clone();
///             let document = document.clone();
///             move |_state| {
///                 let val = document.create_element("p").unwrap();
///                 val.set_inner_html("W released!");
///                 body.append_child(&val).unwrap();
///             }
///         }
///     );
///
///     start_keywise_keyboard_handler!(kh: Kh, document, [KEY_W => w_handler]);
///
///     // Manufacture the element we're gonna append
///     let val = document.create_element("p")?;
///     val.set_inner_html("Hello from Rust!");
///
///     body.append_child(&val)?;
///
///     Ok(())
/// }
/// ```
///
/// ## Screenshot
///
/// ![screenshot](https://i.imgur.com/648X1fL.png)
///
/// ## Notes
///
/// The screenshot above was produced based on code from [`wasm_keyboard_example`]
/// where [`new_simplified_key_handler!`] is replaced with [`new_primitive_key_handler!`].
///
#[doc = arguments_doc!()]
///
/// [`keydown`]: https://developer.mozilla.org/en-US/docs/Web/API/Element/keydown_event
/// [key \[event\] handler]: https://en.wikipedia.org/wiki/Event_(computing)#Event_handler
/// [`wasm_keyboard_example`]: https://github.com/JohnScience/wasm_keyboard_example
#[proc_macro]
pub fn new_primitive_key_handler(input: TokenStream) -> TokenStream {
    use key_handlers::Args;

    let args = parse_macro_input!(input as Args);

    let mut ts = proc_macro2::TokenStream::new();
    args.extend_with_primitive_key_handler_expr(&mut ts);
    ts.into()
}

/// This macro creates a new "simplified" [key \[event\] handler].
///
/// A simplified key handler is a key handler for which the [`keydown`] event
/// does not trigger the handler consequent times when the key is long-pressed
/// and the handler closures accept the undecorated `state`. Internally, the
/// simplified key handler uses the `state` augmented with `is_pressed: Cell<bool>`
/// to account for the consequent triggers of `keydown` event when a key is long-pressed.
///
/// For finer control, consider using [`new_primitive_key_handler!`].
///
/// One notable use of key handlers is creating and starting a keywise keyboard handler, e.g.
/// using [`start_keywise_keyboard_handler!`].
///
/// # Example
///
#[doc = simplified_key_handling_example_doc!()]
///
/// ## Screenshot
///
/// ![screenshot](https://i.imgur.com/nEKLzrN.png)
///
/// ## Notes
///
/// The screenshot above was produced using the code from [`wasm_keyboard_example`].
///
#[doc = arguments_doc!()]
///
/// [`keydown`]: https://developer.mozilla.org/en-US/docs/Web/API/Element/keydown_event
/// [key \[event\] handler]: https://en.wikipedia.org/wiki/Event_(computing)#Event_handler
/// [`wasm_keyboard_example`]: https://github.com/JohnScience/wasm_keyboard_example
#[proc_macro]
pub fn new_simplified_key_handler(input: TokenStream) -> TokenStream {
    use key_handlers::Args;

    let args = parse_macro_input!(input as Args);

    let mut ts = proc_macro2::TokenStream::new();
    args.extend_with_simplified_key_handler_expr(&mut ts);
    ts.into()
}
