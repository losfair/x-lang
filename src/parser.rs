use crate::ast::*;
use crate::error::*;
use std::borrow::Cow;
use std::rc::Rc;

pub struct TokenStream<'a> {
    raw: &'a [u8],
    pos: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token<'a> {
    ExprBegin,
    ExprEnd,
    Lambda,
    Identifier(&'a str),
    HostFunction(&'a str),
    IntLiteral(i64),
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
                self.pos = token_end(self.raw, self.pos, |x| !x.is_ascii_digit());
                Ok(Token::IntLiteral(
                    ::std::str::from_utf8(&self.raw[start..self.pos])
                        .map_err(|_| ParseError::InvalidUtf8)?
                        .parse::<i64>()
                        .map_err(|_| ParseError::InvalidNumber)?,
                ))
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

pub fn parse_expr(input: &str) -> Result<Expr<'static>, ParseError> {
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

fn _parse_expr<'a>(input: &mut TokenStream<'a>) -> Result<Expr<'static>, ParseError> {
    let mut apply_target: Option<Expr<'static>> = None;
    let mut apply_params: Vec<Expr<'static>> = Vec::new();

    loop {
        let e = match input.next_token()? {
            Token::Identifier(id) => Expr {
                body: Rc::new(match id {
                    "true" => ExprBody::Const(ConstExpr::Bool(true)),
                    "false" => ExprBody::Const(ConstExpr::Bool(false)),
                    _ => ExprBody::Name(Cow::Owned(id.to_owned())),
                }),
            },
            Token::IntLiteral(v) => Expr {
                body: Rc::new(ExprBody::Const(ConstExpr::Int(v))),
            },
            Token::ExprBegin => _parse_expr(input)?,
            Token::ExprEnd => break,
            Token::Lambda => {
                let mut param_names: Vec<Cow<'static, str>> = Vec::new();
                let end_tk = loop {
                    let tk = input.next_token()?;
                    if let Token::Identifier(id) = tk {
                        param_names.push(Cow::Owned(id.to_owned()));
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
                    body: AbstractBody::Host(Cow::Owned(name.to_owned())),
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
