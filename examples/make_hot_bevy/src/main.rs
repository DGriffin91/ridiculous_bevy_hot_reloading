use bevy::prelude::*;
use lib_make_hot_bevy::{print_last_update, rotate, rotate2, setup};
use ridiculous_bevy_hot_reloading::HotReload;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_startup_system(setup)
        .add_system(rotate)
        .add_system(rotate2)
        .add_system(print_last_update)
        //Default has auto_watch: true, bevy_dynamic: true, and lib_ prefix
        .add_plugin(HotReload::default())
        .run();
}
