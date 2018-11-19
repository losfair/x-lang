use crate::builtin::ValueType;
use std::borrow::Cow;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DataType<'a> {
    Value(ValueType),
    Sum(Vec<(Cow<'a, str>, DataType<'a>)>),
    Product(Vec<(Cow<'a, str>, DataType<'a>)>),
    FunctionDecl { params: Vec<Cow<'a, str>> },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Expr<'a> {
    #[serde(flatten)]
    pub body: ExprBody<'a>,
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum ExprBody<'a> {
    Const(ConstExpr),
    Name(Cow<'a, str>),
    Apply {
        target: Box<Expr<'a>>,
        params: Vec<Expr<'a>>,
    },
    Abstract {
        params: Vec<Cow<'a, str>>,
        body: AbstractBody<'a>,
    },
    Match {
        value: Box<Expr<'a>>,
        branches: Vec<(Cow<'a, str>, Expr<'a>)>,
    },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum AbstractBody<'a> {
    Host(Cow<'a, str>),
    Expr(Box<Expr<'a>>),
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum ConstExpr {
    Int(i64),
    Bool(bool),
}
