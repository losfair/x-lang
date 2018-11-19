use crate::ast::*;
use crate::builtin::ValueType;
use crate::error::TypeError;
use crate::host::HostFunction;
use std::borrow::Cow;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct TypeResolveState<'a, 'b> {
    subs: BTreeMap<Cow<'a, str>, DataType<'a>>,
    host_functions: BTreeMap<Cow<'a, str>, &'b dyn HostFunction>,
}

impl<'a, 'b> TypeResolveState<'a, 'b> {
    pub fn add_hosts<H: IntoIterator<Item = (Cow<'a, str>, &'b dyn HostFunction)>>(
        &mut self,
        host_functions: H,
    ) {
        self.host_functions.extend(host_functions);
    }

    pub fn with_resolved<T, F: FnOnce(&mut Self) -> T>(
        &mut self,
        pairs: &[(Cow<'a, str>, DataType<'a>)],
        callback: F,
    ) -> T {
        let old: Vec<(&Cow<'a, str>, Option<DataType<'a>>)> = pairs
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
    e: &mut Expr<'a>,
    trs: &mut TypeResolveState<'a, 'b>,
) -> Result<DataType<'a>, TypeError> {
    match e.body {
        ExprBody::Const(ref c) => Ok(match *c {
            ConstExpr::Int(_) => DataType::Value(ValueType::Int),
            ConstExpr::Bool(_) => DataType::Value(ValueType::Bool),
        }),
        ExprBody::Name(ref name) => match trs.subs.get(name) {
            Some(v) => Ok(v.clone()),
            None => Err(TypeError::Custom("undefined name".into())),
        },
        ExprBody::Apply {
            ref mut target,
            ref mut params,
        } => {
            let target_ty = check_expr(target, trs)?;
            let apply_target = target;
            let apply_params = params;

            match target_ty {
                DataType::FunctionDecl { ref params } => {
                    if params.len() == apply_params.len() {
                        let mut resolved: Vec<(Cow<'a, str>, DataType<'a>)> = Vec::new();
                        let mut param_types: Vec<DataType<'a>> = Vec::new();

                        for i in 0..params.len() {
                            let param_ty = check_expr(&mut apply_params[i], trs)?;
                            param_types.push(param_ty.clone());
                            resolved.push((params[i].clone(), param_ty));
                        }
                        Ok(
                            trs.with_resolved(resolved.as_ref(), |trs| match apply_target.body {
                                ExprBody::Abstract { ref mut body, .. } => match *body {
                                    AbstractBody::Host(ref host) => {
                                        if let Some(ref host) =
                                            trs.host_functions.get(host.as_ref())
                                        {
                                            Ok(host.typeck(&param_types)?)
                                        } else {
                                            Err(TypeError::Custom("host function not found".into()))
                                        }
                                    }
                                    AbstractBody::Expr(ref mut e) => check_expr(e, trs),
                                },
                                _ => panic!("bug: got FunctionDecl but expr is not Abstract"),
                            })?,
                        )
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
        }),
        ExprBody::Match { .. } => {
            unimplemented!();
        }
    }
}
