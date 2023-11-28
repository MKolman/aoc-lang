use std::fs;

use aoc_lang::aoc::{compile_and_run, debug_run};

#[derive(Debug, Default)]
struct Args {
    name: String,
    debug: bool,
    version: bool,
    help: bool,
    fnames: Vec<String>,
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut cli_args = std::env::args();
    args.name = cli_args.next().unwrap();
    for arg in cli_args {
        match arg.as_str() {
            "--debug" | "-d" => args.debug = true,
            "--version" | "-v" => args.version = true,
            "--help" | "-h" => args.help = true,
            _ => args.fnames.push(arg),
        }
    }
    args
}
fn main() {
    let args = parse_args();
    if args.version {
        println!("{} v{}", args.name, env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.help {
        println!("Usage: {} [options] [file ...]", args.name);
        println!("Options:");
        println!("  -d, --debug     Run in debug mode");
        println!("  -v, --version   Print version and exit");
        println!("  -h, --help      Print this help and exit");
        return;
    }
    if args.fnames.is_empty() {
        println!(
            "No input files provided. For help run:\n\t{} --help",
            args.name
        );
        return;
    }
    let runner: fn(&str) -> aoc_lang::runtime::Value = if args.debug {
        |code| debug_run(code, &mut std::io::stdout())
    } else {
        |code| compile_and_run(code, &mut std::io::stdout())
    };
    for fname in &args.fnames {
        let code = &fs::read_to_string(fname).expect("File not found");
        runner(code);
    }
}
