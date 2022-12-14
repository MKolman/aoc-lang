use std::{
    fs,
    io::{self, Write},
};

use lang::{
    errors::{Error, LangError},
    interpreter::Env,
    runner,
};

fn main() {
    let mut args = std::env::args();
    if let Some(fname) = args.nth(1) {
        interpret(&fname);
    } else {
        terminal();
    }
}

fn interpret(fname: &str) {
    let code = &fs::read_to_string(fname).unwrap();
    let mut env = Env::default();
    let result = runner::run(code, &mut env);
    match &result {
        Ok(_) => {}
        Err(LangError::LexerError(err)) => show_error(code, "Lexer", err),
        Err(LangError::ParserError(err)) => show_error(code, "Parser", err),
        Err(LangError::RuntimeError(err)) => show_error(code, "Runtime", err),
    };
}

fn show_error(code: &str, err_prefix: &str, err: &Error) {
    eprintln!("{} error: {}", err_prefix, err);
    eprintln!("{}", code.lines().nth(err.0 .0.line).unwrap());
}

fn terminal() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut env = Env::default();
    loop {
        print!("> ");
        stdout.flush().unwrap();
        let mut code = String::new();
        stdin.read_line(&mut code).unwrap();
        if code == "exit" {
            break;
        }
        println!("{:?}", runner::run(&code, &mut env))
    }
}
