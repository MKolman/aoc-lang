use std::collections::HashMap;

use crate::{
    errors::{Error, Fail, Loc},
    lexer::Operator,
    parser::{Expr, ExprMeta},
};

#[derive(Debug, Clone)]
pub struct Env {
    parent: Option<Box<Env>>,
    vars: HashMap<String, ExprValue>,
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    pub fn new() -> Self {
        Self {
            parent: None,
            vars: HashMap::new(),
        }
    }

    pub fn fork(env: &Env) -> Self {
        Self {
            parent: Some(Box::new(env.clone())),
            vars: HashMap::new(),
        }
    }

    pub fn kill(self) -> Option<Self> {
        self.parent.map(|p| *p)
    }

    pub fn get(&self, name: &str) -> Option<&ExprValue> {
        if let Some(val) = self.vars.get(name) {
            Some(val)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ExprValue> {
        if let Some(val) = self.vars.get_mut(name) {
            Some(val)
        } else if let Some(parent) = &mut self.parent {
            parent.get_mut(name)
        } else {
            None
        }
    }

    pub fn set(&mut self, name: String, val: ExprValue) {
        if let Some(v) = self.get_mut(&name) {
            *v = val;
        } else {
            self.vars.insert(name, val);
        }
    }

    pub fn set_local(&mut self, name: String, val: ExprValue) {
        self.vars.insert(name, val);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExprValue {
    Number(i64),
    Func(Vec<String>, ExprMeta),
}

impl ExprMeta {
    pub fn eval(&self, env: &mut Env) -> Fail<ExprValue> {
        match &self.0 {
            Expr::Number(n) => Ok(ExprValue::Number(*n)),
            Expr::BinaryOp(op, left, right) => Self::eval_binary_op(env, op, left, right),
            Expr::UnaryOp(op, exp) => Self::eval_unary_op(env, op, exp),
            Expr::Assign(var, exp) => Self::eval_assign(env, var.to_string(), exp),
            Expr::If(cond, exp) => Self::eval_if(env, cond, exp),
            Expr::While(cond, exp) => Self::eval_while(env, cond, exp),
            Expr::Block(exps) => Self::eval_block(env, exps),
            Expr::Print(exp) => Self::eval_print(env, exp),
            Expr::Identifier(var) => Ok(env
                .get(var)
                .ok_or_else(|| Error::new(self.1, format!("Unknown variable: {:?}", var)))?
                .clone()),
            Expr::FnDef(args, exp) => Ok(ExprValue::Func(args.clone(), *exp.clone())),
            Expr::FnCall(name, exps) => Self::eval_fn(env, self.1, name, exps),
            exp => Err(Error::new(self.1, format!("not implemented: {:?}", exp))),
        }
    }

    fn eval_binary_op(env: &mut Env, op: &Operator, left: &Self, right: &Self) -> Fail<ExprValue> {
        if let (ExprValue::Number(n), ExprValue::Number(m)) = (left.eval(env)?, right.eval(env)?) {
            match op {
                Operator::Sum => Ok(ExprValue::Number(n + m)),
                Operator::Sub => Ok(ExprValue::Number(n - m)),
                Operator::Mul => Ok(ExprValue::Number(n * m)),
                Operator::Div => Ok(ExprValue::Number(n / m)),
                Operator::Mod => Ok(ExprValue::Number(n % m)),
                Operator::Equal => Ok(ExprValue::Number((n == m) as i64)),
                Operator::Less => Ok(ExprValue::Number((n < m) as i64)),
                Operator::LessEq => Ok(ExprValue::Number((n <= m) as i64)),
                Operator::More => Ok(ExprValue::Number((n > m) as i64)),
                Operator::MoreEq => Ok(ExprValue::Number((n >= m) as i64)),
                Operator::Or => Ok(ExprValue::Number((n != 0 || m != 0) as i64)),
                Operator::And => Ok(ExprValue::Number((n != 0 && m != 0) as i64)),
                Operator::XOr => Ok(ExprValue::Number(((n == 0) != (m == 0)) as i64)),
                op => Err(Error::new(
                    left.1 + right.1,
                    format!("Unsupported binary operator {:?}", op),
                )),
            }
        } else {
            Err(Error::new(
                left.1 + right.1,
                "Binary operators are not supported for function definitions".into(),
            ))
        }
    }

    fn eval_unary_op(env: &mut Env, op: &Operator, exp: &Self) -> Fail<ExprValue> {
        if let ExprValue::Number(n) = exp.eval(env)? {
            match op {
                Operator::Sub => Ok(ExprValue::Number(-n)),
                Operator::Sum => Ok(ExprValue::Number(n)),
                Operator::Not => Ok(ExprValue::Number((n == 0) as i64)),
                op => Err(Error::new(
                    exp.1,
                    format!("Unsupported unary operator {:?}", op),
                )),
            }
        } else {
            Err(Error::new(
                exp.1,
                "Unary operators are not supported on function definitions.".into(),
            ))
        }
    }

    fn eval_assign(env: &mut Env, var: String, exp: &Self) -> Fail<ExprValue> {
        let val = exp.eval(env)?;
        env.set(var, val.clone());
        Ok(val)
    }

    fn eval_if(env: &mut Env, cond: &Self, exp: &Self) -> Fail<ExprValue> {
        if let ExprValue::Number(0) = cond.eval(env)? {
            return Ok(ExprValue::Number(0));
        }
        exp.eval(env)
    }

    fn eval_block(env: &mut Env, exps: &[Self]) -> Fail<ExprValue> {
        let mut result = ExprValue::Number(0);
        let mut local = Env::fork(env);
        for exp in exps {
            result = exp.eval(&mut local)?;
        }
        *env = local.kill().expect("I made you");
        Ok(result)
    }

    fn eval_while(env: &mut Env, cond: &Self, exp: &Self) -> Fail<ExprValue> {
        let mut result = ExprValue::Number(0);
        while cond.eval(env)? != ExprValue::Number(0) {
            result = exp.eval(env)?;
        }
        Ok(result)
    }

    fn eval_fn(env: &mut Env, loc: Loc, name: &str, exps: &[Self]) -> Fail<ExprValue> {
        if let Some(ExprValue::Func(args, body)) = env.get(name) {
            if args.len() != exps.len() {
                return Err(Error::new(
                    loc,
                    format!(
                        "Incorrect number of arguments for function {}: got {}, expected {}",
                        name,
                        exps.len(),
                        args.len()
                    ),
                ));
            }
            let mut local = Env::fork(env);
            let vals = exps
                .iter()
                .map(|e| e.eval(&mut local))
                .collect::<Fail<Vec<ExprValue>>>()?;
            for (arg, val) in args.iter().zip(vals) {
                local.set_local(arg.to_string(), val)
            }
            let result = body.eval(&mut local);
            *env = local.kill().expect("I made you");
            result
        } else {
            Err(Error::new(loc, format!("{} is not a function", name)))
        }
    }

    fn eval_print(env: &mut Env, exp: &Self) -> Fail<ExprValue> {
        let result = exp.eval(env)?;
        match &result {
            ExprValue::Number(n) => {
                println!("{:?}", n);
            }
            ExprValue::Func(args, exp) => {
                println!("fn({:?}) {:?}", args, exp);
            }
        }
        Ok(result)
    }
}
