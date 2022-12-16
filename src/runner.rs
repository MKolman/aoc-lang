use std::io::Write;
use wasm_bindgen::prelude::*;

use crate::{
    errors::LangError,
    interpreter::{Env, ExprValue},
    lexer::Tokenizer,
    parser::Parser,
};

pub fn run<W: Write>(code: &str, env: &mut Env<W>) -> Result<Vec<ExprValue>, LangError> {
    let tokens = Tokenizer::new(code)
        .tokenize()
        .map_err(LangError::LexerError)?;
    let nodes = Parser::new(&tokens)
        .parse()
        .map_err(LangError::ParserError)?;
    nodes
        .into_iter()
        .map(|e| e.eval(env).map_err(LangError::RuntimeError))
        .collect()
}

#[derive(Debug, Default)]
struct StringWriter(pub String);
impl Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += std::str::from_utf8(buf)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen]
pub fn interpret(code: &str) -> Result<String, String> {
    let mut output = StringWriter::default();
    let result = run(code, &mut Env::new(&mut output));
    if let Err(e) = result {
        return Err(format!("{:?}", e));
    }
    Ok(output.0)
}
