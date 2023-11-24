use std::rc::Rc;

use crate::{
    error,
    interpreter::Interpreter,
    parser::Parser,
    runtime::{Chunk, Value},
    scanner::Scanner,
};
use wasm_bindgen::prelude::*;

pub fn compile_and_run<W: std::io::Write>(code: &str, mut output: W) -> Value {
    let tokens = Scanner::new(code);
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
    let tokens = Scanner::new(code);
    writeln!(
        output,
        "=== Tokens ===\n{:?}",
        tokens
            .clone()
            .into_iter()
            .map(|t| t.kind)
            .collect::<Vec<_>>()
    )
    .unwrap();
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
    writeln!(output, "=== Runtime ===").unwrap();
    writeln!(output, "=== Constants ===").unwrap();
    for (i, c) in chunk.constants.iter().enumerate() {
        writeln!(output, "{i}: {c}").unwrap();
    }
    writeln!(output, "=== Variables ===").unwrap();
    chunk
        .var_names
        .iter()
        .zip(chunk.captured_vars.iter())
        .enumerate()
        .for_each(|(i, (s, c))| writeln!(output, "{i}: {s:?} ({c:?})").unwrap());
    writeln!(output, "=== Bytecode ===").unwrap();
    chunk
        .bytecode
        .iter()
        .for_each(|op| writeln!(output, "{:?}", op).unwrap());
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
