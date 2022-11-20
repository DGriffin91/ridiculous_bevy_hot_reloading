# Ridiculous bevy hot reloading

Usage with `bevy_plugin` feature.
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

[cargo-watch](https://crates.io/crates/cargo-watch) must be installed to use auto_watch.

Use `make_hot_system` with bevy systems, and `make_hot` with any function. 

Note: `make_hot` loads and unloads the dynamic library with every call and is much less efficient than using `make_hot_system` with the `HotReload` bevy plugin.

Setup Cargo.toml for dylib:
```
[package]
name = "your_game"
version = "0.1.0"
edition = "2021"

[lib]
name = "lib_your_game" 
path = "src/lib.rs"
crate-type = ["rlib", "dylib"]

[dependencies]
# use "bypass" feature to bypass all hot macros
ridiculous_bevy_hot_reloading = { git = "https://github.com/DGriffin91/ridiculous_bevy_hot_reloading", features = ["bevy_plugin"] } 
```
*This naming scheme with "lib_" prefix is default and required for `make_hot` but for `make_hot_system` it can be configured with HotReload::library_name.*


Manually use cargo watch with (bevy/dynamic optional):
```
cargo watch -w src -x 'build --lib --features bevy/dynamic'
```

```
cargo run
```
*note: running initially with `cargo run --features bevy/dynamic` does not work because the executable is actively using the lib with the dynamic feature. Hopefully a way around this is eventually found. This could work if cargo watch builds the lib using a different name or to a different path.*

Use `bypass` feature to bypass all hot macros.

