use quote::quote;
use syn::{parse::Parse, Token, TypeInfer};

use crate::{EVENTS, EVENT_COUNT, KEYDOWN_ID, KEYUP_ID};

pub(super) struct Args {
    /// Syntactic path that corresponds to some [`KeyboardEvent.code`].
    ///
    /// It is expected to be a path to a constant of type [`KeyboardEventCode`],
    /// e.g. `KeyboardEventCode::KeyA`.
    ///
    /// [`KeyboardEvent.code`]: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code
    /// [`KeyboardEventCode`]: https://docs.rs/uievents-code/latest/uievents_code/enum.KeyboardEventCode.html
    pub(super) path: syn::Path,
    /// The state of the key handler, which is passed by reference to event handlers
    /// on each event.
    ///
    /// Frequently, this can be wrapped in a [`Cell`][`std::cell::Cell`], in order to
    /// enable interior mutability for mutation by shared reference.
    pub(super) state: syn::Expr,
    /// Key Event Handlers in order:
    ///
    /// 1. [`keydown`].
    /// 2. [`keyup`].
    /// 3. [`keypress`] (deprecated and enabled only by `keypress` feature).
    ///
    /// [`keydown`]: https://developer.mozilla.org/en-US/docs/Web/API/Element/keydown_event
    /// [`keyup`]: https://developer.mozilla.org/en-US/docs/Web/API/Element/keyup_event
    /// [`keypress`]: https://developer.mozilla.org/en-US/docs/Web/API/Element/keypress_event
    pub(super) key_event_handlers: [KeyEventHandler; EVENT_COUNT],
}

pub struct KeyEventHandler {
    prelude: Vec<syn::Stmt>,
    closure: syn::ExprClosure,
}

impl quote::ToTokens for KeyEventHandler {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let prelude_statements = &self.prelude;
        let closure = &self.closure;
        tokens.extend(quote! {
            {
                #(#prelude_statements)*
                #closure
            }
        });
    }
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;
        input.parse::<Token![,]>()?;
        let state = {
            if input.parse::<syn::Ident>()? != "state" {
                return Err(syn::Error::new(input.span(), "expected `state`"));
            };
            input.parse::<Token![=]>()?;
            input.parse()?
        };
        // at the moment of writing, `array_try_map` feature is unstable
        let key_event_handlers: [KeyEventHandler; EVENT_COUNT] = {
            let mut key_event_handlers_it = EVENTS
                .map(|event| {
                    let mut stmts = {
                        input.parse::<Token![,]>()?;
                        if input.parse::<syn::Ident>()? != event {
                            return Err(syn::Error::new(
                                input.span(),
                                format!("expected `{}`", event),
                            ));
                        };
                        input.parse::<Token![=]>()?;
                        input.parse::<syn::Block>()?
                    }
                    .stmts;

                    let last = stmts.pop().ok_or_else(|| {
                        syn::Error::new(input.span(), format!("expected `{}` event handler", event))
                    })?;

                    match last {
                        syn::Stmt::Expr(syn::Expr::Closure(closure)) => Ok(KeyEventHandler {
                            prelude: stmts,
                            closure,
                        }),
                        _ => Err(syn::Error::new(
                            input.span(),
                            format!(
                                "last statement of `{}` event handler is expected to be a closure",
                                event
                            ),
                        )),
                    }
                })
                .into_iter();
            std::array::from_fn(|_i| match key_event_handlers_it.next().unwrap() {
                Ok(f) => f,
                Err(e) => panic!("{e}"),
            })
        };
        Ok(Args {
            path,
            state,
            key_event_handlers,
        })
    }
}

impl Args {
    // 1 inferred type for state and EVENT_COUNT for event handlers
    fn inferred_tys() -> [TypeInfer; EVENT_COUNT + 1] {
        std::array::from_fn(|_i| TypeInfer {
            underscore_token: <Token![_]>::default(),
        })
    }

    /// Responsible for implementation of [crate::new_primitive_key_handler] macro.
    pub(super) fn extend_with_primitive_key_handler_expr(&self, ts: &mut proc_macro2::TokenStream) {
        let &Self {
            ref path,
            ref state,
            ref key_event_handlers,
        } = self;

        let inferred_tys = Self::inferred_tys();

        ts.extend(quote!(
            ::wasm_keyboard::implementors::KeyHandler::<
                { ::core::convert::identity::<::wasm_keyboard::uievents_code::KeyboardEventCode>(#path) as u8 },
                #( #inferred_tys ),*
            >::new(#state, #(#key_event_handlers),*)
        ));
    }

    pub(super) fn extend_with_simplified_key_handler_expr(self, ts: &mut proc_macro2::TokenStream) {
        let Self {
            ref path,
            state,
            mut key_event_handlers,
        } = self;

        let state = quote!(
            // false is the initial value of `is_pressed`
            (::std::cell::Cell::new(false), #state)
        );

        let key_event_handlers = {
            // at the moment, enumerate on arrays in not implemented
            let key_event_handlers_syn: [proc_macro2::TokenStream; EVENT_COUNT] =
                std::array::from_fn(|i| {
                    let &mut KeyEventHandler {
                        ref prelude,
                        ref mut closure,
                    } = &mut key_event_handlers[i];
                    // We need to make the closure capture
                    // the variables from the outer closure by reference
                    // in case if captured them by value.
                    closure.capture = None;
                    match i {
                        KEYDOWN_ID => quote! {
                            {
                                #( #prelude )*
                                move |(ref is_pressed, ref state)| {
                                    let handler = #closure;
                                    if !is_pressed.get() {
                                        is_pressed.set(true);
                                        handler(state);
                                    }
                                }
                            }
                        },
                        KEYUP_ID => quote! {
                            {
                                #( #prelude )*
                                move |(ref is_pressed, ref state)| {
                                    let handler = #closure;
                                    is_pressed.set(false);
                                    handler(state);
                                }
                            }
                        },
                        #[cfg(feature = "keypress")]
                        KEYPRESS_ID => quote! {
                            {
                                #( #prelude )*
                                move |(ref is_pressed, ref state)| {
                                    let handler = #closure;
                                    handler(state);
                                    is_pressed.set(false);
                                }
                            }
                        },
                        _ => unreachable!(),
                    }
                });
            key_event_handlers_syn
        };

        let inferred_tys = Self::inferred_tys();

        ts.extend(quote!(
            ::wasm_keyboard::implementors::KeyHandler::<
                { ::core::convert::identity::<::wasm_keyboard::uievents_code::KeyboardEventCode>(#path) as u8 },
                #( #inferred_tys ),*
            >::new(#state, #(#key_event_handlers),*)
        ));
    }
}
