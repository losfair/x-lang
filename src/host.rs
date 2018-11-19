use crate::ast::DataType;
use crate::error::TypeError;
use std::fmt::Debug;

pub trait HostFunction: Debug {
    fn typeck<'a>(&self, params: &[DataType<'a>]) -> Result<DataType<'a>, TypeError>;
}
