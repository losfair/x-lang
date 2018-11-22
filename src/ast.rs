use crate::builtin::ValueType;
use crate::error::*;
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Empty,
    Value(ValueType),
    FunctionDecl {
        params: Vec<String>,
        decl_expr: Expr,
        param_set: BTreeMap<String, Expr>,
    },
    Divergent,
    Custom(Rc<Box<CustomDataType>>),
}

pub trait CustomDataType: Debug {
    fn cdt_eq(&self, other: &CustomDataType) -> bool;
    fn as_any(&self) -> &Any;
}

impl PartialEq for CustomDataType {
    fn eq(&self, other: &CustomDataType) -> bool {
        self.cdt_eq(other)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Expr {
    #[serde(flatten)]
    pub body: Rc<ExprBody>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ExprBody {
    Const(ConstExpr),
    Name(String),
    Apply {
        target: Expr,
        params: Vec<Expr>,
    },
    Abstract {
        params: Vec<String>,
        body: AbstractBody,
    },
    Match {
        value: Expr,
        branches: Vec<(String, Expr)>,
    },
    Never,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AbstractBody {
    Host(String),
    Expr(Expr),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConstExpr {
    Int(i64),
    Float(f64),
    Bool(bool),
    Empty,
}

#[derive(Default)]
pub struct RenameContext {
    rename_state: BTreeMap<String, usize>,
}

impl RenameContext {
    pub fn with_renamed<T, F: FnOnce(&mut Self) -> T>(&mut self, renames: &[String], f: F) -> T {
        for v in renames {
            if let Some(c) = self.rename_state.get_mut(v) {
                *c += 1;
            } else {
                self.rename_state.insert(v.clone(), 1);
            }
        }

        f(self)
    }

    pub fn get_renamed(&self, k: &String) -> Result<String, ParseError> {
        match self.rename_state.get(k) {
            Some(v) => Ok(format!("{}#{}", k, v)),
            None => Err(ParseError::Custom(format!("name not found: {}", k))),
        }
    }
}

pub fn rename_expr(e: &Expr, ctx: &mut RenameContext) -> Result<Expr, ParseError> {
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
