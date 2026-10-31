//! Process-local font registration: no system font installation is required.
use std::{ffi::CString, fs, path::PathBuf};

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent() -> *mut std::ffi::c_void;
    fn FcConfigAppFontAddFile(config: *mut std::ffi::c_void, file: *const u8) -> i32;
}

const FONTS: &[(&str, &[u8])] = &[
    (
        "Excalifont-Regular-349fac6ca4700ffec595a7150a0d1e1d.ttf",
        include_bytes!(
            "../../../assets/fonts/Excalifont-Regular-349fac6ca4700ffec595a7150a0d1e1d.ttf"
        ),
    ),
    (
        "Excalifont-Regular-3f2c5db56cc93c5a6873b1361d730c16.ttf",
        include_bytes!(
            "../../../assets/fonts/Excalifont-Regular-3f2c5db56cc93c5a6873b1361d730c16.ttf"
        ),
    ),
    (
        "Excalifont-Regular-41b173a47b57366892116a575a43e2b6.ttf",
        include_bytes!(
            "../../../assets/fonts/Excalifont-Regular-41b173a47b57366892116a575a43e2b6.ttf"
        ),
    ),
    (
        "Excalifont-Regular-623ccf21b21ef6b3a0d87738f77eb071.ttf",
        include_bytes!(
            "../../../assets/fonts/Excalifont-Regular-623ccf21b21ef6b3a0d87738f77eb071.ttf"
        ),
    ),
    (
        "Excalifont-Regular-a88b72a24fb54c9f94e3b5fdaa7481c9.ttf",
        include_bytes!(
            "../../../assets/fonts/Excalifont-Regular-a88b72a24fb54c9f94e3b5fdaa7481c9.ttf"
        ),
    ),
    (
        "Excalifont-Regular-b9dcf9d2e50a1eaf42fc664b50a3fd0d.ttf",
        include_bytes!(
            "../../../assets/fonts/Excalifont-Regular-b9dcf9d2e50a1eaf42fc664b50a3fd0d.ttf"
        ),
    ),
    (
        "Excalifont-Regular-be310b9bcd4f1a43f571c46df7809174.ttf",
        include_bytes!(
            "../../../assets/fonts/Excalifont-Regular-be310b9bcd4f1a43f571c46df7809174.ttf"
        ),
    ),
];

pub fn register() -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into())).join(".cache")
        })
        .join("rsdraw/fonts");
    fs::create_dir_all(&cache).map_err(|e| format!("font cache: {e}"))?;
    for (name, bytes) in FONTS {
        let path = cache.join(name);
        if fs::read(&path).ok().as_deref() != Some(*bytes) {
            fs::write(&path, bytes).map_err(|e| format!("font cache: {e}"))?;
        }
        let cpath = CString::new(path.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        // Fontconfig owns the current config. The C call copies the filename.
        if unsafe { FcConfigAppFontAddFile(FcConfigGetCurrent(), cpath.as_ptr().cast()) } == 0 {
            return Err(format!("could not register {}", path.display()));
        }
    }
    Ok(())
}
