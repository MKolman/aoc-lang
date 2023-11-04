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

    Print(usize),
    Read,

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

    Return,
    Jump(i64),
    JumpIf(i64),
    Noop,

    VecGet,
    VecSlice,
    VecSet,
    VecCollect(usize),
    ObjCollect(usize),
    FnCall(usize),
}
