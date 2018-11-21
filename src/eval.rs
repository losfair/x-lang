use crate::ast::*;
use crate::error::*;
use crate::host::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub enum RuntimeValue<'a, 'b> {
    Int(i64),
    Float(f64),
    Bool(bool),
    Function {
        params: &'b [Cow<'a, str>],
        body: &'b Expr<'a>,
        context_stack: Vec<Rc<HashMap<&'b Cow<'a, str>, LazyValue<'a, 'b>>>>,
    },
    Host(&'b Cow<'a, str>),
}

#[derive(Clone, Debug)]
pub struct LazyValue<'a, 'b> {
    expr: &'b Expr<'a>,
    context_stack: Vec<Rc<HashMap<&'b Cow<'a, str>, LazyValue<'a, 'b>>>>,
}

#[derive(Default)]
pub struct EvalContext<'a, 'b, 'c> {
    stack: Vec<Rc<HashMap<&'b Cow<'a, str>, LazyValue<'a, 'b>>>>,
    host_functions: HashMap<Cow<'a, str>, &'c dyn HostFunction>,
}

impl<'a, 'b, 'c> EvalContext<'a, 'b, 'c> {
    pub fn add_hosts<H: IntoIterator<Item = (Cow<'a, str>, &'c dyn HostFunction)>>(
        &mut self,
        host_functions: H,
    ) {
        self.host_functions.extend(host_functions);
    }
}

pub fn eval_expr<'a, 'b, 'c>(
    e: &'b Expr<'a>,
    mut ctx: EvalContext<'a, 'b, 'c>,
) -> Result<RuntimeValue<'a, 'b>, RuntimeError> {
    _eval_expr(e, &mut ctx)
}

fn _eval_expr<'a, 'b, 'c>(
    e: &'b Expr<'a>,
    ctx: &mut EvalContext<'a, 'b, 'c>,
) -> Result<RuntimeValue<'a, 'b>, RuntimeError> {
    match *e.body {
        ExprBody::Abstract {
            ref params,
            ref body,
        } => Ok(match *body {
            AbstractBody::Expr(ref e) => RuntimeValue::Function {
                params: params,
                body: e,
                context_stack: ctx.stack.clone(),
            },
            AbstractBody::Host(ref name) => RuntimeValue::Host(name),
        }),
        ExprBody::Apply {
            ref target,
            ref params,
        } => {
            let apply_params = params;
            let target = _eval_expr(target, ctx)?;

            match target {
                RuntimeValue::Function {
                    params,
                    body,
                    mut context_stack,
                } => {
                    let lazy_params_hm: HashMap<
                        &'b Cow<'a, str>,
                        LazyValue<'a, 'b>,
                    > = apply_params
                        .iter()
                        .enumerate()
                        .map(|(i, x)| {
                            (
                                &params[i],
                                LazyValue {
                                    expr: x,
                                    context_stack: ctx.stack.clone(),
                                },
                            )
                        })
                        .collect();

                    ::std::mem::swap(&mut context_stack, &mut ctx.stack);
                    ctx.stack.push(Rc::new(lazy_params_hm));

                    let ret = _eval_expr(body, ctx);

                    ctx.stack.pop().unwrap();
                    ::std::mem::swap(&mut context_stack, &mut ctx.stack);

                    ret
                }
                RuntimeValue::Host(name) => {
                    let lazy_params: Vec<LazyValue<'a, 'b>> = apply_params
                        .iter()
                        .map(|x| LazyValue {
                            expr: x,
                            context_stack: ctx.stack.clone(),
                        })
                        .collect();
                    let hf = ctx
                        .host_functions
                        .get(name)
                        .unwrap_or_else(|| panic!("bug: host function not found"));
                    hf.eval(ctx, &lazy_params)
                }
                _ => {
                    if apply_params.len() == 0 {
                        Ok(target)
                    } else {
                        panic!("bug: type mismatch");
                    }
                }
            }
        }
        ExprBody::Const(ref ce) => Ok(match *ce {
            ConstExpr::Bool(v) => RuntimeValue::Bool(v),
            ConstExpr::Int(v) => RuntimeValue::Int(v),
        }),
        ExprBody::Match { .. } => unimplemented!(),
        ExprBody::Name(ref name) => {
            let mut lv: Option<LazyValue<'a, 'b>> = None;
            for frame in ctx.stack.iter().rev() {
                if let Some(_lv) = frame.get(name) {
                    lv = Some(_lv.clone());
                    break;
                }
            }
            let lv = lv.unwrap_or_else(|| panic!("bug: name not found: {} {:?}", name, ctx.stack));
            lv.eval(ctx)
        }
        _ => panic!(),
    }
}

impl<'a, 'b> LazyValue<'a, 'b> {
    pub fn eval<'c>(
        &self,
        ctx: &mut EvalContext<'a, 'b, 'c>,
    ) -> Result<RuntimeValue<'a, 'b>, RuntimeError> {
        let mut new_stack = self.context_stack.clone();

        ::std::mem::swap(&mut new_stack, &mut ctx.stack); // FIXME: slow
        let ret = _eval_expr(self.expr, ctx);
        ::std::mem::swap(&mut new_stack, &mut ctx.stack);

        ret
    }
}
