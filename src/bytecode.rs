#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    Nil,
    Constant(u8),
    GetVar(u8),
    SetVar(u8),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Negate,
    UnaryPlus,

    Print(u8),
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
    Jump(i8),
    JumpIf(i8),
    Noop,

    VecGet,
    VecSlice,
    VecSet,
    VecCollect(u8),
    ObjCollect(u8),
    FnCall(u8),
}
