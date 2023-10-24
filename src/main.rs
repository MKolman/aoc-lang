use std::fs;

use aoc_lang::aoc::compile_and_run;

fn main() {
    let mut args = std::env::args();
    if let Some(fname) = args.nth(1) {
        let code = &fs::read_to_string(fname).expect("File not found");
        compile_and_run(code, std::io::stdout());
    } else {
        terminal();
    }
}

fn terminal() {
    // let stdin = io::stdin();
    // let mut stdout = io::stdout();
    // let mut env = Env::default();
    // loop {
    //     print!("> ");
    //     stdout.flush().unwrap();
    //     let mut code = String::new();
    //     stdin.read_line(&mut code).unwrap();
    //     if code == "exit" {
    //         break;
    //     }
    //     println!("{:?}", runner::run(&code, &mut env))
    // }
}
