use std::collections::HashSet;
use std::rc::Rc;

use crate::error;
use crate::expr::{Expr, ExprType, Operator};
use crate::lexer::Lexer;
use crate::token::{Pos, Token, TokenType};

type Error = error::Error<error::ParserError>;
type Result<T> = error::Result<T, error::ParserError>;

pub struct Parser {
    code: Rc<str>,
    tokens: std::iter::Peekable<Lexer>,
}
impl Parser {
    pub fn new(tokens: Lexer) -> Parser {
        Parser {
            code: tokens.get_input(),
            tokens: tokens.into_iter().peekable(),
        }
    }

    pub fn parse(&mut self) -> Result<Expr> {
        let mut result = Vec::new();
        self.skip_whitespace();
        while let Some(token) = self.tokens.peek() {
            match token.kind {
                TokenType::EOF => break,
                _ => result.push(self.parse_single()?),
            }
            self.skip_whitespace();
        }
        if result.len() == 0 {
            result.push(self.make_expr(Pos::new(0, 0), ExprType::Nil));
        }
        let pos = result
            .iter()
            .map(|e| e.pos)
            .fold(result[0].pos, |a, b| a + b);
        Ok(self.make_expr(pos, ExprType::Block(result)))
    }

    fn parse_single(&mut self) -> Result<Expr> {
        self.skip_whitespace();
        let result = self.parse_assignment()?;
        Ok(result)
    }

