use std::{collections::HashSet, str::FromStr};

use crate::errors::{self, Error, Loc, Pos};

#[derive(Debug)]
pub struct Token(pub TokenValue, pub Loc);
impl Token {
    fn new(value: TokenValue, start_pos: Pos, end_pos: Pos) -> Self {
        Token(value, Loc(start_pos, end_pos))
    }
}
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenValue {
    Number(i64),
    Comma,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Operator(Operator),
    Keyword(Keyword),
    Identifier(String),
    EOL,
    EOF,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Operator {
    Not,
    Equal,
    NotEq,
    Less,
    LessEq,
    More,
    MoreEq,

    And,
    Or,
    XOr,

    Assign,

    Sum,
    Sub,
    Div,
    Mul,
    Mod,
}

impl FromStr for Operator {
    type Err = String;
    fn from_str(c: &str) -> Result<Operator, Self::Err> {
        match c {
            "&" => Ok(Operator::And),
            "|" => Ok(Operator::Or),
            "^" => Ok(Operator::XOr),
            "!" => Ok(Operator::Not),
            "==" => Ok(Operator::Equal),
            "!=" => Ok(Operator::Equal),
            "<=" => Ok(Operator::LessEq),
            "<" => Ok(Operator::Less),
            ">=" => Ok(Operator::MoreEq),
            ">" => Ok(Operator::More),
            "+" => Ok(Operator::Sum),
            "-" => Ok(Operator::Sub),
            "/" => Ok(Operator::Div),
            "*" => Ok(Operator::Mul),
            "%" => Ok(Operator::Mod),
            "=" => Ok(Operator::Assign),
            _ => Err(format!("Unknown operator {}", c)),
        }
    }
}

impl Operator {
    pub fn all_bin() -> Vec<HashSet<Operator>> {
        vec![
            // Logical
            HashSet::from([Operator::Or]),
            HashSet::from([Operator::XOr]),
            HashSet::from([Operator::And]),
            // Comparison
            HashSet::from([
                Operator::Less,
                Operator::LessEq,
                Operator::More,
                Operator::MoreEq,
                Operator::Equal,
                Operator::NotEq,
            ]),
            // Sum
            HashSet::from([Operator::Sum, Operator::Sub]),
            // Mul
            HashSet::from([Operator::Mul, Operator::Div, Operator::Mod]),
        ]
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Keyword {
    If,
    For,
    While,
    Print,
    Read,
    Func,
}
impl Keyword {
    fn new(v: &str) -> Option<Keyword> {
        match v {
            "if" => Some(Keyword::If),
            "for" => Some(Keyword::For),
            "while" => Some(Keyword::While),
            "print" => Some(Keyword::Print),
            "read" => Some(Keyword::Read),
            "fn" => Some(Keyword::Func),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Tokenizer<'a> {
    code: std::iter::Peekable<std::str::Chars<'a>>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(code: &str) -> Tokenizer {
        Tokenizer {
            code: code.chars().peekable(),
            pos: 0,
            line: 0,
            col: 0,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.code.next()?;
        self.pos += 1;
        self.col += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 0;
        }
        Some(c)
    }

    fn pos(&self) -> Pos {
        Pos {
            pos: self.pos,
            line: self.line,
            col: self.col,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, errors::Error> {
        let mut tokens = Vec::new();
        while let Some(&c) = self.code.peek() {
            match c {
                c if c.to_string().parse::<Operator>().is_ok() => {
                    tokens.push(self.parse_operator()?)
                }
                '(' | ')' | '{' | '}' | '[' | ']' | ',' => {
                    self.advance();
                    tokens.push(Token::new(
                        match c {
                            ',' => TokenValue::Comma,
                            '(' => TokenValue::OpenParen,
                            ')' => TokenValue::CloseParen,
                            '{' => TokenValue::OpenBrace,
                            '}' => TokenValue::CloseBrace,
                            '[' => TokenValue::OpenBracket,
                            ']' => TokenValue::CloseBracket,
                            _ => {
                                return Err(Error::new(Loc(self.pos(), self.pos()), "Wat?".into()))
                            }
                        },
                        self.pos(),
                        self.pos(),
                    ));
                }
                '0'..='9' => tokens.push(self.parse_number()?),
                'a'..='z' | 'A'..='Z' | '_' => tokens.push(self.parse_identifier_or_keyword()?),
                ';' | '\n' => {
                    self.advance();
                    tokens.push(Token::new(TokenValue::EOL, self.pos(), self.pos()));
                }
                ' ' | '\t' => {
                    self.advance();
                }
                _ => {
                    return Err(errors::Error::new(
                        Loc(self.pos(), self.pos()),
                        format!("cannot parse character '{}'", c),
                    ));
                }
            }
        }
        tokens.push(Token::new(TokenValue::EOF, self.pos(), self.pos()));
        Ok(tokens)
    }

    fn parse_identifier_or_keyword(&mut self) -> Result<Token, errors::Error> {
        let start = self.pos();
        let mut value = String::new();
        while let Some('a'..='z' | 'A'..='Z' | '_') = self.code.peek() {
            value.push(self.advance().expect("Peek was Some"));
        }

        let t_value = if let Some(keyword) = Keyword::new(&value) {
            TokenValue::Keyword(keyword)
        } else {
            TokenValue::Identifier(value)
        };

        Ok(Token::new(t_value, start, self.pos()))
    }

    fn parse_number(&mut self) -> Result<Token, errors::Error> {
        let start = self.pos();
        let zero = '0' as i64;
        let mut value = 0;
        while let Some('0'..='9') = self.code.peek() {
            value *= 10;
            value += self.advance().expect("Peek was Some") as i64 - zero;
        }
        Ok(Token::new(TokenValue::Number(value), start, self.pos()))
    }

    fn parse_operator(&mut self) -> Result<Token, errors::Error> {
        let op = if let Some(c) = self.advance() {
            let mut tmp = c
                .to_string()
                .parse()
                .map_err(|msg| Error::new(Loc(self.pos(), self.pos()), msg))?;

            if let Some(k) = self.code.peek() {
                if let Ok(op) = format!("{}{}", c, k).parse() {
                    self.advance();
                    tmp = op;
                }
            }
            Ok(tmp)
        } else {
            Err(Error::new(
                Loc(self.pos(), self.pos()),
                "No character to parse".into(),
            ))
        };
        Ok(Token::new(
            TokenValue::Operator(op?),
            self.pos(),
            self.pos(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_simple() {
        let tokens = Tokenizer::new("a = 5*3+(1/2)")
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.0)
            .collect::<Vec<TokenValue>>();
        let want = vec![
            TokenValue::Identifier("a".into()),
            TokenValue::Operator(Operator::Assign),
            TokenValue::Number(5),
            TokenValue::Operator(Operator::Mul),
            TokenValue::Number(3),
            TokenValue::Operator(Operator::Sum),
            TokenValue::OpenParen,
            TokenValue::Number(1),
            TokenValue::Operator(Operator::Div),
            TokenValue::Number(2),
            TokenValue::CloseParen,
            TokenValue::EOF,
        ];
        assert_eq!(tokens.len(), want.len());
        for (a, b) in tokens.iter().zip(want) {
            assert_eq!(*a, b);
        }
    }
}
