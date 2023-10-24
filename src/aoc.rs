use std::rc::Rc;

use crate::{
    execute::Executor,
    parser::Parser,
    runtime::{Chunk, Value},
    scanner::Scanner,
};

pub fn compile_and_run<W: std::io::Write>(code: &str, output: W) -> (Value, W) {
    let mut ex = Executor::new(Rc::new(
        Parser::new(Scanner::new(code))
            .parse()
            .to_chunk(Chunk::default()),
    ));
    ex.run(output)
}
