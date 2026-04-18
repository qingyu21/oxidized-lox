use crate::scanner::{self, TokenType};

/// Starts the front end by initializing the scanner over the source text.
pub(crate) fn compile(source: &str) {
    let mut scanner = scanner::init_scanner(source);
    let mut line = None;

    loop {
        let token = scanner.scan_token();
        if line != Some(token.line) {
            print!("{:4} ", token.line);
            line = Some(token.line);
        } else {
            print!("   | ");
        }

        println!("{:2} '{}'", token.token_type as u8, token.lexeme());

        if matches!(token.token_type, TokenType::Eof | TokenType::Error) {
            break;
        }
    }
}
