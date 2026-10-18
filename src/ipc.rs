use std::{
    io::{self, BufRead, BufReader, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

pub struct Request {
    pub command: String,
    pub response: mpsc::Sender<String>,
}
pub fn socket_path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // A per-user fallback avoids sharing /tmp/rsdraw.sock across users.
            PathBuf::from("/tmp").join(format!("rsdraw-{}", unsafe { getuid() }))
        });
    base.join("rsdraw.sock")
}
unsafe extern "C" {
    fn getuid() -> u32;
}
pub fn send_command(cmd: &str) -> io::Result<String> {
    let mut stream = UnixStream::connect(socket_path())?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    writeln!(stream, "{cmd}")?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    if response.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "daemon closed without a response",
        ));
    }
    Ok(response.trim().into())
}
pub fn start_server(sender: mpsc::Sender<Request>) -> io::Result<()> {
    let path = socket_path();
    if UnixStream::connect(&path).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "rsdraw is already running",
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            let mut command = String::new();
            let Ok(reader) = stream.try_clone() else {
                continue;
            };
            use std::io::Read;
            if BufReader::new(reader)
                .take(8192)
                .read_line(&mut command)
                .is_err()
            {
                continue;
            }
            let (tx, rx) = mpsc::channel();
            if sender
                .send(Request {
                    command: command.trim().into(),
                    response: tx,
                })
                .is_err()
            {
                break;
            }
            let response = rx
                .recv_timeout(Duration::from_secs(4))
                .unwrap_or_else(|_| "ERROR daemon did not respond".into());
            let _ = writeln!(stream, "{response}");
        }
    });
    Ok(())
}
