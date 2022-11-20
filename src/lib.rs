pub use hot_reloading_macros;
pub use libloading;

use std::time::UNIX_EPOCH;

pub fn lib_updated() -> Option<std::time::SystemTime> {
    if let Ok(lib_path) = std::env::current_exe() {
        let folder = lib_path.parent().unwrap();
        let stem = lib_path.file_stem().unwrap();
        let mod_stem = format!("lib_{}", stem.to_str().unwrap());
        let mut lib_path = folder.join(&mod_stem);
        #[cfg(unix)]
        lib_path.set_extension("so");
        #[cfg(windows)]
        lib_path.set_extension("dll");
        if let Ok(lib_meta) = std::fs::metadata(&lib_path) {
            if let Ok(t) = lib_meta.modified() {
                return Some(t);
            }
        }
    }
    None
}

pub fn lib_updated_f64() -> Option<f64> {
    if let Some(updated) = lib_updated() {
        if let Ok(t) = updated.duration_since(UNIX_EPOCH) {
            return Some(t.as_secs_f64());
        }
    }
    None
}

pub fn lib_hot_updated() -> Option<std::time::SystemTime> {
    if let Ok(lib_path) = std::env::current_exe() {
        let folder = lib_path.parent().unwrap();
        let stem = lib_path.file_stem().unwrap();
        let mod_stem = format!("lib_{}_hot_in_use", stem.to_str().unwrap());
        let mut lib_path = folder.join(&mod_stem);
        #[cfg(unix)]
        lib_path.set_extension("so");
        #[cfg(windows)]
        lib_path.set_extension("dll");
        if let Ok(lib_meta) = std::fs::metadata(&lib_path) {
            if let Ok(t) = lib_meta.modified() {
                return Some(t);
            }
        }
    }
    None
}

pub fn lib_hot_updated_f64() -> Option<f64> {
    if let Some(updated) = lib_hot_updated() {
        if let Ok(t) = updated.duration_since(UNIX_EPOCH) {
            return Some(t.as_secs_f64());
        }
    }
    None
}

#[cfg(feature = "bevy")]
pub mod bevy_plugin {
    use bevy::{app::AppExit, prelude::*};
    use libloading::Library;

    #[derive(Resource, Default)]
    pub struct HotReloadLib {
        pub library: Option<Library>,
        pub updated_this_frame: bool,
        pub cargo_watch_child: Option<std::process::Child>,
    }

    #[derive(Default)]
    pub struct HotReload {
        pub auto_watch: bool,
    }

    impl Plugin for HotReload {
        fn build(&self, app: &mut App) {
            let mut child = None;
            if self.auto_watch {
                child = Some(
                    std::process::Command::new("cargo")
                        .arg("watch")
                        .arg("-w")
                        .arg("src")
                        .arg("-x")
                        .arg("build --lib --features bevy/dynamic")
                        .spawn()
                        .expect("cargo watch command failed, make sure cargo watch is installed"),
                );
            }

            // TODO move as early as possible
            app.add_system_to_stage(CoreStage::PreUpdate, update_lib)
                .add_system_to_stage(CoreStage::PostUpdate, clean_up_watch)
                .insert_resource(HotReloadLib {
                    cargo_watch_child: child,
                    ..default()
                });
        }
    }

    fn update_lib(mut lib_res: ResMut<HotReloadLib>) {
        lib_res.updated_this_frame = false;
        if let Ok(lib_path) = std::env::current_exe() {
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
                        lib_res.library = None;
                        let _ = std::fs::copy(lib_path, &hot_lib_path);
                    }
                } else {
                    lib_res.library = None;
                    std::fs::copy(lib_path, &hot_lib_path).unwrap();
                }
                if lib_res.library.is_none() {
                    unsafe {
                        if let Ok(lib) = libloading::Library::new(hot_lib_path) {
                            lib_res.library = Some(lib);
                            lib_res.updated_this_frame = true;
                        }
                    }
                }
            }
        }
    }

    fn clean_up_watch(events: EventReader<AppExit>, mut lib_res: ResMut<HotReloadLib>) {
        if !events.is_empty() {
            if let Some(child) = &mut lib_res.cargo_watch_child {
                child.kill().unwrap();
            }
        }
    }
}
