use crate::ast::DataType;
use crate::error::*;
use crate::eval::{EvalContext, LazyValue, RuntimeValue};
use std::fmt::Debug;

pub trait HostFunction: Debug {
    fn typeck<'a>(&self, params: &[DataType<'a>]) -> Result<DataType<'a>, TypeError>;
    fn eval<'a, 'b, 'c>(
        &self,
        ectx: &mut EvalContext<'a, 'b, 'c>,
        params: &mut Iterator<Item = LazyValue<'a, 'b>>,
    ) -> Result<RuntimeValue<'a, 'b>, RuntimeError>;
}
