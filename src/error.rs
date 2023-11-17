use std::fmt::Display;

use crate::token::Pos;

pub type Result<T, E> = std::result::Result<T, Error<E>>;

pub trait Stackable {
    fn stack(self, pos: Pos) -> Self;
    fn context(self, context: &str) -> Self;
    fn wrap(self, context: &str, pos: Pos) -> Self
    where
        Self: Sized,
    {
        self.stack(pos).context(context)
    }
}

impl<T, E: Stackable> Stackable for std::result::Result<T, E> {
    fn stack(self, pos: Pos) -> Self {
        self.map_err(|e| e.stack(pos))
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
    stack: Vec<Pos>,
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

    pub fn build(context: String, pos: Pos) -> Self {
        Self {
            underlying: None,
            kind: E::default(),
            context,
            stack: vec![pos],
        }
    }

    pub fn stack_trace(&self, code: &str) -> String {
        let mut result = String::new();
        for pos in &self.stack {
            let crate::token::Snippet {
                line,
                col,
                line_prefix,
                snippet,
                line_suffix,
            } = pos.extract(code);
            result.push_str(&format!(
                "on line {line}:{col}: {line_prefix}\x1b[91m\x1b[1m{snippet}\x1b[0m{line_suffix}\n",
            ));
        }
        result
    }
}
impl<T: Kind> Stackable for Error<T> {
    fn stack(mut self, pos: Pos) -> Self {
        self.stack.push(pos);
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
        write!(f, "{:?}: {}", self.kind, self.context)?;
        if let Some(e) = &self.underlying {
            write!(f, "\n{}", e)?;
        };
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
