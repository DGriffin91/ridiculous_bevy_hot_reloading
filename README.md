# Ridiculous bevy hot reloading

# `#[make_hot_system]`

Use with `features = ["bevy_plugin"]`.
```rs
//Default has auto_watch: true, bevy_dynamic: true, and lib_ prefix
app.add_plugin(HotReload::default());

[...]

#[make_hot_system]
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
name = "lib_your_app" 
path = "src/lib.rs"
crate-type = ["rlib", "dylib"]

[dependencies]
# use "bypass" feature to bypass all hot macros
ridiculous_bevy_hot_reloading = { git = "https://github.com/DGriffin91/ridiculous_bevy_hot_reloading", 
                                  features = ["bevy_plugin"] } 
```
*This naming scheme with "lib_" prefix is default and required for `#[make_hot]` but for `#[make_hot_system]` it can be configured with HotReload::library_name.*




*note: running initially with `cargo run --features bevy/dynamic` does not work because the executable is actively using the lib with the dynamic feature. Hopefully a way around this is eventually found. This could work if cargo watch builds the lib using a different name or to a different path.*

## How `#[make_hot_system]` works
Given this rotate system as input:
```rs
#[make_hot_system]
pub fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_x(time.delta_seconds() * 1.0);
    }
}
```

`#[make_hot_system]` replaces contents of the rotate function with one that will try to call the function from the dynamic library. If the library is not loaded it will call the original (but renamed) function. To make this possible, it adds the `lib_res: Res<HotReloadLib>` argument to the rotate system.

```rs
// Recursive expansion of make_hot_system! macro
// ==============================================

#[no_mangle]
pub fn ridiculous_bevy_hot_rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_x(time.delta_seconds() * 1.0);
    }
}

pub fn rotate(
    mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>, lib_res: Res<HotReloadLib>,
) {
    if let Some(lib) = &lib_res.library {
        unsafe {
            let func: ridiculous_bevy_hot_reloading::libloading::Symbol<
                unsafe extern "C" fn(Query<&mut Transform, With<Shape>>, Res<Time>),
            > = lib.get("ridiculous_bevy_hot_rotate".as_bytes()).unwrap();
            return func(query, time);
        }
    }
    return ridiculous_bevy_hot_rotate(query, time);
}
```

The `HotReload` plugin rebuilds the code using `cargo-watch` in a subprocess. And handles refreshing the loaded library. When the libray is refreshed, a copy is made. This copy is then loaded, that allows cargo to build and output the library while the previous version is still in use.

# `#[make_hot]`

Use `#[make_hot_system]` with bevy systems, and `#[make_hot]` with any function. 

Note: `#[make_hot]` loads and unloads the dynamic library with every call and is much less efficient than using `#[make_hot_system]` with the `HotReload` bevy plugin.

Manually using cargo watch is required for `#[make_hot]` (bevy/dynamic is optional):
```
cargo watch -w src -x 'build --lib --features bevy/dynamic'
```
```
cargo run
```
Setup Cargo.toml for dylib:
```toml
[package]
name = "your_app"
version = "0.1.0"
edition = "2021"

[lib]
name = "lib_your_app" 
path = "src/lib.rs"
crate-type = ["rlib", "dylib"]

[dependencies]
# use "bypass" feature to bypass all hot macros
ridiculous_bevy_hot_reloading = { git = "https://github.com/DGriffin91/ridiculous_bevy_hot_reloading" } 
```