use std::{collections::HashMap, fmt::Display};

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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExprValue {
    Number(i64),
    Func(Vec<String>, ExprMeta),
    Vec(Vec<ExprValue>),
}

impl Display for ExprValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", n),
            Self::Func(args, _) => write!(f, "fn({})", args.join(", ")),
            Self::Vec(vals) => {
                write!(f, "[")?;
                for (i, v) in vals.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    v.fmt(f)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl ExprMeta {
    pub fn eval(&self, env: &mut Env) -> Fail<ExprValue> {
        match &self.0 {
            Expr::Number(n) => Ok(ExprValue::Number(*n)),
            Expr::BinaryOp(op, left, right) => Self::eval_binary_op(env, op, left, right),
            Expr::UnaryOp(op, exp) => Self::eval_unary_op(env, op, exp),
            Expr::Assign(asignee, exp) => {
                let rhs = exp.eval(env)?;
                Self::eval_assign(env, asignee, rhs)
            }
            Expr::If(cond, exp) => Self::eval_if(env, cond, exp),
            Expr::While(cond, exp) => Self::eval_while(env, cond, exp),
            Expr::Block(exps) => Self::eval_block(env, exps),
            Expr::Print(exp) => Self::eval_print(env, exp),
            Expr::Identifier(var) => Ok(env
                .get(var)
                .ok_or_else(|| Error::new(self.1, format!("Unknown variable: {:?}", var)))?
                .clone()),
            Expr::FnDef(args, exp) => Ok(ExprValue::Func(args.clone(), *exp.clone())),
            Expr::FnCall(name, exps) => Self::eval_fn_call(env, self.1, name, exps),
            Expr::VecDef(exps) => Self::eval_vec_def(env, exps),
            Expr::VecGet(name, exps) => Self::eval_vec_get(env, self.1, name, exps),
            exp => Err(Error::new(self.1, format!("not implemented: {:?}", exp))),
        }
    }

    fn eval_binary_op(env: &mut Env, op: &Operator, left: &Self, right: &Self) -> Fail<ExprValue> {
        match (left.eval(env)?, right.eval(env)?) {
            (ExprValue::Number(n), ExprValue::Number(m)) => {
                Self::eval_binary_op_num_num(left.1 + right.1, op, n, m)
            }
            (ExprValue::Vec(left), ExprValue::Vec(right)) if op == &Operator::Sum => {
                Ok(ExprValue::Vec(left.into_iter().chain(right).collect()))
            }
            (ExprValue::Vec(_), _) | (_, ExprValue::Vec(_)) => Err(Error::new(
                left.1 + right.1,
                format!("Unsupported operator {:?} for vector", op),
            )),
            (ExprValue::Func(_, _), _) | (_, ExprValue::Func(_, _)) => Err(Error::new(
                left.1 + right.1,
                "Binary operators are not supported for function definitions".into(),
            )),
        }
    }

    fn eval_binary_op_num_num(loc: Loc, op: &Operator, left: i64, right: i64) -> Fail<ExprValue> {
        match op {
            Operator::Sum => Ok(ExprValue::Number(left + right)),
            Operator::Sub => Ok(ExprValue::Number(left - right)),
            Operator::Mul => Ok(ExprValue::Number(left * right)),
            Operator::Div => Ok(ExprValue::Number(left / right)),
            Operator::Mod => Ok(ExprValue::Number(left % right)),
            Operator::Equal => Ok(ExprValue::Number((left == right) as i64)),
            Operator::Less => Ok(ExprValue::Number((left < right) as i64)),
            Operator::LessEq => Ok(ExprValue::Number((left <= right) as i64)),
            Operator::More => Ok(ExprValue::Number((left > right) as i64)),
            Operator::MoreEq => Ok(ExprValue::Number((left >= right) as i64)),
            Operator::Or => Ok(ExprValue::Number((left != 0 || right != 0) as i64)),
            Operator::And => Ok(ExprValue::Number((left != 0 && right != 0) as i64)),
            Operator::XOr => Ok(ExprValue::Number(((left == 0) != (right == 0)) as i64)),
            op => Err(Error::new(
                loc,
                format!("Unsupported binary operator {:?}", op),
            )),
        }
    }

    fn eval_unary_op(env: &mut Env, op: &Operator, exp: &Self) -> Fail<ExprValue> {
        match (exp.eval(env)?, op) {
            (ExprValue::Number(n), Operator::Sub) => Ok(ExprValue::Number(-n)),
            (ExprValue::Number(n), Operator::Sum) => Ok(ExprValue::Number(n)),
            (ExprValue::Number(n), Operator::Not) => Ok(ExprValue::Number((n == 0) as i64)),
            (ExprValue::Number(_), op) => Err(Error::new(
                exp.1,
                format!("Numbers don't support unary operator {:?}", op),
            )),
            (ExprValue::Vec(v), Operator::Sum) => Ok(ExprValue::Number(v.len() as i64)),
            (ExprValue::Vec(_), op) => Err(Error::new(
                exp.1,
                format!("Vectors don't support unary operator {:?}", op),
            )),
            (ExprValue::Func(_, _), _) => Err(Error::new(
                exp.1,
                "Unary operators are not supported on function definitions.".into(),
            )),
        }
    }

    fn eval_assign(env: &mut Env, asignee: &Self, val: ExprValue) -> Fail<ExprValue> {
        match (&asignee.0, val.clone()) {
            (Expr::Identifier(name), val) => env.set(name.clone(), val),
            (Expr::VecDef(vec), ExprValue::Vec(vals)) if vec.len() == vals.len() => {
                vec.iter()
                    .zip(vals)
                    .map(|(var, val)| Self::eval_assign(env, var, val))
                    .collect::<Fail<Vec<ExprValue>>>()?;
            }
            (a, v) => {
                if let Some(store) = env.get_mut_exp(asignee) {
                    *store = v.clone();
                    return Ok(v);
                }
                return Err(Error::new(
                    asignee.1,
                    format!("Cannot assign {:?} to {:?}", v, a),
                ));
            }
        }
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

    fn eval_vec_get(env: &mut Env, loc: Loc, func: &Self, exps: &[Self]) -> Fail<ExprValue> {
        let left = func.eval(env)?;
        match left {
            ExprValue::Vec(vals) => Self::eval_vec(env, &vals, exps),
            _ => Err(Error::new(loc, format!("{:?} is not a vec", left))),
        }
    }

    fn eval_fn_call(env: &mut Env, loc: Loc, func: &Self, exps: &[Self]) -> Fail<ExprValue> {
        let left = func.eval(env)?;
        match left {
            ExprValue::Func(args, body) => Self::eval_fn(env, loc, &args, &body, exps),
            _ => Err(Error::new(loc, format!("{:?} is not a function", left))),
        }
    }

    fn eval_fn(
        env: &mut Env,
        loc: Loc,
        args: &[String],
        body: &ExprMeta,
        exps: &[Self],
    ) -> Fail<ExprValue> {
        if args.len() != exps.len() {
            return Err(Error::new(
                loc,
                format!(
                    "Incorrect number of arguments for function: got {}, expected {}",
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
    }

    fn eval_vec(env: &mut Env, vals: &[ExprValue], exps: &[ExprMeta]) -> Fail<ExprValue> {
        let indexes = exps.iter().map(|exp| match exp.eval(env)? {
            ExprValue::Number(n) => Ok((n, exp.1)),
            _ => Err(Error::new(
                exp.1,
                format!("Vec indices have to be numbers not {:?}", exp.0),
            )),
        });
        let copy = indexes
            .map(|v| {
                let (i, loc) = v?;
                vals.get(i as usize)
                    .ok_or_else(|| Error::new(loc, "Vec index out of range".to_string()))
            })
            .collect::<Fail<Vec<&ExprValue>>>()?;
        if copy.len() == 1 {
            Ok(copy[0].clone())
        } else {
            Ok(ExprValue::Vec(copy.clone().into_iter().cloned().collect()))
        }
    }

    fn eval_print(env: &mut Env, exp: &Self) -> Fail<ExprValue> {
        let result = exp.eval(env)?;
        println!("{}", result);
        Ok(result)
    }

    fn eval_vec_def(env: &mut Env, exps: &[Self]) -> Fail<ExprValue> {
        Ok(ExprValue::Vec(
            exps.iter()
                .map(|exp| exp.eval(env))
                .collect::<Fail<Vec<ExprValue>>>()?,
        ))
    }
}
