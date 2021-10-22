extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn};

#[proc_macro_attribute]
pub fn profile(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);
    let input_struct = proc_macro2::TokenStream::from(attr);
    // let input_struct = parse_macro_input!(attr as syn::ItemStatic);
    let instrumented_function_name = function.sig.ident.to_string();

    let body = &function.block;
    let new_body: syn::Block = parse_quote! {
        {
            #input_struct;
            let derp = #input_struct::get();
            let start = derp.start_snapshot();

            #body

            let dur = derp.end_snapshot(start, #instrumented_function_name);
            #input_struct::borrow_writer(|writer| writeln!(writer, "{}", dur).unwrap());
        }
    };

    function.block = Box::new(new_body);

    (quote! {
        #function
    })
    .into()
}
