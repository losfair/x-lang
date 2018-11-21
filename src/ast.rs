use crate::builtin::ValueType;
use crate::error::*;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DataType<'a> {
    Value(ValueType),
    Sum(Vec<(Cow<'a, str>, DataType<'a>)>),
    Product(Vec<(Cow<'a, str>, DataType<'a>)>),
    FunctionDecl {
        params: Vec<Cow<'a, str>>,
        decl_expr: Expr<'a>,
        param_set: BTreeMap<Cow<'a, str>, Expr<'a>>,
    },
    Divergent,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Expr<'a> {
    #[serde(flatten)]
    pub body: Rc<ExprBody<'a>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ExprBody<'a> {
    Const(ConstExpr),
    Name(Cow<'a, str>),
    Apply {
        target: Expr<'a>,
        params: Vec<Expr<'a>>,
    },
    Abstract {
        params: Vec<Cow<'a, str>>,
        body: AbstractBody<'a>,
    },
    Match {
        value: Expr<'a>,
        branches: Vec<(Cow<'a, str>, Expr<'a>)>,
    },
    Never,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum AbstractBody<'a> {
    Host(Cow<'a, str>),
    Expr(Expr<'a>),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ConstExpr {
    Int(i64),
    Bool(bool),
}

#[derive(Default)]
pub struct RenameContext<'a> {
    rename_state: BTreeMap<Cow<'a, str>, usize>,
}

impl<'a> RenameContext<'a> {
    pub fn with_renamed<T, F: FnOnce(&mut Self) -> T>(
        &mut self,
        renames: &[Cow<'a, str>],
        f: F,
    ) -> T {
        for v in renames {
            if let Some(c) = self.rename_state.get_mut(v) {
                *c += 1;
            } else {
                self.rename_state.insert(v.clone(), 1);
            }
        }

        f(self)
    }

    pub fn get_renamed(&self, k: &Cow<'a, str>) -> Result<Cow<'a, str>, ParseError> {
        match self.rename_state.get(k) {
            Some(v) => Ok(Cow::Owned(format!("{}#{}", k, v))),
            None => Err(ParseError::Custom(format!("name not found: {}", k))),
        }
    }
}

pub fn rename_expr<'a>(e: &Expr<'a>, ctx: &mut RenameContext<'a>) -> Result<Expr<'a>, ParseError> {
    Ok(Expr {
        body: match *e.body {
            ExprBody::Const(_) => e.body.clone(),
            ExprBody::Name(ref n) => Rc::new(ExprBody::Name(ctx.get_renamed(n)?)),
            ExprBody::Apply {
                ref target,
                ref params,
            } => Rc::new(ExprBody::Apply {
                target: rename_expr(target, ctx)?,
                params: {
                    let result: Result<Vec<_>, _> =
                        params.iter().map(|v| rename_expr(v, ctx)).collect();
                    result?
                },
            }),
            ExprBody::Abstract {
                ref params,
                ref body,
            } => ctx.with_renamed(params, |ctx| {
                Ok(Rc::new(ExprBody::Abstract {
                    params: {
                        let result: Result<Vec<_>, _> =
                            params.iter().map(|v| ctx.get_renamed(v)).collect();
                        result?
                    },
                    body: match *body {
                        AbstractBody::Host(ref v) => AbstractBody::Host(v.clone()),
                        AbstractBody::Expr(ref e) => AbstractBody::Expr(rename_expr(e, ctx)?),
                    },
                }))
            })?,
            ExprBody::Match { .. } => unimplemented!(),
            ExprBody::Never => {
                return Err(ParseError::Custom("never type not expected in ast".into()));
            }
        },
    })
}
