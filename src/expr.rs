use std::collections::HashSet;
use std::rc::Rc;

use crate::bytecode::Operation;
use crate::runtime::{Chunk, Value};
use crate::token::Pos;

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
            // Sum
            HashSet::from([Operator::Add, Operator::Sub]),
            // Mul
            HashSet::from([Operator::Mul, Operator::Div, Operator::Mod]),
        ]
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
    // Scope
    Block(Vec<Expr>),
    // IO
    Print(Box<Expr>),
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
}

#[derive(Debug, PartialEq, Clone)]
pub struct Expr {
    pub pos: Pos,
    pub kind: ExprType,
}

impl Expr {
    pub fn new(pos: Pos, kind: ExprType) -> Self {
        Self { pos, kind }
    }

    pub fn to_chunk(&self, mut chunk: Chunk) -> Chunk {
        match &self.kind {
            ExprType::Nil => {
                chunk.push_op(Operation::Nil, self.pos);
            }
            ExprType::Int(v) => {
                let idx = chunk.push_const(Value::Int(*v));
                chunk.push_op(Operation::Constant(idx), self.pos);
            }
            ExprType::Float(v) => {
                let idx = chunk.push_const(Value::Float(*v));
                chunk.push_op(Operation::Constant(idx), self.pos);
            }
            ExprType::Str(v) => {
                let idx = chunk.push_const(Value::Str(v.clone()));
                chunk.push_op(Operation::Constant(idx), self.pos);
            }
            ExprType::BinaryOp { op, left, right } => {
                chunk = left.to_chunk(chunk);
                chunk = right.to_chunk(chunk);

                chunk.push_op(
                    match op {
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
                        _ => todo!(),
                    },
                    self.pos,
                );
            }
            ExprType::UnaryOp(op, expr) => {
                chunk = expr.to_chunk(chunk);
                chunk.push_op(
                    match op {
                        Operator::Sub => Operation::Negate,
                        Operator::Not => Operation::Not,
                        Operator::Add => Operation::UnaryPlus,
                        _ => todo!(),
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
                    chunk = expr.to_chunk(chunk);
                }
            }
            ExprType::Print(expr) => {
                chunk = expr.to_chunk(chunk);
                chunk.push_op(Operation::Print, self.pos);
            }
            ExprType::If {
                cond,
                body,
                elsebody,
            } => {
                chunk = cond.to_chunk(chunk);
                let jump_if_idx = chunk.push_op(Operation::JumpIf(0), self.pos);
                chunk = body.to_chunk(chunk);
                let jump_idx = chunk.push_op(Operation::Jump(0), self.pos);
                chunk.jump_from(jump_if_idx);
                if let Some(elsebody) = elsebody {
                    chunk = elsebody.to_chunk(chunk);
                } else {
                    chunk.push_op(Operation::Nil, self.pos);
                }
                chunk.jump_from(jump_idx);
            }
            ExprType::While { cond, body } => {
                chunk.push_op(Operation::Nil, self.pos);
                let start_idx = chunk.num_bytecode() as i64;
                chunk = cond.to_chunk(chunk);
                let jump_if_idx = chunk.push_op(Operation::JumpIf(0), self.pos);
                chunk.push_op(Operation::Pop, self.pos);
                chunk = body.to_chunk(chunk);
                chunk.push_op(
                    Operation::Jump(start_idx - 1 - chunk.num_bytecode() as i64),
                    self.pos,
                );
                chunk.jump_from(jump_if_idx);
            }
            ExprType::Assign { left, right } => match &left.kind {
                ExprType::Identifier(var) => {
                    let idx = chunk.get_var(var);
                    chunk = right.to_chunk(chunk);
                    chunk.push_op(Operation::SetVar(idx), self.pos);
                }
                ExprType::VecGet { vec, idx } if idx.len() == 1 => {
                    // let ExprType::Identifier(var) = &vec.kind else {
                    //     todo!()
                    // };
                    // let var_idx = chunk.get_var(&var);
                    // chunk.push_op(Operation::GetVar(var_idx), vec.pos);
                    chunk = vec.to_chunk(chunk);
                    chunk = idx[0].to_chunk(chunk);
                    chunk = right.to_chunk(chunk);
                    chunk.push_op(Operation::VecSet, self.pos);
                }
                _ => todo!("Can only assign to plain variables and vectors."),
            },

            ExprType::Identifier(var) => {
                let idx = chunk
                    .lookup_var(var, false)
                    .expect(&format!("Unknown variable {var}"));
                chunk.push_op(Operation::GetVar(idx), self.pos);
            }

            ExprType::VecDef(exprs) => {
                for expr in exprs.iter().rev() {
                    chunk = expr.to_chunk(chunk);
                }
                chunk.push_op(Operation::VecCollect(exprs.len()), self.pos);
            }

            ExprType::VecGet { vec, idx } => match idx.len() {
                1 => {
                    chunk = idx[0].to_chunk(chunk);
                    chunk = vec.to_chunk(chunk);
                    chunk.push_op(Operation::VecGet, self.pos);
                }
                2 => {
                    chunk = idx[0].to_chunk(chunk);
                    chunk = idx[1].to_chunk(chunk);
                    chunk = vec.to_chunk(chunk);
                    chunk.push_op(Operation::VecSlice, self.pos);
                }
                _ => panic!("Only single and double vector indexes are supported."),
            },

            ExprType::FnDef { args, body } => {
                let mut f = chunk.to_child();
                for arg in args.iter() {
                    f.def_var(arg);
                }
                f = body.to_chunk(f);
                chunk = f
                    .take_parent()
                    .expect("I just added the parent, now I'm taking it back.");
                let idx = chunk.push_const(Value::Fn {
                    num_params: args.len(),
                    captured: Vec::new(),
                    chunk: Rc::new(f),
                });
                chunk.push_op(Operation::Constant(idx), self.pos);
            }

            ExprType::FnCall { func, args } => {
                for arg in args {
                    chunk = arg.to_chunk(chunk);
                }
                chunk = func.to_chunk(chunk);
                chunk.push_op(Operation::FnCall(args.len()), self.pos);
            }
            ex => todo!("{:?}", ex),
        }

        chunk
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
        let chunk = expr.to_chunk(Chunk::default());
        assert_eq!(chunk.num_bytecode(), 3);
        assert_eq!(chunk.num_const(), 2);
    }
}
