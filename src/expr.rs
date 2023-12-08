use std::collections::HashSet;
use std::rc::Rc;

use crate::bytecode::Operation;
use crate::error::Stackable;
use crate::runtime::{Chunk, Value};
use crate::token::Pos;
use crate::{lexer, parser};

type Error = crate::error::Error<crate::error::SyntaxError>;
type Result<T> = crate::error::Result<T, crate::error::SyntaxError>;

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    XOr,

    And,
    Or,
    Eq,
    Neq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    Not,

    LeftShift,
    RightShift,
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
                Operator::Greater,
                Operator::GreaterEq,
                Operator::Eq,
                Operator::Neq,
            ]),
            // Bitshift
            HashSet::from([Operator::LeftShift, Operator::RightShift]),
            // Sum
            HashSet::from([Operator::Add, Operator::Sub]),
            // Mul
            HashSet::from([Operator::Mul, Operator::Div, Operator::Mod]),
        ]
    }

    fn try_into_binary(&self) -> Option<Operation> {
        Some(match self {
            Operator::Add => Operation::Add,
            Operator::Sub => Operation::Sub,
            Operator::Mul => Operation::Mul,
            Operator::Div => Operation::Div,
            Operator::Mod => Operation::Mod,
            Operator::And => Operation::And,
            Operator::Or => Operation::Or,
            Operator::Eq => Operation::Eq,
            Operator::Neq => Operation::Neq,
            Operator::Less => Operation::Lt,
            Operator::LessEq => Operation::Leq,
            Operator::Greater => Operation::Gt,
            Operator::GreaterEq => Operation::Geq,
            Operator::LeftShift => Operation::LeftShift,
            Operator::RightShift => Operation::RightShift,
            _ => return None,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExprType {
    // Literals
    Int(i64),
    Float(f64),
    Str(Rc<String>),
    Identifier(String),
    Nil,
    // Operations
    BinaryOp {
        op: Operator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp(Operator, Box<Expr>),
    Define {
        var: String,
        val: Box<Expr>,
    },
    Assign {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    AssignOp {
        op: Operator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    // Scope
    Block(Vec<Expr>),
    // IO
    Print(Vec<Expr>),
    Read,
    // Control flow
    If {
        cond: Box<Expr>,
        body: Box<Expr>,
        elsebody: Option<Box<Expr>>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    // Functions
    FnDef {
        args: Vec<String>,
        body: Box<Expr>,
    },
    FnCall {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    // Vectors
    VecDef(Vec<Expr>),
    VecGet {
        vec: Box<Expr>,
        idx: Vec<Expr>,
    },
    ObjectDef(Vec<(Expr, Expr)>),
    Use(String),
    Return(Box<Expr>),
}

#[derive(PartialEq, Clone)]
pub struct Expr {
    pub pos: Pos,
    pub kind: ExprType,
}

impl Expr {
    pub fn new(pos: Pos, kind: ExprType) -> Self {
        Self { pos, kind }
    }

    pub fn to_chunk(&self, mut chunk: Chunk) -> Result<Chunk> {
        match &self.kind {
            ExprType::Nil => {
                chunk.push_op(Operation::Nil, self.pos);
            }
            ExprType::Int(v) => self.constant(&mut chunk, Value::Int(*v))?,
            ExprType::Float(v) => self.constant(&mut chunk, Value::Float(*v))?,
            ExprType::Str(v) => self.constant(&mut chunk, Value::Str(v.clone()))?,
            ExprType::BinaryOp { op, left, right } => {
                chunk = left.to_chunk(chunk)?;
                chunk = right.to_chunk(chunk)?;

                chunk.push_op(
                    op.try_into_binary()
                        .ok_or(self.err(format!("Invalid binary operator {op:?}")))?,
                    self.pos,
                );
            }
            ExprType::UnaryOp(op, expr) => {
                chunk = expr.to_chunk(chunk)?;
                chunk.push_op(
                    match op {
                        Operator::Sub => Operation::Negate,
                        Operator::Not => Operation::Not,
                        Operator::Add => Operation::UnaryPlus,
                        op => return Err(Error::new(format!("Invalid unary operator {:?}", op))),
                    },
                    self.pos,
                );
            }
            ExprType::Block(exprs) => {
                if exprs.is_empty() {
                    chunk.push_op(Operation::Nil, self.pos);
                }

                for (i, expr) in exprs.iter().enumerate() {
                    if i > 0 {
                        chunk.push_op(Operation::Pop, self.pos);
                    }
                    chunk = expr.to_chunk(chunk)?;
                }
            }
            ExprType::Print(exprs) => {
                for expr in exprs {
                    chunk = expr.to_chunk(chunk)?;
                }
                chunk.push_op(
                    Operation::Print(self.to_u8(exprs.len(), "Printing more than 255 values")?),
                    self.pos,
                );
            }
            ExprType::If {
                cond,
                body,
                elsebody,
            } => {
                chunk = cond.to_chunk(chunk)?;
                let jump_if_idx = chunk.push_op(Operation::JumpIf(0), self.pos);
                chunk = body.to_chunk(chunk)?;
                let jump_idx = chunk.push_op(Operation::Jump(0), self.pos);
                chunk.jump_from(jump_if_idx)?;
                if let Some(elsebody) = elsebody {
                    chunk = elsebody.to_chunk(chunk)?;
                } else {
                    chunk.push_op(Operation::Nil, self.pos);
                }
                chunk.jump_from(jump_idx)?;
            }
            ExprType::While { cond, body } => {
                chunk.push_op(Operation::Nil, self.pos);
                let start_idx = chunk.num_bytecode();
                chunk = cond.to_chunk(chunk)?;
                let jump_if_idx = chunk.push_op(Operation::JumpIf(0), self.pos);
                chunk.push_op(Operation::Pop, self.pos);
                chunk = body.to_chunk(chunk)?;
                chunk.push_op(
                    Operation::JumpBack(
                        (chunk.num_bytecode() + 1usize - start_idx)
                            .try_into()
                            .map_err(Error::from)
                            .wrap("Loop body longer than 255 bytecode", self.pos)?,
                    ),
                    self.pos,
                );
                chunk.jump_from(jump_if_idx)?;
            }
            ExprType::Assign { left, right } => {
                if let ExprType::Identifier(var) = &left.kind {
                    chunk.get_var(var); // Initialize variable for recursion
                }
                chunk = right.to_chunk(chunk)?;
                chunk = left.inner_assign(chunk, self.pos)?;
            }
            ExprType::AssignOp { op, left, right } => match &left.kind {
                ExprType::Identifier(var) => {
                    let idx = chunk
                        .lookup_var(var, false)
                        .ok_or(self.err(format!("Unknown variable {var}")))?;
                    chunk.push_op(
                        Operation::GetVar(
                            self.to_u8(idx, "More than 255 variables in local scope")?,
                        ),
                        left.pos,
                    );
                    chunk = right.to_chunk(chunk)?;
                    chunk.push_op(
                        op.try_into_binary()
                            .ok_or(self.err(format!("Invalid binary operator {op:?}")))?,
                        self.pos,
                    );
                    chunk.push_op(
                        Operation::SetVar(
                            self.to_u8(idx, "More than 255 variables in local scope")?,
                        ),
                        self.pos,
                    );
                }
                ExprType::VecGet { vec, idx } if idx.len() == 1 => {
                    chunk = idx[0].to_chunk(chunk)?;
                    chunk = vec.to_chunk(chunk)?;
                    chunk.push_op(Operation::Clone(1), self.pos);
                    chunk.push_op(Operation::Clone(1), self.pos);
                    chunk.push_op(Operation::VecGet, self.pos);
                    chunk = right.to_chunk(chunk)?;
                    chunk.push_op(
                        op.try_into_binary()
                            .ok_or(self.err(format!("Invalid binary operator {op:?}")))?,
                        self.pos,
                    );
                    chunk.push_op(Operation::Swap(2), self.pos);
                    chunk.push_op(Operation::VecSet, self.pos);
                }
                ex => {
                    return Err(self.err(format!(
                        "Can only assign to plain variables and vectors not {ex:?}."
                    )))
                }
            },

            ExprType::Identifier(var) => {
                let idx = chunk
                    .lookup_var(var, false)
                    .ok_or(self.err(format!("Unknown variable {var}")))?;
                chunk.push_op(
                    Operation::GetVar(self.to_u8(idx, "More than 255 variables in local scope")?),
                    self.pos,
                );
            }

            ExprType::VecDef(exprs) => {
                for expr in exprs.iter().rev() {
                    chunk = expr.to_chunk(chunk)?;
                }
                chunk.push_op(
                    Operation::VecCollect(
                        self.to_u8(exprs.len(), "More than 255 elements in vector literal")?,
                    ),
                    self.pos,
                );
            }

            ExprType::VecGet { vec, idx } => match idx.len() {
                1 => {
                    chunk = idx[0].to_chunk(chunk)?;
                    chunk = vec.to_chunk(chunk)?;
                    chunk.push_op(Operation::VecGet, self.pos);
                }
                2 => {
                    chunk = idx[0].to_chunk(chunk)?;
                    chunk = idx[1].to_chunk(chunk)?;
                    chunk = vec.to_chunk(chunk)?;
                    chunk.push_op(Operation::VecSlice, self.pos);
                }
                n => {
                    return Err(self.err(format!(
                        "Invalid number of vec indices: {n}. Only 1 or two are supported."
                    )))
                }
            },

            ExprType::FnDef { args, body } => {
                let mut f = chunk.to_child();
                for arg in args.iter() {
                    f.def_var(arg);
                }
                f = body.to_chunk(f)?;
                chunk = f
                    .take_parent()
                    .expect("I just added the parent, now I'm taking it back.");
                let f = Value::Fn {
                    num_params: args.len(),
                    captured: Vec::new(),
                    chunk: Rc::new(f),
                };
                self.constant(&mut chunk, f)?;
            }

            ExprType::FnCall { func, args } => {
                for arg in args {
                    chunk = arg.to_chunk(chunk)?;
                }
                chunk = func.to_chunk(chunk)?;
                chunk.push_op(
                    Operation::FnCall(self.to_u8(args.len(), "More than 255 function arguments")?),
                    self.pos,
                );
            }
            ExprType::ObjectDef(fields) => {
                for (k, v) in fields {
                    chunk = k.to_chunk(chunk)?;
                    chunk = v.to_chunk(chunk)?;
                }
                chunk.push_op(
                    Operation::ObjCollect(self.to_u8(fields.len(), "More than 255 object fields")?),
                    self.pos,
                );
            }
            ExprType::Read => {
                chunk.push_op(Operation::Read, self.pos);
            }
            ExprType::Return(expr) => {
                chunk = expr.to_chunk(chunk)?;
                chunk.push_op(Operation::Return, self.pos);
            }
            ExprType::Use(filename) => {
                let code = std::fs::read_to_string(filename)
                    .map_err(Error::from)
                    .wrap(&format!("cannot open imported file {filename}"), self.pos)?;
                let tokens = lexer::Lexer::new(&code);
                let expr = parser::Parser::new(tokens)
                    .parse()
                    .map_err(Error::from)
                    .wrap(&format!("cannot parse imported file {filename}"), self.pos)?;
                let use_chunk = expr.to_chunk(Chunk::default()).wrap(
                    &format!("could not compile imported file {filename}"),
                    self.pos,
                )?;
                let f = Value::Fn {
                    num_params: 0,
                    captured: Vec::new(),
                    chunk: Rc::new(use_chunk),
                };
                self.constant(&mut chunk, f)?;
                chunk.push_op(Operation::FnCall(0), self.pos);
            }
            ex => return Err(self.err(format!("Unimplemented expression {ex:?}"))),
        }

        Ok(chunk)
    }

    fn inner_assign(&self, mut chunk: Chunk, pos: Pos) -> Result<Chunk> {
        match &self.kind {
            ExprType::Identifier(var) => {
                let idx = chunk.get_var(var);
                chunk.push_op(
                    Operation::SetVar(self.to_u8(idx, "More than 255 variables in local scope")?),
                    pos,
                );
            }
            ExprType::VecGet { vec, idx } if idx.len() == 1 => {
                chunk = vec.to_chunk(chunk)?;
                chunk = idx[0].to_chunk(chunk)?;
                chunk.push_op(Operation::VecSet, pos);
            }
            ExprType::VecDef(exprs) => {
                chunk.push_op(
                    Operation::VecUnpack(
                        self.to_u8(exprs.len(), "Cannot unpack more than 255 elements")?,
                    ),
                    pos,
                );
                for expr in exprs {
                    chunk = expr.inner_assign(chunk, pos)?;
                    chunk.push_op(Operation::Pop, pos);
                }
            }
            ex => {
                return Err(self.err(format!(
                    "Can only assign to plain variables and vectors not {ex:?}."
                )))
            }
        }
        Ok(chunk)
    }

    fn err(&self, msg: String) -> Error {
        Error::new(msg).stack(self.pos)
    }

    fn constant(&self, chunk: &mut Chunk, v: Value) -> Result<()> {
        let idx = chunk.push_const(v);
        let idx = self.to_u8(idx, "More than 255 constants in local scope")?;
        chunk.push_op(Operation::Constant(idx), self.pos);
        Ok(())
    }

    fn to_u8(&self, n: usize, msg: &str) -> Result<u8> {
        n.try_into().map_err(Error::from).wrap(msg, self.pos)
    }
}

impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{:#?}", self.kind)
        } else {
            write!(f, "{:?}", self.kind)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn basic() {
        let expr = Expr {
            pos: Pos::new(0, 0),
            kind: ExprType::BinaryOp {
                op: Operator::Add,
                left: Box::new(Expr {
                    pos: Pos::new(0, 0),
                    kind: ExprType::Int(1),
                }),
                right: Box::new(Expr {
                    pos: Pos::new(0, 0),
                    kind: ExprType::Int(2),
                }),
            },
        };
        let chunk = expr.to_chunk(Chunk::default()).unwrap();
        assert_eq!(chunk.num_bytecode(), 3);
        assert_eq!(chunk.num_const(), 2);
    }
}
