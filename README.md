# Ridiculous bevy hot reloading

```
cargo watch -w src -x 'build --lib'
```

Or optionally
```
cargo watch -w src -x 'build --lib --features bevy/dynamic'
```

```
cargo run
```
*note: running initially with `cargo run --features bevy/dynamic` does not work because the executable is actively using the lib with the dynamic feature. Hopefully a way around this is eventually found. This could work if cargo watch builds the lib using a different name or to a different path.*

Use `bypass` feature to bypass all hot macros.

