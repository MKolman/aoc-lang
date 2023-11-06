use std::ops::Add;

use crate::expr::Operator;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Pos {
    pub start: usize,
    pub end: usize,
}

pub struct Snippet<'s> {
    pub line: usize,
    pub col: usize,
    pub line_prefix: &'s str,
    pub snippet: &'s str,
    pub line_suffix: &'s str,
}

impl Pos {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn extract<'c>(&self, code: &'c str) -> Snippet<'c> {
        let line_start = code[..self.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = code[self.end..].find('\n').unwrap_or(code.len() - self.end) + self.end;
        Snippet {
            line: code[..self.start].matches('\n').count() + 1,
            col: self.start - line_start,
            line_prefix: &code[line_start..self.start],
            snippet: &code[self.start..self.end],
            line_suffix: &code[self.end..line_end],
        }
    }
}

impl Add<Pos> for Pos {
    type Output = Pos;
    fn add(self, rhs: Pos) -> Pos {
        Pos::new(self.start, rhs.end)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Identifier(String),
    Nil,
    // Keywords
    Return,
    If,
    Else,
    For,
    Print,
    Read,
    While,
    Func,
    // Parenthesis
    LParen,
    RParen,
    OBrace,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    // Comparison
    EqEq,
    BangEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    // Logical operators
    Bang,
    And,
    AndAnd,
    Pipe,
    PipePipe,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    // End
    EOL,
    EOF,
    // Misc
    Eq,
    Comma,
    Dot,
    Comment(String),
    // Error
    Unexpected(char),
}

impl TokenType {
    pub fn keyword_or_identifier(v: &str) -> Self {
        match v {
            "if" => Self::If,
            "else" => Self::Else,
            "for" => Self::For,
            "while" => Self::While,
            "print" => Self::Print,
            "read" => Self::Read,
            "fn" => Self::Func,
            "nil" => Self::Nil,
            "return" => Self::Return,
            v => Self::Identifier(v.to_string()),
        }
    }

    pub fn to_operator(&self) -> Option<Operator> {
        let op = match self {
            TokenType::Plus => Operator::Add,
            TokenType::Minus => Operator::Sub,
            TokenType::Star => Operator::Mul,
            TokenType::Slash => Operator::Div,
            TokenType::Percent => Operator::Mod,
            TokenType::AndAnd => Operator::And,
            TokenType::PipePipe => Operator::Or,
            TokenType::EqEq => Operator::Eq,
            TokenType::BangEq => Operator::Neq,
            TokenType::Bang => Operator::Not,
            TokenType::Less => Operator::Less,
            TokenType::LessEq => Operator::LessEq,
            TokenType::Greater => Operator::Greater,
            TokenType::GreaterEq => Operator::GreaterEq,
            _ => return None,
        };
        Some(op)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub pos: Pos,
    pub kind: TokenType,
}

impl Token {
    pub fn new(start: usize, end: usize, kind: TokenType) -> Self {
        Self {
            pos: Pos::new(start, end),
            kind,
        }
    }
}
