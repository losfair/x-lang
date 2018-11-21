use crate::ast::*;
use crate::builtin::*;
use crate::error::*;
use crate::host::HostFunction;
use std::borrow::Cow;

#[derive(Debug)]
pub struct BasicRelop {
    pub int_op: fn(a: i64, b: i64) -> Result<bool, RuntimeError>,
    pub float_op: fn(a: f64, b: f64) -> Result<bool, RuntimeError>,
    pub bool_op: fn(a: bool, b: bool) -> Result<bool, RuntimeError>,
}

impl HostFunction for BasicRelop {
    fn typeck<'a>(&self, params: &[DataType<'a>]) -> Result<DataType<'a>, TypeError> {
        if params.len() == 2 {
            if params[0] == DataType::Divergent || params[1] == DataType::Divergent {
                return Ok(DataType::Divergent);
            }

            match (&params[0], &params[1]) {
                (&DataType::Value(ValueType::Int), &DataType::Value(ValueType::Int))
                | (&DataType::Value(ValueType::Int), &DataType::Value(ValueType::Float))
                | (&DataType::Value(ValueType::Float), &DataType::Value(ValueType::Int))
                | (&DataType::Value(ValueType::Float), &DataType::Value(ValueType::Float))
                | (&DataType::Value(ValueType::Bool), &DataType::Value(ValueType::Bool)) => {
                    Ok(DataType::Value(ValueType::Bool))
                }
                x => Err(TypeError::Custom(format!(
                    "unsupported types for rel operator: {:?}",
                    x
                ))),
            }
        } else {
            Err(TypeError::Custom(
                "invalid param count for rel operator".into(),
            ))
        }
    }
}

#[derive(Debug)]
pub struct BasicBinop {
    pub int_op: fn(a: i64, b: i64) -> Result<i64, RuntimeError>,
    pub float_op: fn(a: f64, b: f64) -> Result<f64, RuntimeError>,
}

impl HostFunction for BasicBinop {
    fn typeck<'a>(&self, params: &[DataType<'a>]) -> Result<DataType<'a>, TypeError> {
        if params.len() == 2 {
            if params[0] == DataType::Divergent || params[1] == DataType::Divergent {
                return Ok(DataType::Divergent);
            }

            match (&params[0], &params[1]) {
                (&DataType::Value(ValueType::Int), &DataType::Value(ValueType::Int)) => {
                    Ok(DataType::Value(ValueType::Int))
                }
                (&DataType::Value(ValueType::Int), &DataType::Value(ValueType::Float)) => {
                    Ok(DataType::Value(ValueType::Int))
                }
                (&DataType::Value(ValueType::Float), &DataType::Value(ValueType::Int)) => {
                    Ok(DataType::Value(ValueType::Float))
                }
                (&DataType::Value(ValueType::Float), &DataType::Value(ValueType::Float)) => {
                    Ok(DataType::Value(ValueType::Float))
                }
                x => Err(TypeError::Custom(format!(
                    "unsupported types for binary operator: {:?}",
                    x
                ))),
            }
        } else {
            Err(TypeError::Custom(
                "invalid param count for binary operator".into(),
            ))
        }
    }
}

#[derive(Debug)]
pub struct IfOp;
impl HostFunction for IfOp {
    fn typeck<'a>(&self, params: &[DataType<'a>]) -> Result<DataType<'a>, TypeError> {
        if params.len() == 3 {
            if params[0] == DataType::Divergent {
                Ok(DataType::Divergent)
            } else {
                if params[0] != DataType::Value(ValueType::Bool) {
                    return Err(TypeError::Custom(
                        "if predicate must be of bool type".into(),
                    ));
                }

                if params[1] == DataType::Divergent {
                    Ok(params[2].clone())
                } else if params[2] == DataType::Divergent {
                    Ok(params[1].clone())
                } else if params[1] == params[2] {
                    Ok(params[1].clone())
                } else {
                    Err(TypeError::Custom(
                        "invalid operand types for if operator".into(),
                    ))
                }
            }
        } else {
            Err(TypeError::Custom(
                "invalid param count for if operator".into(),
            ))
        }
    }
}

pub struct HostManager {
    binops: Vec<(&'static str, BasicBinop)>,
    relops: Vec<(&'static str, BasicRelop)>,
    ifop: IfOp,
}

impl HostManager {
    pub fn new() -> HostManager {
        HostManager {
            binops: vec![
                (
                    "add",
                    BasicBinop {
                        int_op: |a, b| Ok(a + b),
                        float_op: |a, b| Ok(a + b),
                    },
                ),
                (
                    "sub",
                    BasicBinop {
                        int_op: |a, b| Ok(a - b),
                        float_op: |a, b| Ok(a - b),
                    },
                ),
                (
                    "mul",
                    BasicBinop {
                        int_op: |a, b| Ok(a * b),
                        float_op: |a, b| Ok(a * b),
                    },
                ),
                (
                    "div",
                    BasicBinop {
                        int_op: |a, b| {
                            if b == 0 {
                                Err(RuntimeError::DivByZero)
                            } else {
                                Ok(a / b)
                            }
                        },
                        float_op: |a, b| Ok(a / b),
                    },
                ),
                (
                    "mod",
                    BasicBinop {
                        int_op: |a, b| {
                            if b == 0 {
                                Err(RuntimeError::DivByZero)
                            } else {
                                Ok(a % b)
                            }
                        },
                        float_op: |a, b| Ok(a % b),
                    },
                ),
            ],
            relops: vec![
                (
                    "eq",
                    BasicRelop {
                        int_op: |a, b| Ok(a == b),
                        float_op: |a, b| Ok(a == b),
                        bool_op: |a, b| Ok(a == b),
                    },
                ),
                (
                    "ne",
                    BasicRelop {
                        int_op: |a, b| Ok(a != b),
                        float_op: |a, b| Ok(a != b),
                        bool_op: |a, b| Ok(a != b),
                    },
                ),
                (
                    "and",
                    BasicRelop {
                        int_op: |a, b| Ok(a != 0 && b != 0),
                        float_op: |a, b| Ok(a != 0.0 && b != 0.0),
                        bool_op: |a, b| Ok(a && b),
                    },
                ),
                (
                    "or",
                    BasicRelop {
                        int_op: |a, b| Ok(a != 0 || b != 0),
                        float_op: |a, b| Ok(a != 0.0 || b != 0.0),
                        bool_op: |a, b| Ok(a || b),
                    },
                ),
            ],
            ifop: IfOp,
        }
    }

    pub fn get_binops<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Cow<'static, str>, &'a dyn HostFunction)> {
        self.binops
            .iter()
            .map(|(k, v)| (Cow::Borrowed(*k), v as &dyn HostFunction))
    }

    pub fn get_relops<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Cow<'static, str>, &'a dyn HostFunction)> {
        self.relops
            .iter()
            .map(|(k, v)| (Cow::Borrowed(*k), v as &dyn HostFunction))
    }

    pub fn get_ifop<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Cow<'static, str>, &'a dyn HostFunction)> {
        ::std::iter::once((Cow::Borrowed("if"), &self.ifop as &dyn HostFunction))
    }
}
