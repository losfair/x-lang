use crate::ast::*;
use crate::builtin::*;
use crate::error::*;
use crate::eval::{EvalContext, LazyValue, RuntimeValue};
use crate::host::*;
use crate::typeck::*;
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Debug)]
struct NotFunction {}

impl HostFunction for NotFunction {
    fn typeck<'a>(&self, params: &[DataType<'a>]) -> Result<DataType<'a>, TypeError> {
        if params.len() == 1 && params[0] == DataType::Value(ValueType::Bool) {
            Ok(DataType::Value(ValueType::Bool))
        } else {
            Err(TypeError::Custom("not: type mismatch".into()))
        }
    }

    fn eval<'a, 'b, 'c>(
        &self,
        _ectx: &mut EvalContext<'a, 'b, 'c>,
        _params: &mut Iterator<Item = LazyValue<'a, 'b>>,
    ) -> Result<RuntimeValue<'a, 'b>, RuntimeError> {
        unreachable!()
    }
}

#[test]
fn test_typeck() {
    let mut e = Expr {
        body: Rc::new(ExprBody::Apply {
            target: Expr {
                body: Rc::new(ExprBody::Abstract {
                    params: vec![Cow::Borrowed("a")],
                    body: AbstractBody::Expr(Expr {
                        body: Rc::new(ExprBody::Apply {
                            target: Expr {
                                body: Rc::new(ExprBody::Abstract {
                                    params: vec![Cow::Borrowed("value")], // unused
                                    body: AbstractBody::Host(Cow::Borrowed("not")),
                                }),
                            },
                            params: vec![Expr {
                                body: Rc::new(ExprBody::Name(Cow::Borrowed("a"))),
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
    trs.add_hosts(vec![(Cow::Borrowed("not"), &not_f as &dyn HostFunction)]);
    let out = check_expr(&mut e, &mut trs).unwrap();
    if out != DataType::Value(ValueType::Bool) {
        panic!("output type mismatch");
    }
}
