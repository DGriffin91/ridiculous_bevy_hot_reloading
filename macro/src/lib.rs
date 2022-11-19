extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::*;
use syn::{parse_macro_input, FnArg, ItemFn};

use quote::quote;

#[proc_macro_attribute]
pub fn make_hot(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as ItemFn);

    let fn_name = &ast.sig.ident;

    let mut args = Vec::new();
    let mut arg_names = Vec::new();
    let mut arg_types = Vec::new();

    for arg in ast.sig.inputs {
        args.push(arg.clone());
        match arg {
            FnArg::Receiver(_) => (),
            FnArg::Typed(a) => {
                match *a.pat {
                    syn::Pat::Ident(id) => arg_names.push(id.ident),
                    _ => (),
                }
                arg_types.push(a.ty)
            }
        }
    }

    let fn_name_orig_code_str = &format!("ridiculous_bevy_hot_{}", fn_name);

    let fn_name_orig_code = &Ident::new(fn_name_orig_code_str, Span::call_site());

    let orig_stmts = ast.block.stmts;

    let orig_func = quote! {
        #[no_mangle]
        pub fn #fn_name_orig_code(#[allow(unused_mut)] #(#args),*) {
            #(#orig_stmts)*
        }
    };

    let dyn_func = quote! {
        pub fn #fn_name(#[allow(unused_mut)] #(#args),*) {
            unsafe {
                if let Ok(lib) = libloading::Library::new(FILE_NAME) {
                    let func: libloading::Symbol<unsafe extern "C" fn (#(#arg_types),*) , > = lib.get(#fn_name_orig_code_str.as_bytes()).unwrap();
                    func(#(#arg_names),*);
                } else {
                    #fn_name_orig_code(#(#arg_names),*);
                }
            }
        }
    };

    TokenStream::from(quote! {
        #orig_func
        #dyn_func
    })
}
