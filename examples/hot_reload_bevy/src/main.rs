use bevy::prelude::*;
use lib_hot_reload_bevy::{rotate, rotate2, setup};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_startup_system(setup)
        .add_system(rotate)
        .add_system(rotate2)
        .run();
}
