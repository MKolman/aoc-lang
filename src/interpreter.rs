use std::{fmt::Display, io::Write};

use crate::{
    env::Env,
    errors::{Error, Fail, Loc},
    lexer::Operator,
    parser::{Expr, ExprMeta},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExprValue {
    Number(i64),
    Func(Vec<String>, Box<ExprMeta>),
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
    pub fn eval<W: Write>(&self, env: &mut Env<W>) -> Fail<ExprValue> {
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
            Expr::FnDef(args, exp) => Ok(ExprValue::Func(args.clone(), exp.clone())),
            Expr::FnCall(name, exps) => Self::eval_fn_call(env, self.1, name, exps),
            Expr::VecDef(exps) => Self::eval_vec_def(env, exps),
            Expr::VecGet(name, exps) => Self::eval_vec_get(env, self.1, name, exps),
            exp => Err(Error::new(self.1, format!("not implemented: {:?}", exp))),
        }
    }

    fn eval_binary_op<W: Write>(
        env: &mut Env<W>,
        op: &Operator,
        left: &Self,
        right: &Self,
    ) -> Fail<ExprValue> {
        match (left.eval(env)?, right.eval(env)?) {
            (ExprValue::Number(n), ExprValue::Number(m)) => {
                Self::eval_binary_op_num_num(left.1 + right.1, op, n, m)
            }
            (ExprValue::Vec(left), ExprValue::Vec(right)) if op == &Operator::Sum => {
                Ok(ExprValue::Vec(left.into_iter().chain(right).collect()))
            }
            (ExprValue::Vec(v), ExprValue::Number(n))
            | (ExprValue::Number(n), ExprValue::Vec(v))
                if op == &Operator::Mul =>
            {
                let l = v.len() * n as usize;
                Ok(ExprValue::Vec(v.into_iter().cycle().take(l).collect()))
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

    fn eval_unary_op<W: Write>(env: &mut Env<W>, op: &Operator, exp: &Self) -> Fail<ExprValue> {
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

    fn eval_assign<W: Write>(env: &mut Env<W>, asignee: &Self, val: ExprValue) -> Fail<ExprValue> {
        match (&asignee.0, val.clone()) {
            (Expr::Identifier(name), val) => env.set(name, val),
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

    fn eval_if<W: Write>(env: &mut Env<W>, cond: &Self, exp: &Self) -> Fail<ExprValue> {
        if let ExprValue::Number(0) = cond.eval(env)? {
            return Ok(ExprValue::Number(0));
        }
        exp.eval(env)
    }

    fn eval_block<W: Write>(env: &mut Env<W>, exps: &[Self]) -> Fail<ExprValue> {
        let mut result = ExprValue::Number(0);
        env.fork_now(false);
        for exp in exps {
            result = exp.eval(env)?;
        }
        env.kill();
        Ok(result)
    }

    fn eval_while<W: Write>(env: &mut Env<W>, cond: &Self, exp: &Self) -> Fail<ExprValue> {
        let mut result = ExprValue::Number(0);
        while cond.eval(env)? != ExprValue::Number(0) {
            result = exp.eval(env)?;
        }
        Ok(result)
    }

    fn eval_vec_get<W: Write>(
        env: &mut Env<W>,
        loc: Loc,
        func: &Self,
        exps: &[Self],
    ) -> Fail<ExprValue> {
        let left = func.eval(env)?;
        match left {
            ExprValue::Vec(vals) => Self::eval_vec(env, &vals, exps),
            _ => Err(Error::new(loc, format!("{:?} is not a vec", left))),
        }
    }

    fn eval_fn_call<W: Write>(
        env: &mut Env<W>,
        loc: Loc,
        func: &Self,
        exps: &[Self],
    ) -> Fail<ExprValue> {
        let left = func.eval(env)?;
        match left {
            ExprValue::Func(args, body) => Self::eval_fn(env, loc, &args, &body, exps),
            _ => Err(Error::new(loc, format!("{:?} is not a function", left))),
        }
    }

    fn eval_fn<W: Write>(
        env: &mut Env<W>,
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

        // TODO: fork from function definition not function call
        env.fork_now(true);

        let vals = exps
            .iter()
            .map(|e| e.eval(env))
            .collect::<Fail<Vec<ExprValue>>>()?;
        for (arg, val) in args.iter().zip(vals) {
            env.set_local(arg.to_string(), val)
        }
        let result = body.eval(env);
        env.kill();
        result
    }

    fn eval_vec<W: Write>(
        env: &mut Env<W>,
        vals: &[ExprValue],
        exps: &[ExprMeta],
    ) -> Fail<ExprValue> {
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

    fn eval_print<W: Write>(env: &mut Env<W>, exp: &Self) -> Fail<ExprValue> {
        let result = exp.eval(env)?;
        writeln!(env.output, "{}", result)
            .map_err(|_| Error::new(exp.1, "Failed to print".to_string()))?;
        Ok(result)
    }

    fn eval_vec_def<W: Write>(env: &mut Env<W>, exps: &[Self]) -> Fail<ExprValue> {
        Ok(ExprValue::Vec(
            exps.iter()
                .map(|exp| exp.eval(env))
                .collect::<Fail<Vec<ExprValue>>>()?,
        ))
    }
}
