use crate::ast::*;
use crate::builtin::*;
use crate::error::*;
use crate::eval::{EvalContext, LazyValue, RuntimeValue};
use crate::host::*;
use crate::typeck::*;
use std::rc::Rc;

#[derive(Debug)]
struct NotFunction {}

impl HostFunction for NotFunction {
    fn typeck(&self, params: &[DataType]) -> Result<DataType, TypeError> {
        if params.len() == 1 && params[0] == DataType::Value(ValueType::Bool) {
            Ok(DataType::Value(ValueType::Bool))
        } else {
            Err(TypeError::Custom("not: type mismatch".into()))
        }
    }

    fn eval<'b, 'c>(
        &self,
        _ectx: &mut EvalContext<'b, 'c>,
        _params: &mut Iterator<Item = LazyValue<'b>>,
    ) -> Result<RuntimeValue<'b>, RuntimeError> {
        unreachable!()
    }
}

#[test]
fn test_typeck() {
    let mut e = Expr {
        body: Rc::new(ExprBody::Apply {
            target: Expr {
                body: Rc::new(ExprBody::Abstract {
                    params: vec!["a".to_string()],
                    body: AbstractBody::Expr(Expr {
                        body: Rc::new(ExprBody::Apply {
                            target: Expr {
                                body: Rc::new(ExprBody::Abstract {
                                    params: vec!["value".to_string()], // unused
                                    body: AbstractBody::Host("not".to_string()),
                                }),
                            },
                            params: vec![Expr {
                                body: Rc::new(ExprBody::Name("a".to_string())),
                            }],
                        }),
                    }),
                }),
            },
            params: vec![Expr {
                body: Rc::new(ExprBody::Const(ConstExpr::Bool(false))),
            }],
        }),
    };

    let not_f = NotFunction {};
    let mut trs = TypeResolveState::default();
    trs.add_hosts(vec![("not".to_string(), &not_f as &dyn HostFunction)]);
    let out = check_expr(&mut e, &mut trs).unwrap();
    if out != DataType::Value(ValueType::Bool) {
        panic!("output type mismatch");
    }
}
