use crate::ast::*;
use crate::error::*;
use std::rc::Rc;

pub struct TokenStream<'a> {
    raw: &'a [u8],
    pos: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'a> {
    ExprBegin,
    ExprEnd,
    Lambda,
    Identifier(&'a str),
    HostFunction(&'a str),
    EmptyLiteral,
    IntLiteral(i64),
    FloatLiteral(f64),
}

fn token_end<F: Fn(u8) -> bool>(raw: &[u8], begin: usize, predicate: F) -> usize {
    raw[begin..]
        .iter()
        .enumerate()
        .find(|(_, &x)| predicate(x))
        .map(|(i, _)| begin + i)
        .unwrap_or(raw.len())
}

impl<'a> TokenStream<'a> {
    pub fn new(raw: &'a str) -> TokenStream<'a> {
        TokenStream {
            raw: raw.as_bytes(),
            pos: 0,
        }
    }

    pub fn next_token(&mut self) -> Result<Token<'a>, ParseError> {
        if self.pos == self.raw.len() {
            return Err(ParseError::UnexpectedEnd);
        }

        let ch = self.raw[self.pos];
        self.pos += 1;

        let ret = match ch {
            b'(' => Ok(Token::ExprBegin),
            b')' => Ok(Token::ExprEnd),
            b'\\' => Ok(Token::Lambda),
            b'~' => Ok(Token::EmptyLiteral),
            b'$' => {
                let start = self.pos;
                self.pos = token_end(self.raw, self.pos, |x| {
                    !(x.is_ascii_alphanumeric() || x == b'_')
                });
                Ok(Token::HostFunction(
                    ::std::str::from_utf8(&self.raw[start..self.pos])
                        .map_err(|_| ParseError::InvalidUtf8)?,
                ))
            }
            b'#' => {
                self.pos = token_end(self.raw, self.pos, |x| x == b'\r' || x == b'\n');
                self.next_token()
            }
            x if x.is_ascii_alphabetic() || x == b'_' => {
                let start = self.pos - 1;
                self.pos = token_end(self.raw, self.pos, |x| {
                    !(x.is_ascii_alphanumeric() || x == b'_')
                });
                Ok(Token::Identifier(
                    ::std::str::from_utf8(&self.raw[start..self.pos])
                        .map_err(|_| ParseError::InvalidUtf8)?,
                ))
            }
            x if x.is_ascii_digit() => {
                let start = self.pos - 1;
                self.pos = token_end(self.raw, self.pos, |x| !x.is_ascii_digit() && x != b'.');
                Ok(::std::str::from_utf8(&self.raw[start..self.pos])
                    .map_err(|_| ParseError::InvalidUtf8)
                    .and_then(|v| {
                        if v.find(|x| x == '.').is_some() {
                            v.parse::<f64>()
                                .map(Token::FloatLiteral)
                                .map_err(|_| ParseError::InvalidNumber)
                        } else {
                            v.parse::<i64>()
                                .map(Token::IntLiteral)
                                .map_err(|_| ParseError::InvalidNumber)
                        }
                    })?)
            }
            x if x.is_ascii_whitespace() => {
                self.pos = token_end(self.raw, self.pos, |x| !x.is_ascii_whitespace());
                self.next_token()
            }
            _ => Err(ParseError::InvalidToken),
        };
        //eprintln!("{:?}", ret);
        ret
    }
}

pub fn parse_expr(input: &str) -> Result<Expr, ParseError> {
    let mut ts = TokenStream::new(input);
    match ts.next_token()? {
        Token::ExprBegin => {
            let ret = rename_expr(&_parse_expr(&mut ts)?, &mut RenameContext::default());
            if token_end(ts.raw, ts.pos, |x| !x.is_ascii_whitespace()) != ts.raw.len() {
                return Err(ParseError::BracketMismatch);
            }
            ret
        }
        _ => Err(ParseError::ExpectingExprBegin),
    }
}

fn _parse_expr<'a>(input: &mut TokenStream<'a>) -> Result<Expr, ParseError> {
    let mut apply_target: Option<Expr> = None;
    let mut apply_params: Vec<Expr> = Vec::new();

    loop {
        let e = match input.next_token()? {
            Token::Identifier(id) => Expr {
                body: Rc::new(match id {
                    "true" => ExprBody::Const(ConstExpr::Bool(true)),
                    "false" => ExprBody::Const(ConstExpr::Bool(false)),
                    _ => ExprBody::Name(id.to_string()),
                }),
            },
            Token::EmptyLiteral => Expr {
                body: Rc::new(ExprBody::Const(ConstExpr::Empty)),
            },
            Token::IntLiteral(v) => Expr {
                body: Rc::new(ExprBody::Const(ConstExpr::Int(v))),
            },
            Token::FloatLiteral(v) => Expr {
                body: Rc::new(ExprBody::Const(ConstExpr::Float(v))),
            },
            Token::ExprBegin => _parse_expr(input)?,
            Token::ExprEnd => break,
            Token::Lambda => {
                let mut param_names: Vec<String> = Vec::new();
                let end_tk = loop {
                    let tk = input.next_token()?;
                    if let Token::Identifier(id) = tk {
                        param_names.push(id.to_string());
                    } else {
                        break tk;
                    }
                };
                if end_tk != Token::ExprBegin {
                    return Err(ParseError::ExpectingExprBegin);
                }
                let body = _parse_expr(input)?;
                Expr {
                    body: Rc::new(ExprBody::Abstract {
                        params: param_names,
                        body: AbstractBody::Expr(body),
                    }),
                }
            }
            Token::HostFunction(name) => Expr {
                body: Rc::new(ExprBody::Abstract {
                    params: vec![],
                    body: AbstractBody::Host(name.to_string()),
                }),
            },
        };
        if apply_target.is_none() {
            apply_target = Some(e);
        } else {
            apply_params.push(e);
        }
    }
    if let Some(apply_target) = apply_target {
        Ok(if apply_params.len() == 0 {
            apply_target
        } else {
            Expr {
                body: Rc::new(ExprBody::Apply {
                    target: apply_target,
                    params: apply_params,
                }),
            }
        })
    } else {
        Err(ParseError::ExpectingExprBody)
    }
}
