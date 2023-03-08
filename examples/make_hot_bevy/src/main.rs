use ridiculous_bevy_hot_reloading::dyn_load_main;

fn main() {
    // Everything needs to be in the library for the TypeIds to be consistent between builds.

    // Copies library file before running so the original can be overwritten
    // Only needed if using bevy_dylib. Otherwise this could just be `lib_make_hot_bevy::main();`
    dyn_load_main("main", None);
}
