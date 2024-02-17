# Ridiculous bevy hot reloading

This crate is currently very experimental.

# `#[make_hot]`

Use with bevy 0.13
```rs
//Default has auto_watch: true, bevy_bevy_dylib: true, and lib_ prefix
app.add_plugin(HotReloadPlugin::default());

[...]

#[make_hot]
pub fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_x(time.delta_seconds() * 1.0);
    }
}
```

```
cargo run
```

[cargo-watch](https://crates.io/crates/cargo-watch) must be installed to use auto_watch.

### Warning: There are a significant number of ways that hot reloading can result in undefined behavior. When hot reloading, do not change any function signatures or structs.
*It is potentially possible to change structs and non-system function signatures, but only if they are exclusively used in the hot reloaded code paths and are not referenced or stored anywhere else. (including in `Local<>`)*



Setup Cargo.toml for dylib:
```toml
[package]
name = "your_app"
version = "0.1.0"
edition = "2021"

[lib]
# This naming scheme with "lib_" prefix is default but can be configured with HotReload::library_name.
name = "lib_your_app" 
path = "src/lib.rs"
crate-type = ["rlib", "dylib"]

[features]
hot_reload = ["ridiculous_bevy_hot_reloading/hot_reload"]

[dependencies]
ridiculous_bevy_hot_reloading = { git = "https://github.com/DGriffin91/ridiculous_bevy_hot_reloading" } 
```

To run with hot reloading:

```shell
cargo run --features hot_reload
```

## How `#[make_hot]` works
Given this rotate system as input:
```rs
#[make_hot]
pub fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_x(time.delta_seconds() * 1.0);
    }
}
```

`#[make_hot]` replaces contents of the rotate function with one that will try to call the function from the dynamic library. To make this possible, it adds the `hot_reload_lib_internal_use_only: Res<HotReloadLibInternalUseOnly>` argument to the rotate system.

```rs
// Recursive expansion of make_hot! macro
// ==============================================
#[no_mangle]
pub fn ridiculous_bevy_hot_rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_x(time.delta_seconds() * 1.0);
    }
}

#[allow(unused_mut)] // added because rust analyzer will complain about the mut on `mut query: Query<`
pub fn rotate(
    mut query: Query<&mut Transform, With<Shape>>,
    time: Res<Time>,
    hot_reload_lib_internal_use_only: Res<HotReloadLibInternalUseOnly>,
) {
    if let Some(lib) = &hot_reload_lib_internal_use_only.library {
        unsafe {
            let func: libloading::Symbol<
                unsafe extern "C" fn(Query<&mut Transform, With<Shape>>, Res<Time>),
            > = lib
                .get("ridiculous_bevy_hot_rotate".as_bytes())
                .unwrap_or_else(|_| {
                    panic!(
                        "Can't find required function {}",
                        "ridiculous_bevy_hot_rotate"
                    )
                });
            return func(query, time);
        }
    }
    panic!("Hot reload library is None");
}
```

`HotReloadPlugin` rebuilds the code using `cargo-watch` in a subprocess. And handles refreshing the loaded library. When the libray is refreshed, a copy is made. This copy is then loaded, that allows cargo to build and output the library while the previous version is still in use.

