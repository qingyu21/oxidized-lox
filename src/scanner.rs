use crate::token::Token;

pub struct Scanner;

impl Scanner {
    pub fn new(_source: &str) -> Self {
        Scanner
    }

    pub fn scan_tokens(&self) -> Vec<Token> {
        Vec::new()
    }
}
