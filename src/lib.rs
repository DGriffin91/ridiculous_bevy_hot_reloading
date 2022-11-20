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
    use std::time::{SystemTime, UNIX_EPOCH};

    use bevy::{app::AppExit, prelude::*, window::WindowCloseRequested};
    use libloading::Library;

    #[derive(Resource, Default)]
    pub struct HotReloadLib {
        pub library: Option<Library>,
        pub updated_this_frame: bool,
        pub last_update_time: f64,
        pub cargo_watch_child: Option<std::process::Child>,
        pub library_name: String,
    }

    pub struct HotReload {
        /// Start cargo watch with plugin
        pub auto_watch: bool,
        /// Use bevy/dynamic feature with cargo watch
        pub bevy_dynamic: bool,
        /// The name of the library target in Cargo.toml:
        /// [lib]
        /// name = "lib_your_project_name"
        /// Defaults to your_project_name with lib_ prefix
        /// This should be without .so or .dll
        pub library_name: String,
    }

    impl Default for HotReload {
        fn default() -> Self {
            let lib_path = std::env::current_exe().unwrap();
            let stem = lib_path.file_stem().unwrap();
            let lib_stem = format!("lib_{}", stem.to_str().unwrap());

            HotReload {
                auto_watch: true,
                bevy_dynamic: true,
                library_name: lib_stem,
            }
        }
    }

    impl Plugin for HotReload {
        fn build(&self, app: &mut App) {
            let mut child = None;
            if self.auto_watch {
                let build_cmd = format!(
                    "build --lib {}",
                    if self.bevy_dynamic {
                        "--features bevy/dynamic"
                    } else {
                        ""
                    }
                );
                child = Some(
                    std::process::Command::new("cargo")
                        .arg("watch")
                        .arg("--watch-when-idle")
                        .arg("-w")
                        .arg("src")
                        .arg("-x")
                        .arg(build_cmd)
                        .spawn()
                        .expect("cargo watch command failed, make sure cargo watch is installed"),
                );
            }

            // TODO move as early as possible
            app.add_system_to_stage(CoreStage::PreUpdate, update_lib)
                .add_system_to_stage(CoreStage::PostUpdate, clean_up_watch)
                .insert_resource(HotReloadLib {
                    cargo_watch_child: child,
                    library_name: self.library_name.clone(),
                    ..default()
                });
        }
    }

    fn system_time_f64(t: SystemTime) -> f64 {
        t.duration_since(UNIX_EPOCH).unwrap().as_secs_f64()
    }

    fn update_lib(mut lib_res: ResMut<HotReloadLib>) {
        lib_res.updated_this_frame = false;
        if let Ok(lib_path) = std::env::current_exe() {
            let folder = lib_path.parent().unwrap();
            let lib_stem = &lib_res.library_name;
            let mut lib_path = folder.join(lib_stem);
            #[cfg(unix)]
            lib_path.set_extension("so");
            #[cfg(windows)]
            lib_path.set_extension("dll");
            if lib_path.is_file() {
                let stem = lib_path.file_stem().unwrap();
                let lib_stem = format!("{}_hot_in_use", stem.to_str().unwrap());
                let main_lib_meta = std::fs::metadata(&lib_path).unwrap();
                let mut hot_lib_path = folder.join(&lib_stem);
                #[cfg(unix)]
                hot_lib_path.set_extension("so");
                #[cfg(windows)]
                hot_lib_path.set_extension("dll");
                if hot_lib_path.exists() {
                    let hot_lib_meta = std::fs::metadata(&hot_lib_path).unwrap();
                    let hot_lib_modified = system_time_f64(hot_lib_meta.modified().unwrap());
                    let main_lib_modified = system_time_f64(main_lib_meta.modified().unwrap());
                    if hot_lib_modified < main_lib_modified
                        && system_time_f64(SystemTime::now()) - lib_res.last_update_time > 1.0
                    {
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
                            lib_res.last_update_time = system_time_f64(SystemTime::now());
                        }
                    }
                }
            }
        }
    }

    fn clean_up_watch(
        app_exit: EventReader<AppExit>,
        window_close: EventReader<WindowCloseRequested>,
        mut lib_res: ResMut<HotReloadLib>,
    ) {
        if !app_exit.is_empty() || !window_close.is_empty() {
            if let Some(child) = &mut lib_res.cargo_watch_child {
                child.kill().unwrap();
            }
        }
    }
}
