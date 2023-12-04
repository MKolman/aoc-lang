#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    Nil,
    Constant(u8),
    Clone(u8),
    Swap(u8),
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
    Jump(u8),
    JumpBack(u8),
    JumpIf(u8),
    Noop,

    VecGet,
    VecSlice,
    VecSet,
    VecCollect(u8),
    VecUnpack(u8),
    ObjCollect(u8),
    FnCall(u8),
}
