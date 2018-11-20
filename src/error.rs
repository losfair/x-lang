#[derive(Debug)]
pub enum TypeError {
    Custom(String),
}

#[derive(Debug)]
pub enum RuntimeError {
    DivByZero,
    Custom(String),
}
