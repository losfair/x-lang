use crate::ast::*;
use crate::builtin::ValueType;
use crate::error::TypeError;
use crate::host::HostFunction;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

fn never_expr<'a>() -> Expr<'a> {
    Expr {
        body: Rc::new(ExprBody::Never),
    }
}

#[derive(Debug, Default)]
pub struct TypeResolveState<'a, 'b> {
    subs: BTreeMap<Cow<'a, str>, Expr<'a>>,
    host_functions: BTreeMap<Cow<'a, str>, &'b dyn HostFunction>,
    expr_reach: Rc<RefCell<BTreeSet<*const ExprBody<'a>>>>,
}

pub struct ExprReachGuard<'a> {
    me: *const ExprBody<'a>,
    expr_reach: Rc<RefCell<BTreeSet<*const ExprBody<'a>>>>,
}

impl<'a> Drop for ExprReachGuard<'a> {
    fn drop(&mut self) {
        if self.expr_reach.borrow_mut().remove(&self.me) == false {
            panic!("erg: not found");
        }
    }
}

impl<'a, 'b> TypeResolveState<'a, 'b> {
    fn guarded_expr_reach(&self, e: &Expr<'a>) -> Option<ExprReachGuard<'a>> {
        let b: *const ExprBody<'a> = &*e.body;

        let mut reach = self.expr_reach.borrow_mut();
        if reach.contains(&b) {
            None
        } else {
            reach.insert(b);
            Some(ExprReachGuard {
                me: b,
                expr_reach: self.expr_reach.clone(),
            })
        }
    }

    pub fn add_hosts<H: IntoIterator<Item = (Cow<'a, str>, &'b dyn HostFunction)>>(
        &mut self,
        host_functions: H,
    ) {
        self.host_functions.extend(host_functions);
    }

    pub fn resolve_name(&self, mut name: Cow<'a, str>) -> Option<Expr<'a>> {
        let mut path: BTreeSet<Cow<'a, str>> = BTreeSet::new();

        loop {
            if path.contains(&name) {
                return Some(never_expr());
            }
            path.insert(name.clone());

            let expr = if let Some(v) = self.subs.get(&name).cloned() {
                v
            } else {
                return None;
            };
            if let ExprBody::Name(ref n) = *expr.body {
                name = n.clone();
            } else {
                return Some(expr);
            }
        }
    }

    pub fn with_resolved<T, F: FnOnce(&mut Self) -> T>(
        &mut self,
        pairs: &[(Cow<'a, str>, Expr<'a>)],
        callback: F,
    ) -> T {
        let old: Vec<(&Cow<'a, str>, Option<Expr<'a>>)> = pairs
            .iter()
            .map(|(k, _)| (k, self.subs.get(k).cloned()))
            .collect();
        pairs.iter().for_each(|(k, expr)| {
            self.subs.insert(k.clone(), expr.clone());
        });
        let ret = callback(self);
        old.into_iter().for_each(|(k, expr)| {
            if let Some(expr) = expr {
                self.subs.insert(k.clone(), expr);
            } else {
                self.subs.remove(k);
            }
        });
        ret
    }
}

pub fn check_expr<'a, 'b>(
    e: &Expr<'a>,
    trs: &mut TypeResolveState<'a, 'b>,
) -> Result<DataType<'a>, TypeError> {
    let _guard = match trs.guarded_expr_reach(e) {
        Some(v) => v,
        None => return Ok(DataType::Divergent),
    };
    match *e.body {
        ExprBody::Name(ref name) => match trs.resolve_name(name.clone()) {
            Some(e) => {
                if *e.body == ExprBody::Never {
                    Ok(DataType::Divergent)
                } else {
                    check_expr(&e, trs)
                }
            }
            None => Err(TypeError::Custom("cannot resolve name".into())),
        },
        ExprBody::Const(ref c) => Ok(match *c {
            ConstExpr::Int(_) => DataType::Value(ValueType::Int),
            ConstExpr::Bool(_) => DataType::Value(ValueType::Bool),
        }),
        ExprBody::Apply {
            ref target,
            ref params,
        } => {
            let apply_target = if let ExprBody::Name(ref name) = *target.body {
                match trs.resolve_name(name.clone()) {
                    Some(e) => {
                        if *e.body == ExprBody::Never {
                            return Ok(DataType::Divergent);
                        } else {
                            e
                        }
                    }
                    None => return Err(TypeError::Custom("cannot resolve name".into())),
                }
            } else {
                target.clone()
            };
            let target_ty = check_expr(&apply_target, trs)?;
            let apply_params = params;

            match target_ty {
                DataType::FunctionDecl {
                    ref params,
                    ref decl_expr,
                    ref param_set,
                } => {
                    if params.len() == apply_params.len() {
                        let mut resolved: Vec<(Cow<'a, str>, Expr<'a>)> = Vec::new();
                        let mut param_types: Vec<DataType<'a>> = Vec::new();

                        for i in 0..params.len() {
                            let param_ty = check_expr(&apply_params[i], trs)?;
                            param_types.push(param_ty.clone());
                            resolved.push((params[i].clone(), apply_params[i].clone()));
                        }

                        match *decl_expr.body {
                            ExprBody::Abstract { ref body, .. } => match *body {
                                AbstractBody::Host(ref host) => {
                                    if let Some(ref host) = trs.host_functions.get(host.as_ref()) {
                                        Ok(host.typeck(&param_types)?)
                                    } else {
                                        Err(TypeError::Custom("host function not found".into()))
                                    }
                                }
                                AbstractBody::Expr(ref e) => {
                                    let mut new_subs = param_set.clone();
                                    ::std::mem::swap(&mut new_subs, &mut trs.subs);

                                    let ret = trs
                                        .with_resolved(resolved.as_ref(), |trs| check_expr(e, trs));

                                    ::std::mem::swap(&mut new_subs, &mut trs.subs);

                                    Ok(ret?)
                                }
                            },
                            _ => panic!("invalid decl expr"),
                        }
                    } else {
                        Err(TypeError::Custom("param count mismatch".into()))
                    }
                }
                _ => {
                    if apply_params.len() != 0 {
                        Err(TypeError::Custom(
                            "cannot apply with params on non-function values".into(),
                        ))
                    } else {
                        Ok(target_ty)
                    }
                }
            }
        }
        ExprBody::Abstract { ref params, .. } => Ok(DataType::FunctionDecl {
            params: params.clone(),
            decl_expr: e.clone(),
            param_set: trs.subs.clone(),
        }),
        ExprBody::Match { .. } => {
            unimplemented!();
        }
        ExprBody::Never => Err(TypeError::Custom("unexpected never expr".into())),
    }
}
