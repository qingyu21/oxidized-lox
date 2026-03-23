use std::fmt;

#[derive(Debug)]
pub struct Token;

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token")
    }
}
