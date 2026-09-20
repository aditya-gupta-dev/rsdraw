#[cfg(not(target_os = "linux"))]
compile_error!("rsdraw is Linux-only (requires Linux and Wayland layer-shell protocol).");

mod app;
mod canvas;
mod config;
mod ipc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        let cfg = config::load();
        app::run(cfg);
    } else {
        match args[0].as_str() {
            "help" | "--help" | "-h" => {
                let cfg = config::load();
                config::print_help(&cfg);
            }
            _ => {
                let command = args.join(" ");
                match ipc::send_command(&command) {
                    Ok(resp) => println!("{}", resp),
                    Err(e) => {
                        eprintln!("error: {}", e);
                        eprintln!("Is rsdraw running? Start with: rsdraw");
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
