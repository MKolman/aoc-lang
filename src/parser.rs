use crate::{
    errors::{Error, Fail, Loc},
    lexer::{Keyword, Operator, Token, TokenValue},
};

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expr {
    Number(i64),
    Identifier(String),
    BinaryOp(Operator, Box<ExprMeta>, Box<ExprMeta>),
    UnaryOp(Operator, Box<ExprMeta>),
    Assign(Box<ExprMeta>, Box<ExprMeta>),
    Block(Vec<ExprMeta>),
    Print(Box<ExprMeta>),
    Read,
    If(Box<ExprMeta>, Box<ExprMeta>),
    While(Box<ExprMeta>, Box<ExprMeta>),
    For(Box<ExprMeta>, Box<ExprMeta>),
    FnDef(Vec<String>, Box<ExprMeta>),
    FnCall(Box<ExprMeta>, Vec<ExprMeta>),
    VecDef(Vec<ExprMeta>),
    VecGet(Box<ExprMeta>, Vec<ExprMeta>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExprMeta(pub Expr, pub Loc);

pub struct Parser<'a> {
    tokens: std::iter::Peekable<std::slice::Iter<'a, Token>>,
}
impl<'a> Parser<'a> {
    pub fn new(tokens: &[Token]) -> Parser {
        Parser {
            tokens: tokens.iter().peekable(),
        }
    }

    pub fn parse(&mut self) -> Fail<Vec<ExprMeta>> {
        let mut result = Vec::new();
        while let Some(token) = self.tokens.peek() {
            match token.0 {
                TokenValue::EOL => {
                    self.tokens.next();
                    continue;
                }
                TokenValue::EOF => break,
                _ => result.push(self.parse_single()?),
            }
        }
        Ok(result)
    }

    fn parse_single(&mut self) -> Fail<ExprMeta> {
        while self.check(&TokenValue::EOL) {
            self.consume(&TokenValue::EOL)?;
        }
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Fail<ExprMeta> {
        let mut left = self.parse_vec_get()?;
        if self.check(&TokenValue::Operator(Operator::Assign)) {
            self.consume(&TokenValue::Operator(Operator::Assign))?;
            let right = self.parse_assignment()?;
            let loc = left.1 + right.1;
            left = ExprMeta(Expr::Assign(Box::new(left), Box::new(right)), loc)
        }
        Ok(left)
    }

    fn parse_vec_get(&mut self) -> Fail<ExprMeta> {
        let mut left = self.parse_fn_call()?;
        while self.check(&TokenValue::OpenBracket) {
            let start_loc = self.consume(&TokenValue::OpenBracket)?;
            let args = self.parse_comma_sep_values(&TokenValue::CloseBracket)?;
            let end_loc = self.consume(&TokenValue::CloseBracket)?;
            left = ExprMeta(Expr::VecGet(Box::new(left), args), start_loc + end_loc);
        }
        Ok(left)
    }

    fn parse_fn_call(&mut self) -> Fail<ExprMeta> {
        let mut left = self.parse_binary_op(0)?;
        while self.check(&TokenValue::OpenParen) {
            let start_loc = self.consume(&TokenValue::OpenParen)?;
            let args = self.parse_comma_sep_values(&TokenValue::CloseParen)?;
            let end_loc = self.consume(&TokenValue::CloseParen)?;
            left = ExprMeta(Expr::FnCall(Box::new(left), args), start_loc + end_loc);
        }
        Ok(left)
    }

    fn parse_binary_op(&mut self, idx: usize) -> Fail<ExprMeta> {
        if let Some(bin_ops) = Operator::all_bin().get(idx) {
            let mut left = self.parse_binary_op(idx + 1)?;
            let start_loc = left.1;
            while let Some(Token(TokenValue::Operator(op), _)) = self.tokens.peek() {
                if bin_ops.contains(op) {
                    self.tokens.next();
                    let right = self.parse_binary_op(idx)?;
                    let loc = start_loc + right.1;
                    left = ExprMeta(Expr::BinaryOp(*op, Box::new(left), Box::new(right)), loc);
                } else {
                    break;
                }
            }
            Ok(left)
        } else {
            self.parse_unary_op()
        }
    }

    fn parse_unary_op(&mut self) -> Fail<ExprMeta> {
        if let Some(Token(TokenValue::Operator(op), loc)) = self.tokens.peek() {
            self.tokens.next();
            let exp = self.parse_unary_op()?;
            let loc = *loc + exp.1;
            return Ok(ExprMeta(Expr::UnaryOp(*op, Box::new(exp)), loc));
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Fail<ExprMeta> {
        if let Some(Token(value, loc)) = self.tokens.peek() {
            match value {
                TokenValue::Number(n) => {
                    self.tokens.next();
                    Ok(ExprMeta(Expr::Number(*n), *loc))
                }
                TokenValue::Identifier(name) => {
                    self.tokens.next();
                    Ok(ExprMeta(Expr::Identifier(name.clone()), *loc))
                }
                TokenValue::OpenParen => self.parse_paren(),
                TokenValue::Keyword(Keyword::For) => self.parse_if(),
                TokenValue::Keyword(Keyword::If) => self.parse_if(),
                TokenValue::Keyword(Keyword::While) => self.parse_while(),
                TokenValue::Keyword(Keyword::Func) => self.parse_fn_def(),
                TokenValue::Keyword(Keyword::Read) => {
                    self.tokens.next();
                    Ok(ExprMeta(Expr::Read, *loc))
                }
                TokenValue::Keyword(Keyword::Print) => {
                    self.tokens.next();
                    let exp = self.parse_single()?;
                    let loc = *loc + exp.1;
                    Ok(ExprMeta(Expr::Print(Box::new(exp)), loc))
                }
                TokenValue::OpenBrace => self.parse_block(),
                TokenValue::OpenBracket => self.parse_vec(),
                t => Err(Error::new(*loc, format!("Unexpected token {:?}", t))),
            }
        } else {
            Err(Error::eof())
        }
    }

    fn parse_fn_def(&mut self) -> Fail<ExprMeta> {
        let start_loc = self.consume(&TokenValue::Keyword(Keyword::Func))?;
        self.consume(&TokenValue::OpenParen)?;
        let args = self.parse_comma_sep_values(&TokenValue::CloseParen)?;
        let args_names = args
            .into_iter()
            .map(|e| match e.0 {
                Expr::Identifier(name) => Ok(name),
                _ => Err(Error::new(e.1, "Epected identifier.".into())),
            })
            .collect::<Fail<Vec<String>>>()?;
        self.consume(&TokenValue::CloseParen)?;

        let body = self.parse_single()?;
        let loc = start_loc + body.1;
        Ok(ExprMeta(Expr::FnDef(args_names, Box::new(body)), loc))
    }

    fn parse_vec(&mut self) -> Fail<ExprMeta> {
        let start_loc = self.consume(&TokenValue::OpenBracket)?;
        let result = self.parse_comma_sep_values(&TokenValue::CloseBracket)?;
        let end_loc = self.consume(&TokenValue::CloseBracket)?;
        Ok(ExprMeta(Expr::VecDef(result), start_loc + end_loc))
    }

    fn parse_comma_sep_values(&mut self, terminator: &TokenValue) -> Fail<Vec<ExprMeta>> {
        let mut args = Vec::new();
        while !self.check(terminator) {
            args.push(self.parse_single()?);
            if !self.check(&TokenValue::Comma) {
                break;
            }
            self.consume(&TokenValue::Comma)?;
        }
        Ok(args)
    }

    fn parse_paren(&mut self) -> Fail<ExprMeta> {
        self.consume(&TokenValue::OpenParen)?;
        let result = self.parse_single()?;
        self.consume(&TokenValue::CloseParen)?;
        Ok(result)
    }

    fn parse_block(&mut self) -> Fail<ExprMeta> {
        let start_loc = self.consume(&TokenValue::OpenBrace)?;
        let mut result = Vec::new();
        while !self.check(&TokenValue::CloseBrace) {
            if self.check(&TokenValue::EOL) {
                self.tokens.next();
                continue;
            }
            result.push(self.parse_single()?);
        }
        let end_loc = self.consume(&TokenValue::CloseBrace)?;
        Ok(ExprMeta(Expr::Block(result), start_loc + end_loc))
    }

    fn parse_if(&mut self) -> Fail<ExprMeta> {
        let start_loc = self.consume(&TokenValue::Keyword(Keyword::If))?;
        let cond = self.parse_single()?;
        let body = self.parse_single()?;
        let loc = start_loc + body.1;
        Ok(ExprMeta(Expr::If(Box::new(cond), Box::new(body)), loc))
    }

    fn parse_while(&mut self) -> Fail<ExprMeta> {
        let start_loc = self.consume(&TokenValue::Keyword(Keyword::While))?;
        let cond = self.parse_single()?;
        let body = self.parse_single()?;
        let loc = start_loc + body.1;
        Ok(ExprMeta(Expr::While(Box::new(cond), Box::new(body)), loc))
    }

    fn consume(&mut self, consume_type: &TokenValue) -> Fail<Loc> {
        if let Some(Token(value, loc)) = self.tokens.next() {
            if value == consume_type {
                Ok(*loc)
            } else {
                Err(Error::new(
                    *loc,
                    format!("Unexpected token {:?}, expected {:?}", value, consume_type),
                ))
            }
        } else {
            Err(Error::eof())
        }
    }

    fn check(&mut self, check_type: &TokenValue) -> bool {
        match self.tokens.peek() {
            Some(Token(value, _)) => value == check_type,
            _ => false,
        }
    }
}
