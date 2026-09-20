use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("rsdraw.sock")
}

pub fn send_command(cmd: &str) -> std::io::Result<String> {
    let mut stream = UnixStream::connect(socket_path())?;
    writeln!(stream, "{}", cmd)?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response.trim().to_string())
}

pub fn start_server(sender: std::sync::mpsc::Sender<String>) {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);

    std::thread::spawn(move || {
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("failed to bind IPC socket at {:?}: {}", path, e);
                return;
            }
        };

        for stream in listener.incoming().flatten() {
            let writer = match stream.try_clone() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut reader = BufReader::new(stream);
            let mut cmd = String::new();
            if reader.read_line(&mut cmd).is_ok() {
                let cmd = cmd.trim().to_string();
                if !cmd.is_empty() {
                    let _ = sender.send(cmd);
                }
            }
            let mut writer = writer;
            let _ = writeln!(writer, "OK");
        }
    });
}
