#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    Nil,
    Constant(usize),
    GetVar(usize),
    SetVar(usize),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Negate,
    UnaryPlus,

    Print,

    Not,
    And,
    Or,

    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,

    Pop,

    Jump(i64),
    JumpIf(i64),
    Noop,

    VecGet,
    VecSet,
    VecCollect(usize),
    FnCall(usize),
}
