use std::{iter::Peekable, str::CharIndices};

use crate::token::{Token, TokenType};

pub struct Scanner<'a> {
    input: &'a str,
    iter: Peekable<CharIndices<'a>>,
    eof: bool,
}

impl<'a> Scanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            iter: input.char_indices().peekable(),
            eof: false,
        }
    }

    fn get_token(&mut self) -> Token {
        loop {
            let Some(&(_, c)) = self.iter.peek() else {
                return Token::new(self.input.len(), self.input.len(), TokenType::EOF);
            };
            return match c {
                '&' => self.one_or_two('&', TokenType::And, TokenType::AndAnd),
                '|' => self.one_or_two('|', TokenType::Pipe, TokenType::PipePipe),
                '!' => self.one_or_two('=', TokenType::Bang, TokenType::BangEq),
                '=' => self.one_or_two('=', TokenType::Eq, TokenType::EqEq),
                '<' => self.one_or_two('=', TokenType::Less, TokenType::LessEq),
                '>' => self.one_or_two('=', TokenType::Greater, TokenType::GreaterEq),
                '{' => self.one_or_two('=', TokenType::LBrace, TokenType::OBrace),
                '}' => self.one(TokenType::RBrace),
                '(' => self.one(TokenType::LParen),
                ')' => self.one(TokenType::RParen),
                '[' => self.one(TokenType::LBracket),
                ']' => self.one(TokenType::RBracket),
                '+' => self.one(TokenType::Plus),
                '-' => self.one(TokenType::Minus),
                '*' => self.one(TokenType::Star),
                '/' => self.one(TokenType::Slash),
                '%' => self.one(TokenType::Percent),
                '\n' | ';' => self.one(TokenType::EOL),
                ',' => self.one(TokenType::Comma),
                '.' => self.one(TokenType::Dot),
                'a'..='z' | 'A'..='Z' | '_' => self.keyword_or_identifier(),
                '0'..='9' => self.number(),
                '#' => self.comment(),
                '"' => self.string(),
                ' ' | '\t' => {
                    self.iter.next().expect("");
                    continue;
                }
                c => self.one(TokenType::Unexpected(c)),
            };
        }
    }

    fn one(&mut self, kind: TokenType) -> Token {
        let (start, c) = self.iter.next().expect("Needs one character");
        Token::new(start, start + c.len_utf8(), kind)
    }

    fn one_or_two(&mut self, if_char: char, if_one: TokenType, if_two: TokenType) -> Token {
        let (start, first) = self.iter.next().expect("Needs one character");
        if matches!(self.iter.peek(), Some((_, c)) if c == &if_char) {
            let (end, second) = self.iter.next().expect("peek() was Some");
            return Token::new(start, end + second.len_utf8(), if_two);
        }
        Token::new(start, start + first.len_utf8(), if_one)
    }

    fn comment(&mut self) -> Token {
        let &(start, mut last) = self.iter.peek().expect("Needs one character");
        let mut end = 0;

        while matches!(self.iter.peek(), Some(&(_, c)) if c != '\n') {
            (end, last) = self.iter.next().expect("peek() was Some");
        }
        end += last.len_utf8();
        Token::new(
            start,
            end,
            TokenType::Comment(self.input[start..end].to_string()),
        )
    }

    fn keyword_or_identifier(&mut self) -> Token {
        let (start, mut last) = self.iter.next().expect("Needs one character");
        let mut end = start;
        while matches!(
            self.iter.peek(),
            Some((_, '0'..='9' | 'a'..='z' | 'A'..='Z' | '_'))
        ) {
            (end, last) = self.iter.next().expect("peek() was Some");
        }
        end += last.len_utf8();
        Token::new(
            start,
            end,
            TokenType::keyword_or_identifier(&self.input[start..end]),
        )
    }

    fn number(&mut self) -> Token {
        let &(start, mut last) = self.iter.peek().expect("Needs one character");
        let mut end = 0;
        let mut dot = false;
        while let Some((_, c)) = self.iter.peek() {
            if !(('0'..='9').contains(c) || (!dot && c == &'.')) {
                break;
            }
            (end, last) = self.iter.next().expect("peek() was Some");
            dot |= last == '.';
        }
        end += last.len_utf8();
        let num = &self.input[start..end];
        if dot {
            Token::new(
                start,
                end,
                TokenType::Float(num.parse().expect("Only contains digits and one dot.")),
            )
        } else {
            Token::new(
                start,
                end,
                TokenType::Integer(num.parse().expect("Only contains digits.")),
            )
        }
    }

    fn string(&mut self) -> Token {
        let (start, _) = self.iter.next().expect("Strings start with '\"'");
        let mut res = String::new();
        while matches!(self.iter.peek(), Some(&(_, c)) if c != '"') {
            let (_, c) = self.iter.next().expect("peek() was Some");
            res.push(c);
        }
        let (end, _) = self.iter.next().expect("Strings end with '\"'");
        Token::new(start, end, TokenType::String(res))
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.get_token();
        let is_eof = token.kind == TokenType::EOF;
        if is_eof && self.eof {
            return None;
        }

        self.eof |= is_eof;
        Some(token)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn arithmetic() {
        let s = Scanner::new("a = 12 + 3.5 * 12 / 0.1");
        assert_eq!(
            s.map(|t| t.kind).collect::<Vec<_>>(),
            vec![
                TokenType::Identifier("a".to_string()),
                TokenType::Eq,
                TokenType::Integer(12),
                TokenType::Plus,
                TokenType::Float(3.5),
                TokenType::Star,
                TokenType::Integer(12),
                TokenType::Slash,
                TokenType::Float(0.1),
                TokenType::EOF,
            ]
        );
    }

    #[test]
    fn comment() {
        let s = Scanner::new("if for while print fn # test comment\n\tread");
        assert_eq!(
            s.map(|t| t.kind).collect::<Vec<_>>(),
            vec![
                TokenType::If,
                TokenType::For,
                TokenType::While,
                TokenType::Print,
                TokenType::Func,
                TokenType::Comment("# test comment".to_string()),
                TokenType::EOL,
                TokenType::Read,
                TokenType::EOF,
            ]
        );
    }
}
