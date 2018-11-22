use crate::ast::*;
use crate::error::*;
use crate::host::*;
use rpds::RedBlackTreeMap;
use slab::Slab;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum RuntimeValue<'b> {
    Empty,
    Int(i64),
    Float(f64),
    Bool(bool),
    Function {
        params: &'b [String],
        body: &'b Expr,
        context_values: RedBlackTreeMap<&'b String, LazyValue<'b>>,
    },
    Host(&'b String),
    Custom(CustomValueBox),
}

#[derive(Debug)]
pub struct CustomValueBox {
    pub inner: Rc<Box<CustomValue>>,
}

impl CustomValueBox {
    pub fn new(inner: Box<CustomValue>) -> CustomValueBox {
        CustomValueBox {
            inner: Rc::new(inner),
        }
    }
}

pub trait CustomValue: Debug {
    fn as_any(&self) -> &Any;
}

impl Clone for CustomValueBox {
    fn clone(&self) -> Self {
        CustomValueBox {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LazyValue<'b> {
    expr: &'b Expr,
    context_values: RedBlackTreeMap<&'b String, LazyValue<'b>>,
    outcome: Rc<RefCell<Option<RuntimeValue<'b>>>>,
}

#[derive(Default, Debug)]
pub struct EvalContext<'b, 'c> {
    values: RedBlackTreeMap<&'b String, LazyValue<'b>>,
    host_functions: HashMap<String, &'c dyn HostFunction>,
    slots: Slab<LazyValue<'b>>,
    pub release_pool: SlotReleasePool,
}

#[derive(Clone, Debug, Default)]
pub struct SlotReleasePool {
    pool: Rc<RefCell<Vec<SlotRef>>>,
}

impl SlotReleasePool {
    pub fn put(&self, r: SlotRef) {
        self.pool.borrow_mut().push(r);
    }

    pub fn release<'b, 'c>(&self, ctx: &mut EvalContext<'b, 'c>) {
        let pool = ::std::mem::replace(&mut *self.pool.borrow_mut(), Vec::new());
        for r in pool {
            ctx.slots.remove(r.id);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SlotRef {
    id: usize,
}

impl<'b, 'c> EvalContext<'b, 'c> {
    pub fn add_hosts<H: IntoIterator<Item = (String, &'c dyn HostFunction)>>(
        &mut self,
        host_functions: H,
    ) {
        self.host_functions.extend(host_functions);
    }

    pub fn write_slot(&mut self, v: LazyValue<'b>) -> SlotRef {
        SlotRef {
            id: self.slots.insert(v),
        }
    }

    pub fn read_slot(&mut self, r: SlotRef) -> LazyValue<'b> {
        self.slots[r.id].clone()
    }
}

pub fn eval_expr<'b, 'c>(
    e: &'b Expr,
    ctx: &mut EvalContext<'b, 'c>,
) -> Result<RuntimeValue<'b>, RuntimeError> {
    let ret = _do_eval_expr(e, ctx);
    let pool = ctx.release_pool.clone();
    pool.release(ctx);
    ret
}

fn _do_eval_expr<'b, 'c>(
    e: &'b Expr,
    ctx: &mut EvalContext<'b, 'c>,
) -> Result<RuntimeValue<'b>, RuntimeError> {
    match *e.body {
        ExprBody::Abstract {
            ref params,
            ref body,
        } => Ok(match *body {
            AbstractBody::Expr(ref e) => RuntimeValue::Function {
                params: params,
                body: e,
                context_values: ctx.values.clone(),
            },
            AbstractBody::Host(ref name) => RuntimeValue::Host(name),
        }),
        ExprBody::Apply {
            ref target,
            ref params,
        } => {
            let apply_params = params;
            let target = eval_expr(target, ctx)?;

            match target {
                RuntimeValue::Function {
                    params,
                    body,
                    mut context_values,
                } => {
                    apply_params.iter().enumerate().for_each(|(i, x)| {
                        context_values = context_values.insert(
                            &params[i],
                            LazyValue {
                                expr: x,
                                context_values: ctx.values.clone(),
                                outcome: Rc::new(RefCell::new(None)),
                            },
                        );
                    });

                    ::std::mem::swap(&mut context_values, &mut ctx.values);
                    let ret = eval_expr(body, ctx);
                    ::std::mem::swap(&mut context_values, &mut ctx.values);

                    ret
                }
                RuntimeValue::Host(name) => {
                    let hf = ctx
                        .host_functions
                        .get(name)
                        .unwrap_or_else(|| panic!("bug: host function not found"));
                    let values = ctx.values.clone();
                    hf.eval(
                        ctx,
                        &mut apply_params.iter().map(|x| LazyValue {
                            expr: x,
                            context_values: values.clone(),
                            outcome: Rc::new(RefCell::new(None)),
                        }),
                    )
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
            ConstExpr::Float(v) => RuntimeValue::Float(v),
            ConstExpr::Empty => RuntimeValue::Empty,
        }),
        ExprBody::Match { .. } => unimplemented!(),
        ExprBody::Name(ref name) => {
            let lv: LazyValue<'b> =
                ctx.values.get(name).cloned().unwrap_or_else(|| {
                    panic!("bug: name not found: {} {:?}", name, ctx.values.iter())
                });
            lv.eval(ctx)
        }
        ExprBody::Never => unreachable!(),
    }
}

impl<'b> LazyValue<'b> {
    pub fn eval<'c>(
        &self,
        ctx: &mut EvalContext<'b, 'c>,
    ) -> Result<RuntimeValue<'b>, RuntimeError> {
        let mut outcome = self.outcome.borrow_mut(); // a lazy value should never be evaluated recursively
        if let Some(ref oc) = *outcome {
            return Ok(oc.clone());
        }

        let mut new_values = self.context_values.clone();

        ::std::mem::swap(&mut new_values, &mut ctx.values);
        let ret = eval_expr(self.expr, ctx);
        ::std::mem::swap(&mut new_values, &mut ctx.values);

        let ret = ret?;
        *outcome = Some(ret.clone());

        Ok(ret)
    }
}
