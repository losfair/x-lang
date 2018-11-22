extern crate x_lang;

use std::env;
use std::fs::File;
use std::io::Read;

fn main() {
    let ast_path = env::args().nth(1).unwrap();

    let mut ast_text = String::new();
    File::open(&ast_path)
        .unwrap()
        .read_to_string(&mut ast_text)
        .unwrap();
    let ast: x_lang::ast::Expr = x_lang::parser::parse_expr(ast_text.as_str()).unwrap();

    let mut trs = x_lang::typeck::TypeResolveState::default();
    let mut ectx = x_lang::eval::EvalContext::default();

    let hm = x_lang::corelib::HostManager::new();

    trs.add_hosts(hm.get_binops());
    ectx.add_hosts(hm.get_binops());

    trs.add_hosts(hm.get_ifop());
    ectx.add_hosts(hm.get_ifop());

    trs.add_hosts(hm.get_relops());
    ectx.add_hosts(hm.get_relops());

    trs.add_hosts(hm.get_list_ops());
    ectx.add_hosts(hm.get_list_ops());

    let ty = x_lang::typeck::check_expr(&ast, &mut trs).unwrap();
    println!("{:?}", ty);

    if ty == x_lang::ast::DataType::Divergent {
        panic!("error: your program will never terminate");
    }

    let ret = x_lang::eval::eval_expr(&ast, &mut ectx).unwrap();
    println!("ECTX: {:?}\nVALUE: {:?}", ectx, ret);
}
