use crate::{
    chunk::Chunk,
    scanner::{self, Token, TokenType},
};

#[derive(Debug)]
struct Parser<'source> {
    scanner: scanner::Scanner<'source>,
    current: Option<Token<'source>>,
    previous: Option<Token<'source>>,
    had_error: bool,
}

impl<'source> Parser<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            scanner: scanner::init_scanner(source),
            current: None,
            previous: None,
            had_error: false,
        }
    }

    /// Primes the scanner and skips over scanner-produced error tokens.
    fn advance(&mut self) {
        self.previous = self.current;

        loop {
            let token = self.scanner.scan_token();
            self.current = Some(token);

            if token.token_type != TokenType::Error {
                break;
            }

            self.error_at_current(token.lexeme());
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.check(token_type) {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.current
            .is_some_and(|token| token.token_type == token_type)
    }

    fn error_at_current(&mut self, message: &str) {
        if let Some(token) = self.current {
            self.error_at(token, message);
        } else {
            self.had_error = true;
            eprintln!("Error: {message}");
        }
    }

    fn error_at(&mut self, token: Token<'source>, message: &str) {
        eprint!("[line {}] Error", token.line);
        match token.token_type {
            TokenType::Eof => eprint!(" at end"),
            TokenType::Error => {}
            _ => eprint!(" at '{}'", token.lexeme()),
        }
        eprintln!(": {message}");

        self.had_error = true;
    }
}

/// Compiles source into `chunk`, returning whether compilation succeeded.
pub(crate) fn compile(source: &str, chunk: &mut Chunk) -> bool {
    let mut parser = Parser::new(source);

    parser.advance();
    if !parser.had_error {
        expression(&mut parser, chunk);
    }
    if !parser.had_error {
        parser.consume(TokenType::Eof, "Expect end of expression.");
    }

    !parser.had_error
}

/// Chapter 17 fills this in with Pratt parsing and bytecode emission.
fn expression(parser: &mut Parser<'_>, _chunk: &mut Chunk) {
    parser.error_at_current("Expression compiler is not implemented yet.");
}

#[cfg(test)]
mod tests {
    use super::compile;
    use crate::chunk::Chunk;

    #[test]
    fn compile_reports_failure_for_empty_source_until_expression_exists() {
        let mut chunk = Chunk::new();

        assert!(!compile("", &mut chunk));
        assert!(chunk.code().is_empty());
    }

    #[test]
    fn compile_reports_failure_for_non_empty_source_until_expression_exists() {
        let mut chunk = Chunk::new();

        assert!(!compile("123", &mut chunk));
        assert!(chunk.code().is_empty());
    }

    #[test]
    fn compile_reports_failure_for_scanner_errors() {
        let mut chunk = Chunk::new();

        assert!(!compile("@", &mut chunk));
        assert!(chunk.code().is_empty());
    }
}
