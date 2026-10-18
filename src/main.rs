#[cfg(not(target_os = "linux"))]
compile_error!("rsdraw requires Linux and a Wayland compositor with layer-shell support.");
mod app;
mod canvas;
mod config;
mod fonts;
mod ipc;
mod keys;
mod model;
mod rendering;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut path = config::config_path();
    if args.first().is_some_and(|s| s == "--config") {
        if args.len() < 2 {
            eprintln!("--config requires a path");
            std::process::exit(2);
        }
        path = std::path::PathBuf::from(args.remove(1));
        args.remove(0);
    }
    if args.first().is_some_and(|s| s == "config-path") {
        println!("{}", path.display());
        return;
    }
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h" | "check-config") {
        let cfg = config::load(&path).unwrap_or_else(|error| {
            eprintln!("configuration error in {}:\n{error}\nSee config/default.toml. Your file has not been overwritten.", path.display());
            std::process::exit(2)
        });
        if args.is_empty() {
            app::run(cfg);
        } else if args[0] == "check-config" {
            println!("OK {}", path.display());
        } else {
            config::print_help(&cfg, &path);
        }
    } else {
        match ipc::send_command(&args.join(" ")) {
            Ok(response) => {
                if response.starts_with("ERROR") {
                    eprintln!("{response}");
                    std::process::exit(1);
                }
                println!("{response}");
            }
            Err(e) => {
                eprintln!("error: {e}\nIs rsdraw running? Start with: rsdraw");
                std::process::exit(1);
            }
        }
    }
}
