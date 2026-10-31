//! Capture the compositor's desktop (including annotations), then save locally.
use gtk4::{gio, glib};
use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

pub fn directory() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Pictures)
        .unwrap_or_else(|| glib::home_dir().join("Pictures"))
        .join("Screenshots")
}

pub async fn capture(output: Option<&str>) -> Result<Vec<u8>, String> {
    // Pass arguments directly: paths and desktop contents never enter a shell.
    let mut args = vec![OsStr::new("grim"), OsStr::new("-t"), OsStr::new("png")];
    if let Some(output) = output {
        args.extend([OsStr::new("-o"), OsStr::new(output)]);
    }
    args.push(OsStr::new("-"));
    let process = gio::Subprocess::newv(
        &args,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|e| format!("Could not start grim. Install grim to save desktop screenshots. {e}"))?;
    let result =
        glib::future_with_timeout(Duration::from_secs(15), process.communicate_future(None)).await;
    let (stdout, stderr) = match result {
        Ok(result) => result.map_err(|e| format!("Screenshot capture failed: {e}"))?,
        Err(_) => {
            process.force_exit();
            return Err("Screenshot capture timed out.".into());
        }
    };
    if !process.is_successful() {
        let reason = stderr
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).trim().to_owned())
            .unwrap_or_default();
        return Err(format!("Could not capture the desktop with grim: {reason}"));
    }
    let bytes = stdout.ok_or("Screenshot capture returned no image")?;
    validate_png(&bytes)?;
    Ok(bytes.to_vec())
}

fn validate_png(bytes: &[u8]) -> Result<(), String> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Screenshot capture did not return a PNG image.".into());
    }
    Ok(())
}

pub fn save(directory: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let stamp = glib::DateTime::now_local()
        .and_then(|time| time.format("%Y-%m-%d_%H-%M-%S"))
        .map_err(|e| e.to_string())?;
    write_png(directory, bytes, &stamp)
}

fn write_png(directory: &Path, bytes: &[u8], stamp: &str) -> Result<PathBuf, String> {
    validate_png(bytes)?;
    fs::create_dir_all(directory)
        .map_err(|e| format!("Cannot create {}: {e}", directory.display()))?;
    for suffix in 0..10000 {
        let name = if suffix == 0 {
            format!("rsdraw-{stamp}.png")
        } else {
            format!("rsdraw-{stamp}-{suffix}.png")
        };
        let path = directory.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(format!("Cannot save {}: {e}", path.display()));
                }
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Cannot save {}: {e}", path.display())),
        }
    }
    Err("Could not find an unused screenshot filename.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saving_preserves_existing_files_and_rejects_capture_errors() {
        let directory =
            std::env::temp_dir().join(format!("rsdraw-save-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let original = directory.join("rsdraw-test.png");
        fs::write(&original, b"existing screenshot").unwrap();
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 2, 2).unwrap();
        let mut png = Vec::new();
        surface.write_to_png(&mut png).unwrap();
        let saved = write_png(&directory, &png, "test").unwrap();
        assert_ne!(saved, original);
        assert_eq!(fs::read(&original).unwrap(), b"existing screenshot");
        assert_eq!(fs::read(saved).unwrap(), png);
        assert!(write_png(&directory, b"capture error", "invalid").is_err());
        assert!(!directory.join("rsdraw-invalid.png").exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
