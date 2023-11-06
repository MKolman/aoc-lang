use std::rc::Rc;

use crate::{
    execute::Executor,
    parser::Parser,
    runtime::{Chunk, Value},
    scanner::Scanner,
};
use wasm_bindgen::prelude::*;

pub fn compile_and_run<W: std::io::Write>(code: &str, output: W) -> (Value, W) {
    let mut ex = Executor::new(Rc::new(
        Parser::new(Scanner::new(code))
            .parse()
            .to_chunk(Chunk::default()),
    ));
    match ex.run(output) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("{}", e);
            eprint!("{}", e.stack_trace(code));
            (Value::Nil, ex.output.take().unwrap())
        }
    }
}

#[wasm_bindgen]
pub fn run(code: &str) -> String {
    let (_value, output) = compile_and_run(code, Vec::new());
    String::from_utf8_lossy(&output).to_string()
}
