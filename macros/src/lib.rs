extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::*;
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

    let return_type = ast.sig.output;

    let fn_name_orig_code_str = &format!("ridiculous_bevy_hot_{}", fn_name);

    let fn_name_orig_code = &Ident::new(fn_name_orig_code_str, Span::call_site());

    let orig_stmts = ast.block.stmts;

    let orig_func = quote! {
        #[no_mangle]
        pub fn #fn_name_orig_code(#[allow(unused_mut)] #(#args),*) #return_type {
            #(#orig_stmts)*
        }
    };

    let dyn_func = quote! {
        pub fn #fn_name(#[allow(unused_mut)] #(#args),*) #return_type  {
            unsafe {
                if let Ok(mut lib_path) = std::env::current_exe() {
                    let folder = lib_path.parent().unwrap();
                    let stem = lib_path.file_stem().unwrap();
                    let mod_stem = format!("lib_{}", stem.to_str().unwrap());
                    let mut lib_path = folder.join(&mod_stem);
                    #[cfg(unix)]
                    lib_path.set_extension("so");
                    #[cfg(windows)]
                    lib_path.set_extension("dll");
                    if lib_path.is_file() {
                        let stem = lib_path.file_stem().unwrap();
                        let mod_stem = format!("{}_hot_in_use", stem.to_str().unwrap());

                        let main_lib_meta = std::fs::metadata(&lib_path).unwrap();
                        let mut hot_lib_path = folder.join(&mod_stem);
                        #[cfg(unix)]
                        hot_lib_path.set_extension("so");
                        #[cfg(windows)]
                        hot_lib_path.set_extension("dll");

                        if hot_lib_path.exists() {
                            let hot_lib_meta = std::fs::metadata(&hot_lib_path).unwrap();
                            if hot_lib_meta.modified().unwrap() < main_lib_meta.modified().unwrap() {
                                // Try to copy
                                let _ = std::fs::copy(lib_path, &hot_lib_path);
                            }
                        } else {
                            std::fs::copy(lib_path, &hot_lib_path).unwrap();
                        }

                        if let Ok(lib) = libloading::Library::new(hot_lib_path) {
                            let func: libloading::Symbol<unsafe extern "C" fn (#(#arg_types),*) #return_type , > =
                                                   lib.get(#fn_name_orig_code_str.as_bytes()).unwrap();
                            return func(#(#arg_names),*);
                        }
                    }
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
