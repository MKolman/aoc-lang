use std::{cell::RefCell, fmt::Display, io::Write, rc::Rc};

use crate::{
    bytecode::Operation,
    error::{RuntimeError, Stackable},
    runtime::{Capture, Chunk, Value},
};

type Error = crate::error::Error<RuntimeError>;
type Result<T> = crate::error::Result<T, RuntimeError>;

pub struct Interpreter<W: Write> {
    chunk: Rc<Chunk>,
    stack: Vec<Value>,
    idx: usize,
    pub output: Option<W>,
    debug: bool,
}

impl<W: Write> Interpreter<W> {
    pub fn new(chunk: Rc<Chunk>, output: W) -> Self {
        Self {
            chunk,
            stack: Vec::new(),
            idx: 0,
            output: Some(output),
            debug: false,
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn run(&mut self) -> Result<Value> {
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
            self.dump_stack();
            self.idx += 1;
            let result = match cmd {
                Operation::Return => break,
                Operation::Constant(idx) => {
                    let mut val = self.chunk.get_const(idx as usize).clone();
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
                    Ok(())
                }
                Operation::Nil => {
                    self.stack.push(Value::Nil);
                    Ok(())
                }
                Operation::GetVar(idx) => self.get_var(idx as usize),
                Operation::SetVar(idx) => self.set_var(idx as usize),
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
                Operation::VecCollect(n) => self.vec_collect(n as usize),
                Operation::VecUnpack(n) => self.vec_unpack(n as usize),
                Operation::ObjCollect(n) => self.obj_collect(n as usize),
                Operation::Print(n) => self.print(n as usize),
                Operation::Read => self.read(),
                Operation::Pop => {
                    _ = self.stack.pop();
                    Ok(())
                }
                Operation::Jump(n) => self.jump(n as i64),
                Operation::JumpIf(n) => self.op_jump_if(n as i64),
                Operation::Noop => Ok(()),
                Operation::FnCall(n) => self.fn_call(n as usize),
                Operation::Clone(idx) => Ok(self
                    .stack
                    .push(self.stack[self.stack.len() - 1 - idx as usize].clone())),
            };
            result.stack(self.chunk.pos[self.idx - 1])?;
        }
        Ok(self.stack.pop().expect("frame did not return a value"))
    }

    fn unary(&mut self, cmd: &dyn Fn(Value) -> Result<Value>) -> Result<()> {
        let v = self.stack.pop().expect("ran out of stack during execution");
        self.stack.push(cmd(v)?);
        Ok(())
    }

    fn binary(&mut self, cmd: &dyn Fn(Value, Value) -> Result<Value>) -> Result<()> {
        let right = self.stack.pop().expect("Ran out of stack during execution");
        let left = self.stack.pop().expect("Ran out of stack during execution");
        self.stack.push(cmd(left, right)?);
        Ok(())
    }

    fn tertiary(&mut self, cmd: &dyn Fn(Value, Value, Value) -> Result<Value>) -> Result<()> {
        let right = self.stack.pop().expect("Ran out of stack during execution");
        let mid = self.stack.pop().expect("Ran out of stack during execution");
        let left = self.stack.pop().expect("Ran out of stack during execution");
        self.stack.push(cmd(left, mid, right)?);
        Ok(())
    }

    fn get_var(&mut self, idx: usize) -> Result<()> {
        let val = match &self.stack[idx] {
            Value::Ref(var) => var.borrow().clone(),
            var => var.clone(),
        };
        self.stack.push(val);
        Ok(())
    }

    fn set_var(&mut self, idx: usize) -> Result<()> {
        let val = self
            .stack
            .last()
            .expect("Ran out of stack during execution.")
            .clone();

        match &mut self.stack[idx] {
            Value::Ref(var) => *var.borrow_mut() = val,
            var => *var = val,
        };
        Ok(())
    }

    fn op_add(left: Value, right: Value) -> Result<Value> {
        let v = match (left, right) {
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
            (a, b) => return Err(format!("Unsupported Add for {a} and {b}").into()),
        };
        Ok(v)
    }

    fn op_sub(left: Value, right: Value) -> Result<Value> {
        let v = match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a - b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a - b as f64),
            (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 - b),
            (a, b) => return Err(format!("Unsupported Sub for {a} and {b}").into()),
        };
        Ok(v)
    }

    fn op_mul(left: Value, right: Value) -> Result<Value> {
        let v = match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
            (Value::Float(a), Value::Int(b)) | (Value::Int(b), Value::Float(a)) => {
                Value::Float(a * b as f64)
            }
            (Value::Str(a), Value::Int(b)) | (Value::Int(b), Value::Str(a)) => {
                Value::Str(Rc::new(a.repeat(b as usize)))
            }
            (Value::Vec(v), Value::Int(n)) | (Value::Int(n), Value::Vec(v)) => {
                let v = v.borrow();
                let mut result = Vec::with_capacity(v.len() * n as usize);
                for _ in 0..n {
                    result.extend(v.iter().cloned());
                }
                Value::Vec(Rc::new(RefCell::new(result)))
            }
            (a, b) => return Err(format!("Unsupported Mul for {a} and {b}").into()),
        };
        Ok(v)
    }

    fn op_div(left: Value, right: Value) -> Result<Value> {
        let v = match (left, right) {
            (Value::Int(a), Value::Int(b)) if b != 0 => Value::Int(a / b),
            (Value::Float(a), Value::Float(b)) if b != 0.0 => Value::Float(a / b),
            (Value::Float(a), Value::Int(b)) if b != 0 => Value::Float(a / b as f64),
            (Value::Int(a), Value::Float(b)) if b != 0.0 => Value::Float(a as f64 / b),
            (a, b) => return Err(format!("Unsupported Div for {a} and {b}").into()),
        };
        Ok(v)
    }

    fn op_mod(left: Value, right: Value) -> Result<Value> {
        let v = match (left, right) {
            (Value::Int(a), Value::Int(b)) if b != 0 => Value::Int(a % b),
            (Value::Float(a), Value::Float(b)) if b != 0. => Value::Float(a % b),
            (Value::Float(a), Value::Int(b)) if b != 0 => Value::Float(a % b as f64),
            (Value::Int(a), Value::Float(b)) if b != 0. => Value::Float(a as f64 % b),
            (a, b) => return Err(format!("Unsupported Mod for {a} and {b}").into()),
        };
        Ok(v)
    }

    fn op_not(v: Value) -> Result<Value> {
        match v.truthy() {
            true => Ok(Value::Int(0)),
            false => Ok(Value::Int(1)),
        }
    }

    fn op_negate(v: Value) -> Result<Value> {
        match v {
            Value::Int(i) => Ok(Value::Int(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            v => Err(format!("Cannot negate {v}").into()),
        }
    }

    fn op_unary_plus(v: Value) -> Result<Value> {
        match v {
            Value::Int(_) | Value::Float(_) => Ok(v),
            Value::Vec(v) => Ok(Value::Int(v.borrow().len() as i64)),
            Value::Str(s) => Ok(Value::Int(s.len() as i64)),
            v => Err(format!("Unary + invalid for {v}").into()),
        }
    }

    fn op_and(left: Value, right: Value) -> Result<Value> {
        if !left.truthy() {
            Ok(left)
        } else {
            Ok(right)
        }
    }

    fn op_or(left: Value, right: Value) -> Result<Value> {
        if left.truthy() {
            Ok(left)
        } else {
            Ok(right)
        }
    }

    fn op_eq(left: Value, right: Value) -> Result<Value> {
        if left == right {
            Ok(Value::Int(1))
        } else {
            Ok(Value::Int(0))
        }
    }

    fn op_neq(left: Value, right: Value) -> Result<Value> {
        Self::op_not(Self::op_eq(left, right)?)
    }

    fn op_jump_if(&mut self, n: i64) -> Result<()> {
        if !self.stack.pop().expect("Ran out of stack").truthy() {
            self.jump(n)?;
        }
        Ok(())
    }

    fn jump(&mut self, n: i64) -> Result<()> {
        if n > 0 {
            self.idx += n as usize;
        } else {
            self.idx -= (-n) as usize;
        }
        Ok(())
    }

    fn op_gt(left: Value, right: Value) -> Result<Value> {
        let v = match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int((a > b) as i64),
            (Value::Float(a), Value::Float(b)) => Value::Int((a > b) as i64),
            (Value::Float(a), Value::Int(b)) => Value::Int((a > b as f64) as i64),
            (Value::Int(a), Value::Float(b)) => Value::Int((a as f64 > b) as i64),
            (a, b) => return Err(format!("Unsupported Gt for {:?} and {:?}", a, b).into()),
        };
        Ok(v)
    }

    fn op_geq(left: Value, right: Value) -> Result<Value> {
        Self::op_or(
            Self::op_eq(left.clone(), right.clone())?,
            Self::op_gt(left, right)?,
        )
    }

    fn op_lt(left: Value, right: Value) -> Result<Value> {
        Self::op_not(Self::op_geq(left, right)?)
    }

    fn op_leq(left: Value, right: Value) -> Result<Value> {
        Self::op_not(Self::op_gt(left, right)?)
    }
    fn op_vec_get(index: Value, vec: Value) -> Result<Value> {
        match (vec, index) {
            (Value::Vec(v), Value::Int(i)) => {
                let v = v.borrow();
                let val = v.get(wrap_vec_idx(i, v.len())).ok_or::<Error>(
                    format!("Index {i} out of range for vector of length {}", v.len()).into(),
                )?;
                Ok(val.clone())
            }
            (Value::Str(s), Value::Int(i)) => Ok(Value::Int(
                *s.as_bytes().get(wrap_vec_idx(i, s.len())).ok_or::<Error>(
                    format!(
                        "String index {i} out of range for string of length {}",
                        s.len()
                    )
                    .into(),
                )? as i64,
            )),
            (Value::Obj(o), v) => Ok(o.borrow().get(&v).unwrap_or(&Value::Nil).clone()),
            (a, b) => Err(format!("Unsupported VecGet for {}[{}]", a, b).into()),
        }
    }
    fn op_vec_slice(start_idx: Value, end_idx: Value, vec: Value) -> Result<Value> {
        match (vec, start_idx, end_idx) {
            (Value::Vec(v), Value::Int(s), Value::Int(e)) => {
                let v = v.borrow();
                let s = wrap_vec_idx(s, v.len());
                let e = wrap_vec_idx(e, v.len());
                Ok(Value::Vec(Rc::new(RefCell::new(
                    v[s..e].into_iter().cloned().collect(),
                ))))
            }
            (Value::Str(st), Value::Int(s), Value::Int(e)) => {
                let s = wrap_vec_idx(s, st.len());
                let e = wrap_vec_idx(e, st.len());
                Ok(Value::Str(Rc::new(st[s..e].to_string())))
            }
            (a, b, c) => Err(format!("Unsupported VecGet for {a}[{b},{c}]").into()),
        }
    }
    fn op_vec_set(value: Value, vec: Value, index: Value) -> Result<Value> {
        match (vec, index) {
            (Value::Vec(v), Value::Int(i)) => {
                let mut val = v.borrow_mut();
                let i = wrap_vec_idx(i, val.len());
                val[i] = value.clone();
                Ok(value)
            }
            (Value::Obj(o), index) => {
                o.borrow_mut().insert(index, value.clone());
                Ok(value)
            }
            (a, b) => Err(format!("Unsupported VecSet for {a}[{b}]").into()),
        }
    }
    fn vec_collect(&mut self, size: usize) -> Result<()> {
        let mut vec = Vec::with_capacity(size);
        for _ in 0..size {
            vec.push(self.stack.pop().expect("Ran out of stack"));
        }
        self.stack.push(Value::Vec(Rc::new(RefCell::new(vec))));
        Ok(())
    }
    fn vec_unpack(&mut self, size: usize) -> Result<()> {
        let mut unpacked_values = Vec::with_capacity(size);
        {
            let vec = self.stack.last().expect("Ran out of stack");
            let Value::Vec(vec) = vec else {
                return Err(format!("Can only unpack vector not {vec:?}").into());
            };
            let vec = vec.borrow();
            if vec.len() != size {
                return Err(format!("Expected vector of length {size}, got {}", vec.len()).into());
            }
            for val in vec.iter().rev() {
                unpacked_values.push(val.clone());
            }
        }
        self.stack.extend(unpacked_values);
        Ok(())
    }

    fn obj_collect(&mut self, size: usize) -> Result<()> {
        let mut obj = std::collections::HashMap::with_capacity(size);
        for _ in 0..size {
            let val = self.stack.pop().expect("Ran out of stack");
            let key = self.stack.pop().expect("Ran out of stack");
            obj.insert(key, val);
        }
        self.stack.push(Value::Obj(Rc::new(RefCell::new(obj))));
        Ok(())
    }

    fn print(&mut self, num_args: usize) -> Result<()> {
        let mut args = self.stack.split_off(self.stack.len() - num_args);
        for arg in &args {
            write!(self.output.as_mut().unwrap(), "{arg}").map_err(Error::from)?;
        }
        let last = args.pop().unwrap_or(Value::Nil);
        writeln!(self.output.as_mut().unwrap()).expect("invalid writer");
        self.stack.push(last);
        Ok(())
    }

    fn read(&mut self) -> Result<()> {
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
        Ok(())
    }

    fn fn_call(&mut self, num_args: usize) -> Result<()> {
        let func = self.stack.pop().expect("Ran out of stack.");
        if self.debug {
            writeln!(self.output.as_mut().unwrap(), "=== Function {func} ===",).unwrap();
        }
        let Value::Fn {
            num_params,
            captured,
            chunk,
        } = func
        else {
            return Err(format!("Only functions can be called, not {func:?}.").into());
        };
        if num_params != num_args {
            return Err(format!("function expects {num_params} args, but got {num_args}").into());
        }
        let args = self.stack.split_off(self.stack.len() - num_args);
        if self.debug {
            writeln!(self.output.as_mut().unwrap(), "{chunk}").unwrap();
        }
        let mut executor = Self::new(chunk, self.output.take().unwrap());
        executor.set_debug(self.debug);
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
        let result = executor.run();
        self.output = Some(executor.output.take().unwrap());
        if self.debug {
            writeln!(self.output.as_mut().unwrap(), "=== Exit function ===").unwrap();
        }
        match result {
            Ok(val) => {
                self.stack.push(val);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    fn dump_stack(&mut self) {
        if !self.debug {
            return;
        }
        let f = self.output.as_mut().unwrap();
        writeln!(f, "=== Stack ===").unwrap();
        self.stack
            .iter()
            .enumerate()
            .rev()
            .for_each(|(i, v)| writeln!(f, "{i}: {v}").unwrap());
        writeln!(
            f,
            "=== Next operation ===\n{}: {:?}",
            self.idx, self.chunk.bytecode[self.idx]
        )
        .unwrap();
        writeln!(f, "=== Stdout ===").unwrap();
    }
}

fn wrap_vec_idx(idx: i64, len: usize) -> usize {
    if idx < 0 {
        len - (-idx) as usize
    } else {
        idx as usize
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
