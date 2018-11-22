use crate::ast::DataType;
use crate::error::*;
use crate::eval::{EvalContext, LazyValue, RuntimeValue};
use std::fmt::Debug;

pub trait HostFunction: Debug {
    fn typeck(&self, params: &[DataType]) -> Result<DataType, TypeError>;
    fn eval<'b, 'c>(
        &self,
        ectx: &mut EvalContext<'b, 'c>,
        params: &mut Iterator<Item = LazyValue<'b>>,
    ) -> Result<RuntimeValue<'b>, RuntimeError>;
}
