//! Procedural macros for the [`embedded-profiling`] crate. Meant to only be accessed
//! via that crate. See [`embedded-profiling`] for full documentation.
//!
//! [`embedded-profiling`]: https://docs.rs/embedded-profiling/
extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn};

#[proc_macro_attribute]
/// profiles the annotated function using `embedded_profiling`.
/// ```
/// #[embedded_profiling::profile_function]
/// fn my_long_running_function() {
///     println!("Hello, world!");
/// }
/// // Prints:
/// // Hello, world!
/// // <EPSS my_long_running_function: xx us>
/// ```
pub fn profile_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);
    let instrumented_function_name = function.sig.ident.to_string();

    let body = &function.block;
    let new_body: syn::Block = parse_quote! {
        {
            let start = embedded_profiling::start_snapshot();
            #body
            if let Some(dur) = embedded_profiling::end_snapshot(start, #instrumented_function_name) {
                embedded_profiling::log_snapshot(&dur);
            }
        }
    };

    function.block = Box::new(new_body);

    (quote! {
        #function
    })
    .into()
}
