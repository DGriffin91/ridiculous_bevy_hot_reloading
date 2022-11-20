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
