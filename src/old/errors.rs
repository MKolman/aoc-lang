use std::fmt;

pub type Fail<T> = Result<T, Error>;

#[derive(Debug)]
pub enum LangError {
    LexerError(Error),
    ParserError(Error),
    RuntimeError(Error),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Default)]
pub struct Pos {
    pub pos: usize,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Loc(pub Pos, pub Pos);

impl std::ops::Add<Loc> for Loc {
    type Output = Loc;
    fn add(self, o: Loc) -> Loc {
        Loc(std::cmp::min(self.0, o.0), std::cmp::max(self.1, o.1))
    }
}

#[derive(Debug)]
pub struct Error(pub Loc, String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 .0.line == self.0 .1.line {
            write!(
                f,
                "Error on line {} column {}: {}",
                self.0 .0.line, self.0 .0.col, self.1
            )?
        }
        Ok(())
    }
}

impl Error {
    pub fn new(loc: Loc, msg: String) -> Error {
        Error(loc, msg)
    }

    pub fn eof() -> Error {
        Error::new(Loc::default(), "Unexpected end of input".into())
    }

    pub fn loc(self, loc: Loc) -> Self {
        Error::new(
            if self.0 == Loc::default() {
                loc
            } else {
                self.0
            },
            self.1,
        )
    }
}
