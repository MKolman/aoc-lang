use std::{fs, rc::Rc};

use aoc_lang::{
    execute::Executor, parser::Parser, runtime::Chunk, scanner::Scanner, token::TokenType,
};

fn main() {
    let mut args = std::env::args();
    if let Some(fname) = args.nth(1) {
        let chunk = compile(&fname);
        eprintln!("{:?}", chunk.bytecode);
        let mut ex = Executor::new(Rc::new(chunk));
        ex.run(std::io::stdout());
    } else {
        terminal();
    }
}

fn compile(fname: &str) -> Chunk {
    let code = &fs::read_to_string(fname).expect("File not found");
    let s = Scanner::new(code);
    eprintln!(
        "{:?}",
        Scanner::new(code)
            .map(|t| t.kind)
            .collect::<Vec<TokenType>>()
    );
    let expr = Parser::new(s).parse();
    eprintln!("{:?}", expr);
    expr.to_chunk(Chunk::default())
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
