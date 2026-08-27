use std::env;
use std::process::exit;

mod release;

const USAGE: &str = "Usage: cargo xtask <prepare-release|tag-release> [args...]";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        exit(1);
    }

    match args[1].as_str() {
        "prepare-release" => release::run_prepare(&args[2..]),
        "tag-release" => release::run_tag(&args[2..]),
        other => {
            eprintln!("Unknown command: {other}");
            eprintln!("{USAGE}");
            exit(1);
        }
    }
}
