extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::*;
use proc_macro_crate::{crate_name, FoundCrate};
use syn::{parse_macro_input, FnArg, ItemFn};

use quote::quote;

#[proc_macro_attribute]
pub fn make_hot(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "bypass")]
    {
        return item;
    }
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

    let generics = &ast.sig.generics;
    let where_clause = &ast.sig.generics.where_clause;
    let fn_token = &ast.sig.fn_token;
    let vis = &ast.vis;

    let return_type = ast.sig.output;

    let fn_name_orig_code_str = &format!("ridiculous_bevy_hot_{}", fn_name);

    let fn_name_orig_code = &Ident::new(fn_name_orig_code_str, Span::call_site());

    let orig_stmts = ast.block.stmts;

    let orig_func = quote! {
        #[no_mangle] //#[allow(unused_mut)]
        #vis #fn_token #fn_name_orig_code #generics( #(#args),*) #return_type #where_clause {
            #(#orig_stmts)*
        }
    };

    let found_crate = crate_name("ridiculous_bevy_hot_reloading")
        .expect("ridiculous_bevy_hot_reloading is present in `Cargo.toml`");

    let crate_found = match found_crate {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident)
        }
    };

    let dyn_func = quote! {
        #[allow(unused_mut)] // added because rust analyzer will complain about the mut on `mut query: Query<`
        #vis #fn_token #fn_name #generics( #(#args),*,
        hot_reload_lib_internal_use_only: Res<ridiculous_bevy_hot_reloading::HotReloadLibInternalUseOnly>) #return_type #where_clause {
            if let Some(lib) = &hot_reload_lib_internal_use_only.library {
                unsafe {
                    let func: #crate_found::libloading::Symbol<unsafe extern "C" fn (#(#arg_types),*) #return_type , > =
                                           lib.get(#fn_name_orig_code_str.as_bytes()).unwrap();
                    return func(#(#arg_names),*);
                }
            }
            return #fn_name_orig_code(#(#arg_names),*);
        }
    };

    TokenStream::from(quote! {
        #orig_func
        #dyn_func
    })
}
