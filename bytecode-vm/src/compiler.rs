use crate::{
    chunk::{Chunk, OpCode},
    scanner::{self, Token, TokenType},
};

/// Function pointer type for prefix parsers that can operate on any compiler state lifetimes.
type PrefixParseFn = for<'source, 'chunk> fn(&mut Parser<'source, 'chunk>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Assignment,
    Unary,
}

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

    /// Emits a constant-loading instruction for `value`.
    fn emit_constant(&mut self, value: f64) {
        let line = self
            .previous
            .or(self.current)
            .map(|token| token.line)
            .unwrap_or(1);
        let result = self.current_chunk().write_constant(value, line);

        if result.is_err() {
            self.error("Too many constants in one chunk.");
        }
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

/// Returns the prefix parser associated with the consumed token type, if any.
fn get_prefix_parser(token_type: TokenType) -> Option<PrefixParseFn> {
    match token_type {
        TokenType::LeftParen => Some(grouping),
        TokenType::Minus => Some(unary),
        TokenType::Number => Some(number),
        _ => None,
    }
}

/// Entry point for Pratt parsing and bytecode emission for a single expression.
fn expression(parser: &mut Parser<'_, '_>) {
    parse_precedence(parser, Precedence::Assignment);
}

/// Parses any prefix expression at `precedence` or higher.
fn parse_precedence(parser: &mut Parser<'_, '_>, _precedence: Precedence) {
    parser.advance();

    let Some(token_type) = parser.previous.map(|token| token.token_type) else {
        parser.error("Expect expression.");
        return;
    };
    let Some(prefix_parser) = get_prefix_parser(token_type) else {
        parser.error("Expect expression.");
        return;
    };

    prefix_parser(parser);
}

/// Compiles a parenthesized grouping by recursively compiling the inner expression.
fn grouping(parser: &mut Parser<'_, '_>) {
    expression(parser);
    parser.consume(TokenType::RightParen, "Expect ')' after expression.");
}

/// Compiles a prefix unary operator after recursively compiling its operand.
fn unary(parser: &mut Parser<'_, '_>) {
    let Some(operator_type) = parser.previous.map(|token| token.token_type) else {
        parser.error("Expect unary operator.");
        return;
    };

    parse_precedence(parser, Precedence::Unary);

    match operator_type {
        TokenType::Minus => parser.emit_byte(OpCode::Negate.into()),
        _ => unreachable!("unary parser is only registered for unary operators"),
    }
}

/// Compiles a consumed number literal token into a constant-loading instruction.
fn number(parser: &mut Parser<'_, '_>) {
    let Some(token) = parser.previous else {
        parser.error("Expect number.");
        return;
    };
    let Ok(value) = token.lexeme().parse::<f64>() else {
        parser.error("Invalid number literal.");
        return;
    };

    parser.emit_constant(value);
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
    fn compile_emits_constant_and_return_for_number_literal() {
        let mut chunk = Chunk::new();

        assert!(compile("123.45", &mut chunk));
        assert_eq!(
            chunk.code(),
            &[u8::from(OpCode::Constant), 0, u8::from(OpCode::Return)]
        );
        assert_eq!(chunk.constants(), &[123.45]);
        assert_eq!(chunk.line_at(0), Some(1));
        assert_eq!(chunk.line_at(1), Some(1));
        assert_eq!(chunk.line_at(2), Some(1));
    }

    #[test]
    fn compile_parenthesized_number_emits_only_inner_expression_bytecode() {
        let mut chunk = Chunk::new();

        assert!(compile("(123)", &mut chunk));
        assert_eq!(
            chunk.code(),
            &[u8::from(OpCode::Constant), 0, u8::from(OpCode::Return)]
        );
        assert_eq!(chunk.constants(), &[123.0]);
        assert_eq!(chunk.line_at(0), Some(1));
        assert_eq!(chunk.line_at(1), Some(1));
        assert_eq!(chunk.line_at(2), Some(1));
    }

    #[test]
    fn compile_emits_negate_after_operand_for_unary_minus() {
        let mut chunk = Chunk::new();

        assert!(compile("-123", &mut chunk));
        assert_eq!(
            chunk.code(),
            &[
                u8::from(OpCode::Constant),
                0,
                u8::from(OpCode::Negate),
                u8::from(OpCode::Return),
            ]
        );
        assert_eq!(chunk.constants(), &[123.0]);
        assert_eq!(chunk.line_at(0), Some(1));
        assert_eq!(chunk.line_at(1), Some(1));
        assert_eq!(chunk.line_at(2), Some(1));
        assert_eq!(chunk.line_at(3), Some(1));
    }

    #[test]
    fn compile_supports_nested_unary_minus() {
        let mut chunk = Chunk::new();

        assert!(compile("--123", &mut chunk));
        assert_eq!(
            chunk.code(),
            &[
                u8::from(OpCode::Constant),
                0,
                u8::from(OpCode::Negate),
                u8::from(OpCode::Negate),
                u8::from(OpCode::Return),
            ]
        );
        assert_eq!(chunk.constants(), &[123.0]);
    }

    #[test]
    fn compile_reports_failure_for_missing_right_paren() {
        let mut chunk = Chunk::new();

        assert!(!compile("(123", &mut chunk));
        assert_eq!(
            chunk.code(),
            &[u8::from(OpCode::Constant), 0, u8::from(OpCode::Return)]
        );
        assert_eq!(chunk.constants(), &[123.0]);
    }

    #[test]
    fn compile_reports_failure_for_missing_unary_operand() {
        let mut chunk = Chunk::new();

        assert!(!compile("-", &mut chunk));
        assert_eq!(
            chunk.code(),
            &[u8::from(OpCode::Negate), u8::from(OpCode::Return)]
        );
    }

    #[test]
    fn compile_reports_failure_for_empty_source() {
        let mut chunk = Chunk::new();

        assert!(!compile("", &mut chunk));
        assert_eq!(chunk.code(), &[u8::from(OpCode::Return)]);
        assert_eq!(chunk.line_at(0), Some(1));
    }

    #[test]
    fn compile_reports_failure_for_token_without_prefix_parser() {
        let mut chunk = Chunk::new();

        assert!(!compile("+", &mut chunk));
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
