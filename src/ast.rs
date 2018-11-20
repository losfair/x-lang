use crate::builtin::ValueType;
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
        param_set: BTreeMap<Cow<'a, str>, (DataType<'a>, Expr<'a>)>,
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
