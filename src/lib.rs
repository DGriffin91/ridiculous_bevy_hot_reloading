pub extern crate hot_reloading_macros;
pub extern crate libloading;

use std::{any::TypeId, path::PathBuf, time::Duration};

use bevy::{prelude::*, utils::Instant};
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

#[derive(Debug, Event)]
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
    pub cargo_watch_child: Option<ChildGuard>,
    pub library_paths: LibPathSet,
}

pub struct HotReloadPlugin {
    /// Start cargo watch with plugin
    pub auto_watch: bool,
    /// Use bevy_dylib feature with cargo watch
    pub bevy_dylib: bool,
    /// The name of the library target in Cargo.toml:
    /// [lib]
    /// name = "lib_your_project_name"
    /// Defaults to your_project_name with lib_ prefix
    /// This should be without .so or .dll
    pub library_name: Option<String>,
}

impl Default for HotReloadPlugin {
    fn default() -> Self {
        HotReloadPlugin {
            auto_watch: true,
            bevy_dylib: true,
            library_name: None,
        }
    }
}

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(feature = "hot_reload"))]
        {
            app.add_event::<HotReloadEvent>()
                .insert_resource(HotReload {
                    updated_this_frame: false,
                    disable_reload: true,
                    ..default()
                });
            return;
        }

        #[cfg(feature = "hot_reload")]
        {
            let mut child = None;

            let release_mode = false;
            #[cfg(not(debug_assertions))]
            let release_mode = true;

            let library_paths = LibPathSet::new(self.library_name.clone()).unwrap();

            if self.auto_watch {
                let build_cmd = format!(
                    "build --lib --target-dir {} {} {} --features ridiculous_bevy_hot_reloading/hot_reload",
                    library_paths.folder.parent().unwrap().to_string_lossy(),
                    if release_mode { "--release" } else { "" },
                    if self.bevy_dylib {
                        "--features bevy/dynamic_linking"
                    } else {
                        ""
                    },
                );
                child = Some(ChildGuard(
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
                ));
            }

            // TODO move as early as possible
            app.add_systems(PreUpdate, (update_lib, check_type_ids).chain())
                //.add_system_to_stage(CoreStage::PostUpdate, clean_up_watch)
                .add_event::<HotReloadEvent>()
                .insert_resource(HotReloadLibInternalUseOnly {
                    cargo_watch_child: child,
                    library: None,
                    updated_this_frame: false,
                    // Using 1 second ago so to trigger lib load immediately instead of in 1 second
                    last_update_time: Instant::now().checked_sub(Duration::from_secs(1)).unwrap(),
                    library_paths,
                })
                .insert_resource(HoldTypeId(TypeId::of::<HoldTypeId>()))
                .insert_resource(HotReload::default());
        }
    }
}

pub struct LibPathSet {
    folder: PathBuf,
    name: String,
    extension: String,
}

impl LibPathSet {
    fn new(library_name: Option<String>) -> Option<Self> {
        if let Ok(lib_path) = std::env::current_exe() {
            let name = library_name.unwrap_or({
                let stem = lib_path.file_stem().unwrap();
                format!("lib_{}", stem.to_str().unwrap())
            });
            let folder = lib_path.parent().unwrap();

            #[cfg(unix)]
            let extension = String::from("so");
            #[cfg(windows)]
            let extension = String::from("dll");

            return Some(LibPathSet {
                folder: (folder).to_path_buf(),
                name,
                extension,
            });
        }
        None
    }

    /// File path the compiler outputs to
    fn lib_file_path(&self) -> PathBuf {
        self.folder.join(&self.name).with_extension(&self.extension)
    }
    #[cfg(feature = "hot_reload")]
    /// File path copied to for hot reloads
    fn hot_in_use_file_path(&self) -> PathBuf {
        self.folder
            .join(format!("{}_hot_in_use", self.name))
            .with_extension(&self.extension)
    }

    /// File path copied to for initial run
    fn main_in_use_file_path(&self) -> PathBuf {
        self.folder
            .join(format!("{}_main_in_use", self.name))
            .with_extension(&self.extension)
    }
}

#[cfg(feature = "hot_reload")]
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

    let lib_file_path = hot_reload_int.library_paths.lib_file_path();
    let hot_in_use_file_path = hot_reload_int.library_paths.hot_in_use_file_path();

    // copy over and load lib if it has been updated, or hasn't been initially
    if lib_file_path.is_file() {
        if hot_in_use_file_path.is_file() {
            let hot_lib_meta = std::fs::metadata(&hot_in_use_file_path).unwrap();
            let main_lib_meta = std::fs::metadata(&lib_file_path).unwrap();
            if hot_lib_meta.modified().unwrap() < main_lib_meta.modified().unwrap()
                && hot_reload_int.last_update_time.elapsed() > Duration::from_secs(1)
            {
                hot_reload_int.library = None;
                let _ = std::fs::copy(lib_file_path, &hot_in_use_file_path);
            }
        } else {
            hot_reload_int.library = None;
            std::fs::copy(lib_file_path, &hot_in_use_file_path).unwrap();
        }
        if hot_reload_int.library.is_none() {
            unsafe {
                let lib = libloading::Library::new(&hot_in_use_file_path).unwrap_or_else(|_| {
                    panic!(
                        "Can't open required library {}",
                        &hot_in_use_file_path.to_string_lossy()
                    )
                });
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

    hot_reload.updated_this_frame = hot_reload_int.updated_this_frame;
    hot_reload.last_update_time = hot_reload_int.last_update_time;
}

pub struct ChildGuard(pub std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        match self.0.kill() {
            Err(e) => println!("Could not kill cargo watch process: {}", e),
            Ok(_) => println!("Successfully killed cargo watch process"),
        }
    }
}

#[derive(Resource)]
struct HoldTypeId(TypeId);

mod ridiculous_bevy_hot_reloading {
    pub use super::*;
}

#[cfg(feature = "hot_reload")]
#[hot_reloading_macros::make_hot]
fn check_type_ids(type_id: Res<HoldTypeId>, _hot_reload_int: Res<HotReloadLibInternalUseOnly>) {
    if type_id.0 != TypeId::of::<HoldTypeId>() {
        // If we include Res<HotReloadLibInternalUseOnly> the ChildGuard gets dropped
        // Otherwise cargo watch keeps running
        panic!(
            "{}",
            "ridiculous_bevy_hot_reloading: ERROR TypeIds \
        do not match, this happens when the primary \
        and dynamic libraries are not identically \
        built. Make sure either both, or neither are \
        using bevy_dylib"
        );
    }
}

/// Copies library file before running so the original can be overwritten
/// Only needed if using bevy_dylib
pub fn dyn_load_main(main_function_name: &str, library_name: Option<String>) {
    if let Some(lib_paths) = LibPathSet::new(library_name) {
        let lib_file_path = lib_paths.lib_file_path();
        let main_in_use_file_path = lib_paths.main_in_use_file_path();

        if lib_file_path.is_file() {
            std::fs::copy(lib_file_path, &main_in_use_file_path).unwrap();
            unsafe {
                if let Ok(lib) = libloading::Library::new(main_in_use_file_path) {
                    let func: libloading::Symbol<unsafe extern "C" fn()> =
                        lib.get(main_function_name.as_bytes()).unwrap();
                    func();
                }
            }
        } else {
            panic!("Could not find library file {:?}", lib_file_path);
        }
    }
}
