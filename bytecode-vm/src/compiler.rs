use crate::{
    chunk::{Chunk, OpCode},
    scanner::{self, Token, TokenType},
};

#[derive(Debug)]
struct Parser<'source, 'chunk> {
    scanner: scanner::Scanner<'source>,
    compiling_chunk: &'chunk mut Chunk,
    current: Option<Token<'source>>,
    previous: Option<Token<'source>>,
    had_error: bool,
    panic_mode: bool,
}

impl<'source, 'chunk> Parser<'source, 'chunk> {
    fn new(source: &'source str, chunk: &'chunk mut Chunk) -> Self {
        Self {
            scanner: scanner::init_scanner(source),
            compiling_chunk: chunk,
            current: None,
            previous: None,
            had_error: false,
            panic_mode: false,
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

    /// Consumes the current token only if it matches `token_type`.
    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.check(token_type) {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    /// Checks the current lookahead token without consuming it.
    fn check(&self, token_type: TokenType) -> bool {
        self.current
            .is_some_and(|token| token.token_type == token_type)
    }

    /// Returns the chunk currently being filled with bytecode.
    fn current_chunk(&mut self) -> &mut Chunk {
        &mut *self.compiling_chunk
    }

    /// Emits one byte tagged with the most relevant source line we have consumed.
    fn emit_byte(&mut self, byte: u8) {
        let line = self
            .previous
            .or(self.current)
            .map(|token| token.line)
            .unwrap_or(1);
        self.current_chunk().write_byte(byte, line);
    }

    /// Ends the current chunk with the temporary return instruction used this chapter.
    fn emit_return(&mut self) {
        self.emit_byte(OpCode::Return.into());
    }

    /// Finalizes compiler output for the current top-level chunk.
    fn end_compiler(&mut self) {
        self.emit_return();
    }

    /// Reports an error anchored to the current lookahead token.
    fn error_at_current(&mut self, message: &str) {
        if let Some(token) = self.current {
            self.error_at(token, message);
        } else {
            self.had_error = true;
            eprintln!("Error: {message}");
        }
    }

    /// Reports an error anchored to the most recently consumed token.
    fn error(&mut self, message: &str) {
        if let Some(token) = self.previous {
            self.error_at(token, message);
        } else {
            self.error_at_current(message);
        }
    }

    /// Formats a compiler error at `token` and suppresses cascaded errors in panic mode.
    fn error_at(&mut self, token: Token<'source>, message: &str) {
        if self.panic_mode {
            return;
        }

        self.panic_mode = true;
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
    let mut parser = Parser::new(source, chunk);

    parser.advance();
    if !parser.had_error {
        expression(&mut parser);
    }
    if !parser.had_error {
        parser.consume(TokenType::Eof, "Expect end of expression.");
    }
    parser.end_compiler();

    !parser.had_error
}

/// Entry point for Pratt parsing and bytecode emission for a single expression.
fn expression(parser: &mut Parser<'_, '_>) {
    parser.error("Expression compiler is not implemented yet.");
}

#[cfg(test)]
mod tests {
    use super::{Parser, compile};
    use crate::chunk::{Chunk, OpCode};
    use crate::scanner::TokenType;

    #[test]
    fn advance_skips_error_tokens_and_sets_error_state() {
        let mut chunk = Chunk::new();
        let mut parser = Parser::new("@123", &mut chunk);

        parser.advance();

        assert_eq!(
            parser.current.map(|token| token.token_type),
            Some(TokenType::Number)
        );
        assert_eq!(parser.current.map(|token| token.lexeme()), Some("123"));
        assert_eq!(parser.previous, None);
        assert!(parser.had_error);
        assert!(parser.panic_mode);
    }

    #[test]
    fn advance_moves_old_current_token_into_previous() {
        let mut chunk = Chunk::new();
        let mut parser = Parser::new("123 +", &mut chunk);

        parser.advance();
        parser.advance();

        assert_eq!(
            parser.previous.map(|token| token.token_type),
            Some(TokenType::Number)
        );
        assert_eq!(parser.previous.map(|token| token.lexeme()), Some("123"));
        assert_eq!(
            parser.current.map(|token| token.token_type),
            Some(TokenType::Plus)
        );
        assert_eq!(parser.current.map(|token| token.lexeme()), Some("+"));
    }

    #[test]
    fn consume_advances_when_current_token_matches() {
        let mut chunk = Chunk::new();
        let mut parser = Parser::new("123", &mut chunk);
        parser.advance();

        parser.consume(TokenType::Number, "Expect number.");

        assert_eq!(
            parser.previous.map(|token| token.token_type),
            Some(TokenType::Number)
        );
        assert_eq!(
            parser.current.map(|token| token.token_type),
            Some(TokenType::Eof)
        );
        assert!(!parser.had_error);
        assert!(!parser.panic_mode);
    }

    #[test]
    fn consume_reports_an_error_without_advanced_state_when_token_mismatches() {
        let mut chunk = Chunk::new();
        let mut parser = Parser::new("123", &mut chunk);
        parser.advance();

        parser.consume(TokenType::LeftParen, "Expect '('.");

        assert_eq!(
            parser.current.map(|token| token.token_type),
            Some(TokenType::Number)
        );
        assert_eq!(parser.previous, None);
        assert!(parser.had_error);
        assert!(parser.panic_mode);
    }

    #[test]
    fn panic_mode_suppresses_follow_up_errors() {
        let mut chunk = Chunk::new();
        let mut parser = Parser::new("123", &mut chunk);
        parser.advance();

        parser.error_at_current("first");
        let current = parser.current;
        parser.error_at(current.unwrap(), "second");

        assert!(parser.had_error);
        assert!(parser.panic_mode);
        assert_eq!(
            parser.current.map(|token| token.token_type),
            Some(TokenType::Number)
        );
    }

    #[test]
    fn compile_reports_failure_for_empty_source_until_expression_exists() {
        let mut chunk = Chunk::new();

        assert!(!compile("", &mut chunk));
        assert_eq!(chunk.code(), &[u8::from(OpCode::Return)]);
        assert_eq!(chunk.line_at(0), Some(1));
    }

    #[test]
    fn compile_reports_failure_for_non_empty_source_until_expression_exists() {
        let mut chunk = Chunk::new();

        assert!(!compile("123", &mut chunk));
        assert_eq!(chunk.code(), &[u8::from(OpCode::Return)]);
        assert_eq!(chunk.line_at(0), Some(1));
    }

    #[test]
    fn compile_reports_failure_for_scanner_errors() {
        let mut chunk = Chunk::new();

        assert!(!compile("@", &mut chunk));
        assert_eq!(chunk.code(), &[u8::from(OpCode::Return)]);
        assert_eq!(chunk.line_at(0), Some(1));
    }
}
