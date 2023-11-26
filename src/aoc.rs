use std::rc::Rc;

use crate::{
    error,
    interpreter::Interpreter,
    lexer::Lexer,
    parser::Parser,
    runtime::{Chunk, Value},
};
use wasm_bindgen::prelude::*;

pub fn compile_and_run<W: std::io::Write>(code: &str, mut output: W) -> Value {
    let tokens = Lexer::new(code);
    let expr = match Parser::new(tokens).parse() {
        Ok(expr) => expr,
        Err(e) => {
            dump_err(output, e, code);
            return Value::Nil;
        }
    };
    let chunk = match expr.to_chunk(Chunk::default()) {
        Ok(chunk) => chunk,
        Err(e) => {
            dump_err(output, e, code);
            return Value::Nil;
        }
    };
    let mut ex = Interpreter::new(Rc::new(chunk), &mut output);
    match ex.run() {
        Ok(value) => value,
        Err(e) => {
            dump_err(output, e, code);
            Value::Nil
        }
    }
}

fn dump_err<W: std::io::Write, K: error::Kind>(mut stdout: W, err: error::Error<K>, code: &str) {
    writeln!(stdout, "=== Stderr ===").unwrap();
    writeln!(stdout, "{}", err).unwrap();
    write!(stdout, "{}", err.stack_trace(code)).unwrap();
}

#[wasm_bindgen]
pub fn run(code: &str, debug: bool) -> String {
    let mut stdout = Vec::new();
    if debug {
        debug_run(code, &mut stdout);
    } else {
        compile_and_run(code, &mut stdout);
    }
    String::from_utf8_lossy(&stdout).to_string()
}

pub fn debug_run<W: std::io::Write>(code: &str, mut output: W) -> (Value, W) {
    let tokens = Lexer::new(code);
    writeln!(output, "=== Tokens ===").unwrap();
    tokens
        .clone()
        .into_iter()
        .for_each(|t| writeln!(output, "{:?}", t.kind).unwrap());
    let expr = match Parser::new(tokens).parse() {
        Ok(expr) => expr,
        Err(e) => {
            let mut output = output;
            dump_err(&mut output, e, code);
            return (Value::Nil, output);
        }
    };
    writeln!(output, "=== Expression ===\n{:#?}", expr).unwrap();
    let chunk = match expr.to_chunk(Chunk::default()) {
        Ok(chunk) => chunk,
        Err(e) => {
            let mut output = output;
            dump_err(&mut output, e, code);
            return (Value::Nil, output);
        }
    };
    write!(output, "=== Runtime ===\n{chunk}").unwrap();
    writeln!(output, "=== Stdout ===").unwrap();
    let mut ex = Interpreter::new(Rc::new(chunk), &mut output);
    ex.set_debug(true);
    match ex.run() {
        Ok(value) => (value, output),
        Err(e) => {
            dump_err(&mut output, e, code);
            (Value::Nil, output)
        }
    }
}
