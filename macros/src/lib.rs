extern crate proc_macro;

use proc_macro::TokenStream;
#[cfg(feature = "hot_reload")]
use proc_macro2::*;
#[cfg(feature = "hot_reload")]
use proc_macro_crate::{crate_name, FoundCrate};
#[cfg(feature = "hot_reload")]
use quote::{quote, ToTokens};
#[cfg(feature = "hot_reload")]
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
#[cfg(feature = "hot_reload")]
use syn::{parse_macro_input, FnArg, ItemFn};

#[proc_macro_attribute]
pub fn make_hot(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(not(feature = "hot_reload"))]
    {
        return item;
    }

    #[cfg(feature = "hot_reload")]
    {
        let ast = parse_macro_input!(item as ItemFn);

        let fn_name = &ast.sig.ident;

        // Try to make unique hash that is appended onto function name
        // So that there can be multiple functions with the same name
        // Waiting on https://github.com/rust-lang/rust/issues/54725
        // For more things to hash like source_file(), etc...
        let mut hasher = DefaultHasher::new();
        // An advantage of including the sig here is that it will crash
        // is the user changes the sig.
        ast.sig.to_token_stream().to_string().hash(&mut hasher);
        //format!("{:?}", Span::call_site().unwrap().source_file()).hash(&mut hasher);
        let hash = hasher.finish();

        let mut args = Vec::new();
        let mut args_hot_func = Vec::new();
        let mut arg_names = Vec::new();
        let mut arg_types = Vec::new();

        let mut hot_arg_names = Vec::new();
        let mut hot_arg_types = Vec::new();

        for arg in &ast.sig.inputs {
            args.push(arg.clone());
            args_hot_func.push(arg.clone());
            match arg {
                FnArg::Receiver(_) => (),
                FnArg::Typed(pt) => {
                    let mut pt = pt.clone();
                    match *pt.pat {
                        syn::Pat::Ident(ref mut id) => {
                            arg_names.push(id.ident.clone());
                            let name = id.ident.clone();
                            hot_arg_names.push(quote! { #name });
                        }
                        _ => (),
                    }
                    arg_types.push(pt.ty.clone())
                }
            }
        }

        // Below deals with converting `mut commands: Commands` to `commands: &mut Commands`
        for (idx, arg) in args_hot_func.iter_mut().enumerate() {
            match arg.clone() {
                FnArg::Receiver(_) => (),
                FnArg::Typed(a) => match &*a.ty {
                    syn::Type::Path(p) => {
                        if p.path.segments.len() == 1 {
                            if p.path.segments[0].ident == "Commands" {
                                let name = &mut hot_arg_names[idx];
                                let tok: TokenStream = quote! { #name : &mut Commands }.into();
                                *arg = parse_macro_input!(tok as FnArg);
                                *name = quote! {&mut #name};
                                break;
                            }
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                },
            }
        }

        for arg in &args_hot_func {
            match arg {
                FnArg::Receiver(_) => (),
                FnArg::Typed(a) => hot_arg_types.push(a.ty.clone()),
            }
        }

        let generics = &ast.sig.generics;
        let where_clause = &ast.sig.generics.where_clause;
        let fn_token = &ast.sig.fn_token;
        let vis = &ast.vis;

        let return_type = ast.sig.output;

        let fn_name_orig_code_str = &format!("ridiculous_bevy_hot_{}_{}", fn_name, hash);

        let fn_name_orig_code = &Ident::new(fn_name_orig_code_str, Span::call_site());

        let orig_stmts = ast.block.stmts;

        let orig_func = quote! {
            #[no_mangle] //#[allow(unused_mut)]
            #vis #fn_token #fn_name_orig_code #generics( #(#args_hot_func),*) #return_type #where_clause {
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
                        let func: #crate_found::libloading::Symbol<unsafe extern "C" fn (#(#hot_arg_types),*) #return_type , > =
                            lib.get(#fn_name_orig_code_str.as_bytes()).unwrap_or_else(|_| {
                                panic!(
                                    "Can't find required function {}",
                                    #fn_name_orig_code_str
                                )
                            });
                        return func(#(#hot_arg_names),*);
                    }
                }
                panic!("Hot reload library is None");
                //return #fn_name_orig_code(#(#hot_arg_names),*);
            }
        };

        TokenStream::from(quote! {
            #orig_func
            #dyn_func
        })
    }
}
