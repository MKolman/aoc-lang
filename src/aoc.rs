use std::rc::Rc;

use crate::{
    error,
    execute::Executor,
    parser::Parser,
    runtime::{Chunk, Value},
    scanner::Scanner,
};
use wasm_bindgen::prelude::*;

pub fn compile_and_run<W: std::io::Write>(code: &str, output: W) -> (Value, W) {
    let expr = match Parser::new(Scanner::new(code)).parse() {
        Ok(expr) => expr,
        Err(e) => {
            let mut output = output;
            dump_err(&mut output, e, code);
            return (Value::Nil, output);
        }
    };
    let chunk = match expr.to_chunk(Chunk::default()) {
        Ok(chunk) => chunk,
        Err(e) => {
            let mut output = output;
            dump_err(&mut output, e, code);
            return (Value::Nil, output);
        }
    };
    let mut ex = Executor::new(Rc::new(chunk));
    match ex.run(output) {
        Ok(value) => value,
        Err(e) => {
            let mut output = ex.output.take().unwrap();
            dump_err(&mut output, e, code);
            (Value::Nil, output)
        }
    }
}

fn dump_err<W: std::io::Write, K: error::Kind>(stdout: &mut W, err: error::Error<K>, code: &str) {
    writeln!(stdout, "{}", err).unwrap();
    write!(stdout, "{}", err.stack_trace(code)).unwrap();
}

#[wasm_bindgen]
pub fn run(code: &str) -> String {
    let (_value, stdout) = compile_and_run(code, Vec::new());
    String::from_utf8_lossy(&stdout).to_string()
}
