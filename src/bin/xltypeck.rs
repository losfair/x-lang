extern crate serde_yaml;
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
    let mut ast: x_lang::ast::Expr = serde_yaml::from_str(&ast_text).unwrap();
    let mut trs = x_lang::typeck::TypeResolveState::default();
    let ty = x_lang::typeck::check_expr(&mut ast, &mut trs).unwrap();
    println!("{:?}", ty);
}
