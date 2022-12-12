use crate::{
    errors::LangError,
    interpreter::{Env, ExprValue},
    lexer::Tokenizer,
    parser::Parser,
};

pub fn run(code: &str, env: &mut Env) -> Result<Vec<ExprValue>, LangError> {
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
