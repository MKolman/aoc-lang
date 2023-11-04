use std::{cell::RefCell, fmt::Display, io::Write, rc::Rc};

use crate::{
    bytecode::Operation,
    runtime::{Capture, Chunk, Value},
};

pub struct Executor<W: Write> {
    chunk: Rc<Chunk>,
    stack: Vec<Value>,
    idx: usize,
    output: Option<W>,
    debug: bool,
}

impl<W: Write> Executor<W> {
    pub fn new(chunk: Rc<Chunk>) -> Self {
        Self {
            chunk,
            stack: Vec::new(),
            idx: 0,
            output: None,
            debug: false,
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn run(&mut self, output: W) -> (Value, W) {
        self.output = Some(output);
        for i in self.stack.len()..self.chunk.num_var() {
            match &self.chunk.captured_vars[i] {
                Capture::Local => self.stack.push(Value::Nil),
                Capture::Owned => self
                    .stack
                    .push(Value::Ref(Rc::new(RefCell::new(Value::Nil)))),
                Capture::Captured(_) => todo!(),
            };
        }
        while let Some(&cmd) = self.chunk.bytecode.get(self.idx) {
            if self.debug {
                eprintln!("{self}\n=======");
            }
            self.idx += 1;
            match cmd {
                Operation::Constant(idx) => {
                    let mut val = self.chunk.get_const(idx).clone();
                    if let Value::Fn {
                        captured, chunk, ..
                    } = &mut val
                    {
                        for is_captured in chunk.captured_vars.iter() {
                            if let Capture::Captured(idx) = is_captured {
                                captured.push(self.stack[*idx].clone());
                            }
                        }
                    }
                    self.stack.push(val);
                }
                Operation::Nil => self.stack.push(Value::Nil),
                Operation::GetVar(idx) => self.get_var(idx),
                Operation::SetVar(idx) => self.set_var(idx),
                Operation::Negate => self.unary(&Self::op_negate),
                Operation::Not => self.unary(&Self::op_not),
                Operation::UnaryPlus => self.unary(&Self::op_unary_plus),
                Operation::Add => self.binary(&Self::op_add),
                Operation::Sub => self.binary(&Self::op_sub),
                Operation::Mul => self.binary(&Self::op_mul),
                Operation::Div => self.binary(&Self::op_div),
                Operation::Mod => self.binary(&Self::op_mod),
                Operation::And => self.binary(&Self::op_and),
                Operation::Or => self.binary(&Self::op_or),
                Operation::Eq => self.binary(&Self::op_eq),
                Operation::Neq => self.binary(&Self::op_neq),
                Operation::Gt => self.binary(&Self::op_gt),
                Operation::Geq => self.binary(&Self::op_geq),
                Operation::Lt => self.binary(&Self::op_lt),
                Operation::Leq => self.binary(&Self::op_leq),
                Operation::VecGet => self.binary(&Self::op_vec_get),
                Operation::VecSlice => self.tertiary(&Self::op_vec_slice),
                Operation::VecSet => self.tertiary(&Self::op_vec_set),
                Operation::VecCollect(n) => self.vec_collect(n),
                Operation::ObjCollect(n) => self.obj_collect(n),
                Operation::Print(n) => self.print(n),
                Operation::Read => self.read(),
                Operation::Pop => _ = self.stack.pop(),
                Operation::Jump(n) => self.jump(n),
                Operation::JumpIf(n) => self.op_jump_if(n),
                Operation::Noop => (),
                Operation::FnCall(n) => self.fn_call(n),
            }
        }
        (
            self.stack.pop().expect("frame did not return a value"),
            self.output.take().unwrap(),
        )
    }

    fn unary(&mut self, cmd: &dyn Fn(Value) -> Value) {
        let v = self.stack.pop().expect("ran out of stack during execution");
        self.stack.push(cmd(v));
    }

    fn binary(&mut self, cmd: &dyn Fn(Value, Value) -> Value) {
        let right = self.stack.pop().expect("Ran out of stack during execution");
        let left = self.stack.pop().expect("Ran out of stack during execution");
        self.stack.push(cmd(left, right));
    }

    fn tertiary(&mut self, cmd: &dyn Fn(Value, Value, Value) -> Value) {
        let right = self.stack.pop().expect("Ran out of stack during execution");
        let mid = self.stack.pop().expect("Ran out of stack during execution");
        let left = self.stack.pop().expect("Ran out of stack during execution");
        self.stack.push(cmd(left, mid, right));
    }

    fn get_var(&mut self, idx: usize) {
        let val = match &self.stack[idx] {
            Value::Ref(var) => var.borrow().clone(),
            var => var.clone(),
        };
        self.stack.push(val);
    }

    fn set_var(&mut self, idx: usize) {
        let val = self
            .stack
            .last()
            .expect("Ran out of stack during execution.")
            .clone();

        match &mut self.stack[idx] {
            Value::Ref(var) => *var.borrow_mut() = val,
            var => *var = val,
        }
    }

    fn op_add(left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
            (Value::Float(a), Value::Int(b)) | (Value::Int(b), Value::Float(a)) => {
                Value::Float(a + b as f64)
            }
            (Value::Str(a), Value::Str(b)) => Value::Str(Rc::new((*a).clone() + &b)),
            (Value::Vec(a), Value::Vec(b)) => {
                let mut result = Vec::new();
                result.extend(a.borrow().iter().cloned());
                result.extend(b.borrow().iter().cloned());
                Value::Vec(Rc::new(RefCell::new(result)))
            }
            (a, b) => panic!("Unsupported Add for {a} and {b}"),
        }
    }

    fn op_sub(left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a - b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a - b as f64),
            (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 - b),
            (a, b) => panic!("Unsupported Sub for {a} and {b}"),
        }
    }

    fn op_mul(left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
            (Value::Float(a), Value::Int(b)) | (Value::Int(b), Value::Float(a)) => {
                Value::Float(a * b as f64)
            }
            (Value::Str(a), Value::Int(b)) | (Value::Int(b), Value::Str(a)) => {
                Value::Str(Rc::new(a.repeat(b as usize)))
            }
            (a, b) => panic!("Unsupported Mul for {:?} and {:?}", a, b),
        }
    }

    fn op_div(left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a / b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a / b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a / b as f64),
            (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 / b),
            (a, b) => panic!("Unsupported Div for {:?} and {:?}", a, b),
        }
    }

    fn op_mod(left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a % b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a % b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a % b as f64),
            (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 % b),
            (a, b) => panic!("Unsupported Mod for {:?} and {:?}", a, b),
        }
    }

    fn op_not(v: Value) -> Value {
        match v.truthy() {
            true => Value::Int(0),
            false => Value::Int(1),
        }
    }

    fn op_negate(v: Value) -> Value {
        match v {
            Value::Int(i) => Value::Int(-i),
            Value::Float(f) => Value::Float(-f),
            v => panic!("Cannot negate {}", v),
        }
    }

    fn op_unary_plus(v: Value) -> Value {
        match v {
            Value::Int(_) | Value::Float(_) => v,
            Value::Vec(v) => Value::Int(v.borrow().len() as i64),
            Value::Str(s) => Value::Int(s.len() as i64),
            v => panic!("Unary + invalid for {}", v),
        }
    }

    fn op_and(left: Value, right: Value) -> Value {
        if !left.truthy() {
            left
        } else {
            right
        }
    }

    fn op_or(left: Value, right: Value) -> Value {
        if left.truthy() {
            left
        } else {
            right
        }
    }

    fn op_eq(left: Value, right: Value) -> Value {
        if left == right {
            Value::Int(1)
        } else {
            Value::Int(0)
        }
    }

    fn op_neq(left: Value, right: Value) -> Value {
        Self::op_not(Self::op_eq(left, right))
    }

    fn op_jump_if(&mut self, n: i64) {
        if !self.stack.pop().expect("Ran out of stack").truthy() {
            self.jump(n);
        }
    }

    fn jump(&mut self, n: i64) {
        if n > 0 {
            self.idx += n as usize;
        } else {
            self.idx -= (-n) as usize;
        }
    }

    fn op_gt(left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int((a > b) as i64),
            (Value::Float(a), Value::Float(b)) => Value::Int((a > b) as i64),
            (Value::Float(a), Value::Int(b)) => Value::Int((a > b as f64) as i64),
            (Value::Int(a), Value::Float(b)) => Value::Int((a as f64 > b) as i64),
            (a, b) => panic!("Unsupported Gt for {:?} and {:?}", a, b),
        }
    }

    fn op_geq(left: Value, right: Value) -> Value {
        Self::op_or(
            Self::op_eq(left.clone(), right.clone()),
            Self::op_gt(left, right),
        )
    }

    fn op_lt(left: Value, right: Value) -> Value {
        Self::op_not(Self::op_geq(left, right))
    }

    fn op_leq(left: Value, right: Value) -> Value {
        Self::op_not(Self::op_gt(left, right))
    }
    fn op_vec_get(index: Value, vec: Value) -> Value {
        match (vec, index) {
            (Value::Vec(v), Value::Int(i)) => {
                let v = v.borrow();
                v.get(wrap_vec_idx(i, v.len()))
                    .expect("Index out of range")
                    .clone()
            }
            (Value::Str(s), Value::Int(i)) => Value::Int(
                *s.as_bytes()
                    .get(wrap_vec_idx(i, s.len()))
                    .expect("String index out of range") as i64,
            ),
            (Value::Obj(o), v) => o.borrow().get(&v).unwrap_or(&Value::Nil).clone(),
            (a, b) => panic!("Unsupported VecGet for {}[{}]", a, b),
        }
    }
    fn op_vec_slice(start_idx: Value, end_idx: Value, vec: Value) -> Value {
        match (vec, start_idx, end_idx) {
            (Value::Vec(v), Value::Int(s), Value::Int(e)) => {
                let v = v.borrow();
                let s = wrap_vec_idx(s, v.len());
                let e = wrap_vec_idx(e, v.len());
                Value::Vec(Rc::new(RefCell::new(
                    v[s..e].into_iter().cloned().collect(),
                )))
            }
            (Value::Str(st), Value::Int(s), Value::Int(e)) => {
                let s = wrap_vec_idx(s, st.len());
                let e = wrap_vec_idx(e, st.len());
                Value::Str(Rc::new(st[s..e].to_string()))
            }
            (a, b, c) => panic!("Unsupported VecGet for {a}[{b},{c}]"),
        }
    }
    fn op_vec_set(vec: Value, index: Value, value: Value) -> Value {
        match (vec, index) {
            (Value::Vec(v), Value::Int(i)) => {
                let mut val = v.borrow_mut();
                let i = wrap_vec_idx(i, val.len());
                val[i] = value.clone();
                value
            }
            (Value::Obj(o), index) => {
                o.borrow_mut().insert(index, value.clone());
                value
            }
            (a, b) => panic!("Unsupported VecSet for {:?}[{:?}]", a, b),
        }
    }
    fn vec_collect(&mut self, size: usize) {
        let mut vec = Vec::with_capacity(size);
        for _ in 0..size {
            vec.push(self.stack.pop().expect("Ran out of stack"));
        }
        self.stack.push(Value::Vec(Rc::new(RefCell::new(vec))));
    }

    fn obj_collect(&mut self, size: usize) {
        let mut obj = std::collections::HashMap::with_capacity(size);
        for _ in 0..size {
            let val = self.stack.pop().expect("Ran out of stack");
            let key = self.stack.pop().expect("Ran out of stack");
            obj.insert(key, val);
        }
        self.stack.push(Value::Obj(Rc::new(RefCell::new(obj))));
    }

    fn print(&mut self, num_args: usize) {
        let mut args = self.stack.split_off(self.stack.len() - num_args);
        for arg in &args {
            if let Some(out) = &mut self.output {
                write!(out, "{arg}").expect("invalid writer");
            } else {
                print!("{arg}");
            }
        }
        let last = args.pop().unwrap_or(Value::Nil);
        if let Some(out) = &mut self.output {
            writeln!(out).expect("invalid writer");
        } else {
            println!();
        }
        self.stack.push(last);
    }

    fn read(&mut self) {
        let mut input = String::new();
        let val = match std::io::stdin().read_line(&mut input) {
            Ok(_) if input.len() > 0 => {
                if input.bytes().last() == Some(b'\n') {
                    input.pop();
                }
                Value::Str(Rc::new(input))
            }
            _ => Value::Nil,
        };
        self.stack.push(val);
    }

    fn fn_call(&mut self, num_args: usize) {
        let func = self.stack.pop().expect("Ran out of stack.");
        let Value::Fn {
            num_params,
            captured,
            chunk,
        } = func
        else {
            panic!(
                "Only functions can be called, not {func}. {:?}",
                self.chunk.pos[self.idx - 1]
            );
        };
        if num_params != num_args {
            panic!("function expects {num_params} args, but got {num_args}");
        }
        let args = self.stack.split_off(self.stack.len() - num_args);
        let mut executor = Self::new(chunk);
        for (arg, captured) in args.into_iter().zip(executor.chunk.captured_vars.iter()) {
            match captured {
                Capture::Local => executor.stack.push(arg),
                Capture::Owned => executor.stack.push(Value::Ref(Rc::new(RefCell::new(arg)))),
                Capture::Captured(_) => todo!(),
            }
        }
        let mut captured = captured.iter();
        for is_captured in executor.chunk.captured_vars.iter().skip(num_args) {
            match is_captured {
                Capture::Local => executor.stack.push(Value::Nil),
                Capture::Owned => executor
                    .stack
                    .push(Value::Ref(Rc::new(RefCell::new(Value::Nil)))),
                Capture::Captured(_) => executor.stack.push(captured.next().unwrap().clone()),
            }
        }
        if self.debug {
            eprintln!(" ==== Calling function:\n{}\n ======", executor);
        }
        let (val, output) = executor.run(self.output.take().unwrap());
        self.stack.push(val);
        self.output = Some(output);
    }
}

