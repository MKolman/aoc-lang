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
    vars: HashMap<String, ExprValue>,
    parents: Vec<HashMap<String, ExprValue>>,
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
            vars: HashMap::new(),
        }
    }

    pub fn fork(&mut self) {
        self.parents.push(std::mem::take(&mut self.vars));
    }

    pub fn kill(&mut self) {
        self.vars = self.parents.pop().unwrap_or_default();
    }

    pub fn get(&self, name: &str) -> Option<&ExprValue> {
        if let Some(val) = self.vars.get(name) {
            return Some(val);
        }
        self.parents.iter().rev().find_map(|vars| vars.get(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ExprValue> {
        if let Some(val) = self.vars.get_mut(name) {
            return Some(val);
        }
        self.parents
            .iter_mut()
            .rev()
            .find_map(|vars| vars.get_mut(name))
    }

    pub fn set(&mut self, name: &str, val: ExprValue) {
        if let Some(v) = self.get_mut(name) {
            *v = val;
        } else {
            self.vars.insert(name.to_string(), val);
        }
    }

    pub fn set_local(&mut self, name: String, val: ExprValue) {
        self.vars.insert(name, val);
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