    fn parse_assignment(&mut self) -> Result<Expr> {
        let mut left = self.parse_binary_op(0)?;
        if let Some((_, op)) = self.try_consume_assign_operator() {
            let right = self.parse_assignment()?;
            if op == Operator::Eq {
                left = self.make_expr(
                    left.pos + right.pos,
                    ExprType::Assign {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                )
            } else {
                left = self.make_expr(
                    left.pos + right.pos,
                    ExprType::AssignOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                )
            }
        }
        Ok(left)
    }

    fn parse_binary_op(&mut self, idx: usize) -> Result<Expr> {
        if let Some(bin_ops) = Operator::all_bin().get(idx) {
            let mut left = self.parse_binary_op(idx + 1)?;
            let start_pos = left.pos;
            while let Some((_, op)) = self.try_consume_operator(Some(bin_ops)) {
                let right = self.parse_binary_op(idx + 1)?;
                left = self.make_expr(
                    start_pos + right.pos,
                    ExprType::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                );
            }
            Ok(left)
        } else {
            self.parse_unary_op()
        }
    }

    fn parse_unary_op(&mut self) -> Result<Expr> {
        if let Some((pos, op)) = self.try_consume_operator(None) {
            let exp = self.parse_unary_op()?;
            return Ok(self.make_expr(pos + exp.pos, ExprType::UnaryOp(op, Box::new(exp))));
        }
        self.parse_fn_vec()
    }

    fn parse_fn_vec(&mut self) -> Result<Expr> {
        let mut left = self.parse_atom()?;
        loop {
            if let Some(start_loc) = self.try_consume(&TokenType::LBracket) {
                let args = self.parse_comma_sep_values(&TokenType::RBracket)?;
                let end_loc = self.consume(&TokenType::RBracket)?;
                left = self.make_expr(
                    start_loc + end_loc,
                    ExprType::VecGet {
                        vec: Box::new(left),
                        idx: args,
                    },
                );
                continue;
            }
            if let Some(start_loc) = self.try_consume(&TokenType::LParen) {
                let args = self.parse_comma_sep_values(&TokenType::RParen)?;
                let end_loc = self.consume(&TokenType::RParen)?;
                left = self.make_expr(
                    start_loc + end_loc,
                    ExprType::FnCall {
                        func: Box::new(left),
                        args,
                    },
                );
                continue;
            }
            if let Some(start_pos) = self.try_consume(&TokenType::Dot) {
                let next = self.tokens.next().ok_or(Error::build(
                    "EOF while parsing".into(),
                    start_pos,
                    &self.code,
                ))?;
                let Token {
                    pos,
                    kind: TokenType::Identifier(name),
                } = next
                else {
                    return Err(Error::build(
                        format!("Expected an identifier after a dot not {:?}", next.kind),
                        next.pos,
                        &self.code,
                    ));
                };
                left = self.make_expr(
                    start_pos + pos,
                    ExprType::VecGet {
                        vec: Box::new(left),
                        idx: vec![self.make_expr(pos, ExprType::Str(Rc::new(name)))],
                    },
                );
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn parse_atom(&mut self) -> Result<Expr> {
        if let Some(Token { kind, pos }) = self.tokens.next() {
            match kind {
                TokenType::Nil => Ok(self.make_expr(pos, ExprType::Nil)),
                TokenType::Integer(n) => Ok(self.make_expr(pos, ExprType::Int(n))),
                TokenType::Float(n) => Ok(self.make_expr(pos, ExprType::Float(n))),
                TokenType::Identifier(name) => Ok(self.make_expr(pos, ExprType::Identifier(name))),
                TokenType::String(s) => Ok(self.make_expr(pos, ExprType::Str(Rc::new(s)))),
                TokenType::LParen => self.parse_paren(),
                TokenType::If => self.parse_if(pos),
                TokenType::While => self.parse_while(pos),
                TokenType::For => self.parse_for(pos),
                TokenType::Func => self.parse_fn_def(pos),
                TokenType::Read => self.parse_read(pos),
                TokenType::Print => self.parse_print(pos),
                TokenType::OBrace => self.parse_object(pos),
                TokenType::LBrace => self.parse_block(pos),
                TokenType::LBracket => self.parse_vec(pos),
                TokenType::Return => self.parse_return(pos),
                TokenType::Use => self.parse_use(pos),
                t => Err(Error::build(
                    format!("Unexpected token {t:?}"),
                    pos,
                    &self.code,
                )),
            }
        } else {
            Err(format!("Unexpected EOF while parsing").into())
        }
    }

    fn parse_print(&mut self, start_pos: Pos) -> Result<Expr> {
        self.consume(&TokenType::LParen)?;
        let args = self.parse_comma_sep_values(&TokenType::RParen)?;
        let end_pos = self.consume(&TokenType::RParen)?;
        Ok(self.make_expr(start_pos + end_pos, ExprType::Print(args)))
    }

    fn parse_read(&mut self, start_pos: Pos) -> Result<Expr> {
        self.consume(&TokenType::LParen)?;
        let end_pos = self.consume(&TokenType::RParen)?;
        Ok(self.make_expr(start_pos + end_pos, ExprType::Read))
    }

    fn parse_object(&mut self, start_pos: Pos) -> Result<Expr> {
        self.consume(&TokenType::RBrace)?;
        Ok(self.make_expr(start_pos, ExprType::ObjectDef(Vec::new())))
    }

    fn parse_fn_def(&mut self, start_pos: Pos) -> Result<Expr> {
        self.consume(&TokenType::LParen)?;
        let args = self.parse_comma_sep_values(&TokenType::RParen)?;
        let args_names = args
            .into_iter()
            .map(|e| {
                let ExprType::Identifier(name) = e.kind else {
                    return Err(Error::build(
                        format!(
                            "Function arguments must be plain identifiers not {:?}",
                            e.kind
                        ),
                        e.pos,
                        &self.code,
                    ));
                };
                Ok(name)
            })
            .collect::<Result<Vec<_>>>()?;
        self.consume(&TokenType::RParen)?;

        let body = self.parse_single()?;
        Ok(self.make_expr(
            start_pos + body.pos,
            ExprType::FnDef {
                args: args_names,
                body: Box::new(body),
            },
        ))
    }

    fn parse_vec(&mut self, start_pos: Pos) -> Result<Expr> {
        let result = self.parse_comma_sep_values(&TokenType::RBracket)?;
        let end_pos = self.consume(&TokenType::RBracket)?;
        Ok(self.make_expr(start_pos + end_pos, ExprType::VecDef(result)))
    }

    fn parse_comma_sep_values(&mut self, terminator: &TokenType) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        self.skip_whitespace();
        while !self.check(terminator) {
            args.push(self.parse_single()?);
            if self.try_consume(&TokenType::Comma).is_none() {
                break;
            }
            self.skip_whitespace();
        }
        Ok(args)
    }

    fn parse_paren(&mut self) -> Result<Expr> {
        let result = self.parse_single()?;
        self.consume(&TokenType::RParen)?;
        Ok(result)
    }

    fn parse_block(&mut self, pos: Pos) -> Result<Expr> {
        let mut result = Vec::new();
        self.skip_whitespace();
        while !self.check(&TokenType::RBrace) {
            result.push(self.parse_single()?);
            self.skip_whitespace();
        }
        let end_pos = self.consume(&TokenType::RBrace)?;
        Ok(self.make_expr(pos + end_pos, ExprType::Block(result)))
    }

    fn parse_if(&mut self, pos: Pos) -> Result<Expr> {
        let cond = self.parse_single()?;
        let body = self.parse_single()?;
        let mut elsebody = None;
        if self.try_consume(&TokenType::Else).is_some() {
            elsebody = Some(Box::new(self.parse_single()?));
        }
        Ok(self.make_expr(
            pos + body.pos,
            ExprType::If {
                cond: Box::new(cond),
                body: Box::new(body),
                elsebody,
            },
        ))
    }

    fn parse_while(&mut self, start_pos: Pos) -> Result<Expr> {
        let cond = self.parse_single()?;
        let body = self.parse_single()?;
        let pos = start_pos + body.pos;
        Ok(self.make_expr(
            pos,
            ExprType::While {
                cond: Box::new(cond),
                body: Box::new(body),
            },
        ))
    }

    fn parse_for(&mut self, start_pos: Pos) -> Result<Expr> {
        let init = self.parse_single()?;
        let cond = self.parse_single()?;
        let suff = self.parse_single()?;
        let body = self.parse_single()?;
        Ok(self.make_expr(
            start_pos + body.pos,
            ExprType::Block(vec![
                init,
                self.make_expr(
                    cond.pos + body.pos,
                    ExprType::While {
                        cond: Box::new(cond),
                        body: Box::new(self.make_expr(body.pos, ExprType::Block(vec![body, suff]))),
                    },
                ),
            ]),
        ))
    }

    fn parse_return(&mut self, start_pos: Pos) -> Result<Expr> {
        let result = self.parse_single()?;
        Ok(self.make_expr(start_pos + result.pos, ExprType::Return(Box::new(result))))
    }

    fn parse_use(&mut self, start_pos: Pos) -> Result<Expr> {
        let Token {
            pos,
            kind: TokenType::String(filename),
        } = self.tokens.next().expect("EOF after use")
        else {
            return Err(Error::build(
                "Expected a string literal after use not".into(),
                start_pos,
                &self.code,
            ));
        };
        Ok(self.make_expr(start_pos + pos, ExprType::Use(filename)))
    }

    fn skip_whitespace(&mut self) {
        loop {
            if self.try_consume(&TokenType::EOL).is_some() {
                continue;
            }
            if let Some(Token {
                pos: _,
                kind: TokenType::Comment(_),
            }) = self.tokens.peek()
            {
                self.tokens.next();
                continue;
            }
            break;
        }
    }

    fn try_consume_operator(&mut self, ops: Option<&HashSet<Operator>>) -> Option<(Pos, Operator)> {
        let Some(Token { pos: _, kind }) = self.tokens.peek() else {
            return None;
        };
        let Some(op) = kind.to_operator() else {
            return None;
        };
        if let Some(ops) = ops {
            if !ops.contains(&op) {
                return None;
            }
        }
        self.tokens
            .next()
            .map(|t| (t.pos, t.kind.to_operator().unwrap()))
    }

    fn try_consume_assign_operator(&mut self) -> Option<(Pos, Operator)> {
        let Some(Token { pos: _, kind }) = self.tokens.peek() else {
            return None;
        };
        let op = match kind {
            TokenType::PlusEq => Operator::Add,
            TokenType::MinusEq => Operator::Sub,
            TokenType::StarEq => Operator::Mul,
            TokenType::SlashEq => Operator::Div,
            TokenType::PercentEq => Operator::Mod,
            TokenType::Eq => Operator::Eq,
            _ => return None,
        };
        self.tokens.next().map(|t| (t.pos, op))
    }

    fn try_consume(&mut self, consume_type: &TokenType) -> Option<Pos> {
        if self.check(consume_type) {
            self.consume(consume_type).ok()
        } else {
            None
        }
    }

    fn consume(&mut self, consume_type: &TokenType) -> Result<Pos> {
        let Token { pos, kind } = self
            .tokens
            .next()
            .ok_or(format!("Expected {consume_type:?} but found while parsing"))?;
        if &kind != consume_type {
            return Err(Error::build(
                format!("Unexpected token {kind:?}, expected {consume_type:?}"),
                pos,
                &self.code,
            ));
        }
        Ok(pos)
    }

    fn check(&mut self, check_type: &TokenType) -> bool {
        match self.tokens.peek() {
            Some(Token { pos: _, kind }) => kind == check_type,
            _ => false,
        }
    }

    fn make_expr(&self, pos: Pos, kind: ExprType) -> Expr {
        Expr::new(self.code.clone(), pos, kind)
    }
}
