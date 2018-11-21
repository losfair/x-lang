#[derive(Debug)]
pub enum ParseError {
    InvalidUtf8,
    InvalidNumber,
    InvalidToken,
    UnexpectedEnd,
    ExpectingExprBegin,
    ExpectingExprBody,
    Custom(String),
}

#[derive(Debug)]
pub enum TypeError {
    Custom(String),
}

#[derive(Debug)]
pub enum RuntimeError {
    DivByZero,
    Custom(String),
}
