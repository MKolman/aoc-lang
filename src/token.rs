use std::ops::Add;

use crate::expr::Operator;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Pos {
    start: usize,
    end: usize,
}

impl Pos {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
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
    If,
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
            "for" => Self::For,
            "while" => Self::While,
            "print" => Self::Print,
            "read" => Self::Read,
            "fn" => Self::Func,
            "nil" => Self::Nil,
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
