use std::fmt::Display;

use crate::token::{Pos, Snippet};

pub type Result<T, E> = std::result::Result<T, Error<E>>;

pub trait Stackable {
    fn stack(self, pos: Pos, code: &str) -> Self;
    fn context(self, context: &str) -> Self;
    fn wrap(self, context: &str, pos: Pos, code: &str) -> Self
    where
        Self: Sized,
    {
        self.stack(pos, code).context(context)
    }
}

impl<T, E: Stackable> Stackable for std::result::Result<T, E> {
    fn stack(self, pos: Pos, code: &str) -> Self {
        self.map_err(|e| e.stack(pos, code))
    }
    fn context(self, context: &str) -> Self {
        self.map_err(|e| e.context(context))
    }
}

pub trait Kind: std::fmt::Debug + Default {}

#[derive(Debug, Default)]
pub struct SyntaxError;
impl Kind for SyntaxError {}
#[derive(Debug, Default)]
pub struct RuntimeError;
impl Kind for RuntimeError {}
#[derive(Debug, Default)]
pub struct ParserError;
impl Kind for ParserError {}

#[derive(Debug)]
pub struct Error<E: Kind> {
    underlying: Option<Box<dyn std::error::Error>>,
    kind: E,
    context: String,
    stack: Vec<Snippet>,
}

impl<E: Kind> Error<E> {
    pub fn from<Err: std::error::Error + 'static>(e: Err) -> Self {
        Self {
            underlying: Some(Box::new(e)),
            kind: E::default(),
            context: String::new(),
            stack: Vec::new(),
        }
    }
    pub fn new(context: String) -> Self {
        Self {
            underlying: None,
            kind: E::default(),
            context,
            stack: Vec::new(),
        }
    }

    pub fn build(context: String, pos: Pos, code: &str) -> Self {
        Self {
            underlying: None,
            kind: E::default(),
            context,
            stack: vec![pos.extract(code)],
        }
    }

    pub fn stack_trace(&self) -> String {
        self.stack
            .iter()
            .map(
                |Snippet {
                     line,
                     col,
                     line_prefix,
                     snippet,
                     line_suffix,
                 }| {
                    format!("on line {line}:{col}: {line_prefix}\x1b[91m\x1b[1m{snippet}\x1b[0m{line_suffix}")
                },
            )
            .collect::<Vec<_>>()
            .join("\n")
    }
}
impl<T: Kind> Stackable for Error<T> {
    fn stack(mut self, pos: Pos, code: &str) -> Self {
        self.stack.push(pos.extract(code));
        self
    }
    fn context(mut self, context: &str) -> Self {
        self.context = format!("{}: {}", context, self.context);
        self
    }
}
impl<T: Kind> From<String> for Error<T> {
    fn from(context: String) -> Self {
        Self::new(context)
    }
}

impl<T: Kind> Display for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{:?}: {}", self.kind, self.context)?;
        if let Some(e) = &self.underlying {
            writeln!(f, "{}", e)?;
        };
        write!(f, "{}", self.stack_trace())?;
        Ok(())
    }
}

impl<T: Kind> std::error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.underlying {
            Some(e) => Some(e.as_ref()),
            None => None,
        }
    }
}
