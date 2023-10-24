use std::{
    collections::HashMap,
    io::{stdout, Write},
};

use crate::{
    interpreter::ExprValue,
    parser::{Expr, ExprMeta},
};

#[derive(Debug)]
pub struct Env<W: Write> {
    pub output: W,
    frame: Frame,
    parents: Vec<Frame>,
}

impl Default for Env<std::io::Stdout> {
    fn default() -> Self {
        Self::new(stdout())
    }
}

impl<W: Write> Env<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            parents: Vec::new(),
            frame: Frame::default(),
        }
    }

    fn fork(&mut self, parent: Option<usize>, prevent_parent_mut: bool) {
        self.parents.push(std::mem::replace(
            &mut self.frame,
            Frame::new(parent, prevent_parent_mut),
        ));
    }

    pub fn fork_now(&mut self, prevent_parent_mut: bool) {
        self.fork(Some(self.parents.len()), prevent_parent_mut);
    }

    pub fn fork_from(&mut self, name: &str, prevent_parent_mut: bool) {
        let mut parent_id = self.parents.len();
        let mut cur = &self.frame;
        while let (None, Some(pid)) = (cur.vars.get(name), cur.parent) {
            parent_id = pid;
            cur = &self.parents[pid];
        }
        if cur.vars.get(name).is_none() {
            parent_id = self.parents.len()
        }
        self.fork(Some(parent_id), prevent_parent_mut);
    }

    pub fn kill(&mut self) {
        self.frame = self.parents.pop().unwrap_or_default();
    }

    pub fn get(&self, name: &str) -> Option<&ExprValue> {
        let mut cur = &self.frame;
        while let (None, Some(parent_id)) = (cur.vars.get(name), cur.parent) {
            cur = &self.parents[parent_id];
        }
        cur.vars.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ExprValue> {
        if let Some(v) = self.frame.vars.get_mut(name) {
            return Some(v);
        }
        let Some(mut parent_id) = self.frame.parent else {return None};
        while let (None, Some(pid), false) = (
            self.parents[parent_id].vars.get(name),
            self.parents[parent_id].parent,
            self.parents[parent_id].prevent_parent_mut,
        ) {
            parent_id = pid;
        }
        self.parents[parent_id].vars.get_mut(name)
    }

    pub fn set(&mut self, name: &str, val: ExprValue) {
        if let Some(v) = self.get_mut(name) {
            *v = val;
        } else {
            self.frame.vars.insert(name.to_string(), val);
        }
    }

    pub fn set_local(&mut self, name: String, val: ExprValue) {
        self.frame.vars.insert(name, val);
    }

    pub fn get_mut_exp(&mut self, name: &ExprMeta) -> Option<&mut ExprValue> {
        match &name.0 {
            Expr::Identifier(name) => self.get_mut(name),
            Expr::VecGet(expr, key) => {
                if key.len() != 1 {
                    return None;
                }
                if let Ok(ExprValue::Number(idx)) = key[0].eval(self) {
                    match self.get_mut_exp(expr) {
                        Some(ExprValue::Vec(vals)) => vals.get_mut(idx as usize),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Frame {
    // Saves all the local variables.
    pub vars: HashMap<String, ExprValue>,
    // Id of the parent frame if any.
    pub parent: Option<usize>,
    // Should we make the parent frame read-only and prevent mutability of
    // variables in parent scope.
    pub prevent_parent_mut: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Self::new(None, false)
    }
}

impl Frame {
    pub fn new(parent: Option<usize>, prevent_parent_mut: bool) -> Self {
        Frame {
            vars: HashMap::new(),
            parent,
            prevent_parent_mut,
        }
    }
}
