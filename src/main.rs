use bevy::prelude::*;
use ridiculous_bevy_hot_reloading::{rotate, rotate2, setup};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_startup_system(setup)
        .add_system(rotate)
        .add_system(rotate2)
        .run();
}
