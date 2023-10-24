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
    Fn {
        num_params: usize,
        captured: Vec<Value>,
        chunk: Rc<Chunk>,
    },
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
            Self::Fn {
                num_params: _,
                captured: _,
                chunk: _,
            } => true,
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
            Value::Fn {
                num_params, chunk, ..
            } => {
                write!(f, "<fn({})", chunk.var_names[0..*num_params].join(", "),)?;
                let captured_var_names: Vec<_> = chunk
                    .captured_vars
                    .iter()
                    .zip(chunk.var_names.iter())
                    .filter_map(|(c, n)| {
                        if matches!(c, Capture::Captured(_)) {
                            Some(n)
                        } else {
                            None
                        }
                    })
                    .collect();
                if captured_var_names.len() > 0 {
                    crate::execute::fmt_vec(f, &captured_var_names)?;
                }
                write!(f, "{{ {} bytes }}>", chunk.num_bytecode())
            }
            Value::Ref(v) => write!(f, "*{}", v.borrow()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Capture {
    Local,
    Owned,
    Captured(usize),
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub bytecode: Vec<Operation>,
    pub pos: Vec<Pos>,
    constants: Vec<Value>,
    var_index: HashMap<String, usize>,
    var_names: Vec<String>,
    pub captured_vars: Vec<Capture>,
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
        self.var_index.len()
    }

    pub fn num_bytecode(&self) -> usize {
        self.bytecode.len()
    }

    pub fn lookup_var(&mut self, name: &str, capture: bool) -> Option<usize> {
        if let Some(&v) = self.var_index.get(name) {
            if capture && self.captured_vars[v] == Capture::Local {
                self.captured_vars[v] = Capture::Owned;
            }
            return Some(v);
        }
        if let Some(p) = &mut self.parent {
            if let Some(idx) = p.lookup_var(name, true) {
                let new_idx = self.captured_vars.len();
                self.captured_vars.push(Capture::Captured(idx));
                self.var_names.push(name.to_string());
                self.var_index.insert(name.to_string(), new_idx);
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
        self.var_index.insert(name.to_string(), idx);
        self.var_names.push(name.to_string());
        self.captured_vars.push(Capture::Local);
        idx
    }

    pub fn def_var(&mut self, name: &str) -> usize {
        if let Some(v) = self.var_index.get(name) {
            return *v;
        }
        let idx = self.num_var();
        self.var_index.insert(name.to_string(), idx);
        self.captured_vars.push(Capture::Local);
        self.var_names.push(name.to_string());
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
            var_index: HashMap::new(),
            var_names: vec![],
            captured_vars: vec![],
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

impl Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Shared vars: {:?}", self.captured_vars)?;
        write!(f, "Constants: [")?;
        for (i, a) in self.constants.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{a}")?;
        }
        write!(f, "]")
    }
}