fn wrap_vec_idx(idx: i64, len: usize) -> usize {
    if idx < 0 {
        if (-idx) as usize > len {
            panic!("Index out of range");
        }
        len - (-idx) as usize
    } else {
        if idx as usize >= len {
            panic!("Index out of range");
        }
        idx as usize
    }
}
impl<W: Write> Display for Executor<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.chunk)?;
        write!(f, "Stack: ")?;
        fmt_vec(f, &self.stack)?;
        write!(f, "\nBytecode: [")?;
        for (i, a) in self.chunk.bytecode.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            if i == self.idx {
                write!(f, " >>>")?;
            }
            write!(f, "{a:?}")?;
            if i == self.idx {
                write!(f, "<<< ")?;
            }
        }
        write!(f, "]")
    }
}
pub fn fmt_vec<T>(f: &mut std::fmt::Formatter<'_>, v: &Vec<T>) -> std::fmt::Result
where
    T: Display,
{
    write!(f, "[")?;
    for (i, a) in v.iter().enumerate() {
        if i != 0 {
            write!(f, ", ")?;
        }
        write!(f, "{a}")?;
    }
    write!(f, "]")
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn add() {
        let bytecode = vec![
            Operation::Constant(0),
            Operation::Constant(1),
            Operation::Add,
        ];
        let mut chunk = Chunk::default();
        chunk.bytecode = bytecode;
        chunk.push_const(Value::Int(1));
        chunk.push_const(Value::Int(2));
        let mut ex = Executor::new(Rc::new(chunk));
        let val = ex.run(Vec::new());
        assert_eq!(ex.stack.len(), 0);
        assert_eq!(val, (Value::Int(3), Vec::new()));
    }
}
