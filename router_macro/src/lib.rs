extern crate proc_macro2;
extern crate syn;

use proc_macro::TokenStream;
use syn::{parse_macro_input, LitStr};

extern crate proc_macro;


#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as LitStr);
    let command_name = attr.value();

    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;
    // let fn_name to uppercase
    let register_name = format!("ROUTE_MAP_REGISTER_{}", fn_name);
    let register_name = syn::Ident::new(&register_name, proc_macro2::Span::call_site());

    // use linkme
    /*
        <Origin function>

        #[linkme::distributed_slice(ROUTE_MAP)]
        pub static register_name: fn() -> (string, RouteHandler) {
            (command_name.to_string(), |context, request| { Box::pin(fn_name(context, request)) })
        }
    
    */

    let expanded = quote::quote! {
        // use crate::command_table::RouteHandler;
        #input_fn

        #[linkme::distributed_slice(ROUTE_MAP)]
        pub static #register_name: fn() -> (String, RouteHandler) = || {
            (#command_name.to_string(), |context, request| { Box::pin(#fn_name(context, request)) })
        };

    };

    return TokenStream::from(expanded);
}

#[proc_macro]
pub fn init_route_map(attr: TokenStream) -> TokenStream {
    let route_map_name = parse_macro_input!(attr as syn::Ident);
    let expanded = quote::quote! {
        extern crate linkme;

        #[linkme::distributed_slice]
        pub static ROUTE_MAP: [fn() -> (String, RouteHandler)];

        static #route_map_name: std::sync::LazyLock<std::collections::HashMap<String, RouteHandler>> = std::sync::LazyLock::new(|| {
            let mut map = std::collections::HashMap::new();
            for register_fn in ROUTE_MAP {
                let (command_name, handler) = register_fn();
                map.insert(command_name, handler);
            }
            map
        });
    };

    return TokenStream::from(expanded);
}
