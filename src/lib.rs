pub extern crate hot_reloading_macros;
pub extern crate libloading;

use std::{any::TypeId, time::Duration};

use bevy::{app::AppExit, prelude::*, utils::Instant, window::WindowCloseRequested};
use libloading::Library;

/// Get info about HotReload state.
#[derive(Resource)]
pub struct HotReload {
    pub updated_this_frame: bool,
    pub last_update_time: Instant,
    pub disable_reload: bool,
}

impl Default for HotReload {
    fn default() -> Self {
        HotReload {
            updated_this_frame: false,
            disable_reload: false,
            last_update_time: Instant::now().checked_sub(Duration::from_secs(1)).unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct HotReloadEvent {
    pub last_update_time: Instant,
}

/// Only for HotReload internal use. Must be pub because it is
/// inserted as an arg on systems with #[make_hot]
#[derive(Resource)]
pub struct HotReloadLibInternalUseOnly {
    pub library: Option<Library>,
    pub updated_this_frame: bool,
    pub last_update_time: Instant,
    pub cargo_watch_child: Option<std::process::Child>,
    pub library_name: String,
}

pub struct HotReloadPlugin {
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

impl Default for HotReloadPlugin {
    fn default() -> Self {
        let lib_path = std::env::current_exe().unwrap();
        let stem = lib_path.file_stem().unwrap();
        let lib_stem = format!("lib_{}", stem.to_str().unwrap());

        HotReloadPlugin {
            auto_watch: true,
            bevy_dynamic: true,
            library_name: lib_stem,
        }
    }
}

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bypass")]
        {
            app.add_event::<HotReloadEvent>()
                .insert_resource(HotReload {
                    updated_this_frame: false,
                    disable_reload: true,
                    ..default()
                });
            return;
        }

        let mut child = None;

        let release_mode = false;
        #[cfg(not(debug_assertions))]
        let release_mode = true;

        if self.auto_watch {
            let build_cmd = format!(
                "build --lib {} {}",
                if release_mode { "--release" } else { "" },
                if self.bevy_dynamic {
                    "--features bevy/dynamic"
                } else {
                    ""
                }
            );
            child = Some(
                std::process::Command::new("cargo")
                    .arg("watch")
                    .arg("--postpone")
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
            .add_system_to_stage(CoreStage::PreUpdate, check_type_ids.after(update_lib))
            .add_system_to_stage(CoreStage::PostUpdate, clean_up_watch)
            .add_event::<HotReloadEvent>()
            .insert_resource(HotReloadLibInternalUseOnly {
                cargo_watch_child: child,
                library_name: self.library_name.clone(),
                library: None,
                updated_this_frame: false,
                // Using 1 second ago so to trigger lib load immediately instead of in 1 second
                last_update_time: Instant::now().checked_sub(Duration::from_secs(1)).unwrap(),
            })
            .insert_resource(HoldTypeId(TypeId::of::<HoldTypeId>()))
            .insert_resource(HotReload::default());
    }
}

fn update_lib(
    mut hot_reload_int: ResMut<HotReloadLibInternalUseOnly>,
    mut hot_reload: ResMut<HotReload>,
    mut event: EventWriter<HotReloadEvent>,
) {
    hot_reload_int.updated_this_frame = false;
    hot_reload.updated_this_frame = false;
    if hot_reload.disable_reload {
        return;
    }
    if let Ok(lib_path) = std::env::current_exe() {
        let folder = lib_path.parent().unwrap();
        let lib_stem = &hot_reload_int.library_name;
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
                if hot_lib_meta.modified().unwrap() < main_lib_meta.modified().unwrap()
                    && hot_reload_int.last_update_time.elapsed() > Duration::from_secs(1)
                {
                    hot_reload_int.library = None;
                    let _ = std::fs::copy(lib_path, &hot_lib_path);
                }
            } else {
                hot_reload_int.library = None;
                std::fs::copy(lib_path, &hot_lib_path).unwrap();
            }
            if hot_reload_int.library.is_none() {
                unsafe {
                    if let Ok(lib) = libloading::Library::new(hot_lib_path) {
                        // TODO set globals like IoTaskPool here

                        hot_reload_int.library = Some(lib);
                        hot_reload_int.updated_this_frame = true;
                        hot_reload_int.last_update_time = Instant::now();
                        event.send(HotReloadEvent {
                            last_update_time: hot_reload_int.last_update_time,
                        });
                    }
                }
            }
        }
    }
    hot_reload.updated_this_frame = hot_reload_int.updated_this_frame;
    hot_reload.last_update_time = hot_reload_int.last_update_time;
}

fn clean_up_watch(
    app_exit: EventReader<AppExit>,
    window_close: EventReader<WindowCloseRequested>,
    mut lib_res: ResMut<HotReloadLibInternalUseOnly>,
) {
    if !app_exit.is_empty() || !window_close.is_empty() {
        if let Some(child) = &mut lib_res.cargo_watch_child {
            child.kill().unwrap();
        }
    }
}

#[derive(Resource)]
struct HoldTypeId(TypeId);

mod ridiculous_bevy_hot_reloading {
    pub use super::*;
}

#[hot_reloading_macros::make_hot]
fn check_type_ids(type_id: Res<HoldTypeId>) {
    if type_id.0 != TypeId::of::<HoldTypeId>() {
        panic!(
            "ridiculous_bevy_hot_reloading: ERROR TypeIds \
            do not match, this happens when the primary \
            and dynamic libraries are not identically \
            built. Make sure either both, or neither are \
            using bevy/dynamic"
        );
    }
}

/// Copies library file before running so the original can be overwritten
/// Only needed if using bevy/dynamic
pub fn dyn_load_main(main_function_name: &str, library_name: Option<String>) {
    if let Ok(lib_path) = std::env::current_exe() {
        let lib_stem = library_name.unwrap_or({
            let stem = lib_path.file_stem().unwrap();
            format!("lib_{}", stem.to_str().unwrap())
        });

        let folder = lib_path.parent().unwrap();
        let mut lib_path = folder.join(lib_stem);
        #[cfg(unix)]
        lib_path.set_extension("so");
        #[cfg(windows)]
        lib_path.set_extension("dll");
        if lib_path.is_file() {
            let stem = lib_path.file_stem().unwrap();
            let lib_stem = format!("{}_main_in_use", stem.to_str().unwrap());
            let mut hot_lib_path = folder.join(&lib_stem);
            #[cfg(unix)]
            hot_lib_path.set_extension("so");
            #[cfg(windows)]
            hot_lib_path.set_extension("dll");
            std::fs::copy(lib_path, &hot_lib_path).unwrap();

            unsafe {
                if let Ok(lib) = libloading::Library::new(hot_lib_path) {
                    let func: libloading::Symbol<unsafe extern "C" fn()> =
                        lib.get(main_function_name.as_bytes()).unwrap();
                    func();
                }
            }
        }
    }
}
