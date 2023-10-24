use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::{collections::HashMap, ops::AddAssign};

use crate::{bytecode::Operation, token::Pos};

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Rc<String>),
    Vec(Rc<RefCell<Vec<Value>>>),
    Fn(usize, Rc<Chunk>),
    Nil,
    Ref(Rc<RefCell<Value>>),
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Self::Int(v) => v != &0,
            Self::Float(v) => v != &0.0,
            Self::Str(s) => s.len() != 0,
            Self::Nil => false,
            Self::Vec(v) => v.borrow().len() != 0,
            Self::Fn(_, _) => true,
            Self::Ref(v) => v.borrow().truthy(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Nil, Self::Nil) => true,
            (Self::Vec(a), Self::Vec(b)) => {
                a.borrow().len() == b.borrow().len()
                    && a.borrow()
                        .iter()
                        .zip(b.borrow().iter())
                        .all(|(a, b)| a.eq(b))
            }
            _ => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Vec(v) => {
                write!(f, "[")?;
                for (i, a) in v.borrow().iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{a}")?;
                }
                write!(f, "]")?;
                Ok(())
            }
            Value::Nil => write!(f, "nil"),
            Value::Fn(n_args, chunk) => {
                write!(f, "<fn(")?;
                let mut args = chunk.var_names.iter().collect::<Vec<_>>();
                args.sort_unstable_by_key(|(_, &i)| i);
                for (name, &i) in args {
                    if i >= *n_args {
                        break;
                    }
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}")?;
                }
                write!(f, "){{ {} bytes }}>", chunk.num_bytecode())
            }
            Value::Ref(v) => write!(f, "*{}", v.borrow()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub bytecode: Vec<Operation>,
    pos: Vec<Pos>,
    constants: Vec<Value>,
    var_names: HashMap<String, usize>,
    pub shared_vars: Vec<Option<Rc<RefCell<Value>>>>,
    parent: Option<Box<Chunk>>,
}

impl Chunk {
    pub fn to_child(self) -> Self {
        let mut child = Self::default();
        child.parent = Some(Box::new(self));
        child
    }

    pub fn take_parent(&mut self) -> Option<Self> {
        self.parent.take().map(|c| *c)
    }

    pub fn push_op(&mut self, op: Operation, pos: Pos) -> usize {
        self.bytecode.push(op);
        self.pos.push(pos);
        self.bytecode.len() - 1
    }

    pub fn push_const(&mut self, val: Value) -> usize {
        self.constants.push(val);
        self.constants.len() - 1
    }

    pub fn get_const(&self, idx: usize) -> &Value {
        &self.constants[idx]
    }

    pub fn num_const(&self) -> usize {
        self.constants.len()
    }

    pub fn num_var(&self) -> usize {
        self.var_names.len()
    }

    pub fn num_bytecode(&self) -> usize {
        self.bytecode.len()
    }

    pub fn lookup_var(&mut self, name: &str, share: bool) -> Option<usize> {
        if let Some(&v) = self.var_names.get(name) {
            if share && self.shared_vars[v].is_none() {
                self.shared_vars[v] = Some(Rc::new(RefCell::new(Value::Nil)));
            }
            return Some(v);
        }
        if let Some(p) = &mut self.parent {
            if let Some(idx) = p.lookup_var(name, true) {
                let new_idx = self.shared_vars.len();
                self.shared_vars.push(p.shared_vars[idx].clone());
                self.var_names.insert(name.to_string(), new_idx);
                return Some(new_idx);
            }
        }
        None
    }

    pub fn get_var(&mut self, name: &str) -> usize {
        if let Some(v) = self.lookup_var(name, false) {
            return v;
        }
        let idx = self.num_var();
        self.var_names.insert(name.to_string(), idx);
        self.shared_vars.push(None);
        idx
    }

    pub fn def_var(&mut self, name: &str) -> usize {
        if let Some(v) = self.var_names.get(name) {
            return *v;
        }
        let idx = self.num_var();
        self.var_names.insert(name.to_string(), idx);
        self.shared_vars.push(None);
        idx
    }

    pub fn jump_from(&mut self, from: usize) -> bool {
        let idx = self.bytecode.len();
        if from >= idx {
            return false;
        }

        match &mut self.bytecode[from] {
            Operation::Jump(v) | Operation::JumpIf(v) => {
                *v = (idx - from - 1) as i64;
                true
            }
            _ => false,
        }
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            bytecode: vec![],
            pos: vec![],
            constants: vec![],
            var_names: HashMap::new(),
            shared_vars: vec![],
            parent: None,
        }
    }
}

impl AddAssign<Chunk> for Chunk {
    fn add_assign(&mut self, rhs: Chunk) {
        self.bytecode.extend(rhs.bytecode);
        self.pos.extend(rhs.pos);
        self.constants.extend(rhs.constants);
    }
}
