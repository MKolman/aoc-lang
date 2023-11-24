use std::fs;

use aoc_lang::aoc::{compile_and_run, debug_run};

fn parse_args() -> (String, bool) {
    let mut args = std::env::args();
    let mut debug = false;
    let mut fname = args.nth(1).expect("No file name provided");
    if fname == "--debug" {
        debug = true;
        fname = args.next().expect("No file name provided");
    } else if args.next() == Some("--debug".into()) {
        debug = true;
    }
    (fname, debug)
}
fn main() {
    let (fname, debug) = parse_args();
    let code = &fs::read_to_string(fname).expect("File not found");
    if debug {
        debug_run(code, &mut std::io::stdout());
    } else {
        compile_and_run(code, &mut std::io::stdout());
    }
}
