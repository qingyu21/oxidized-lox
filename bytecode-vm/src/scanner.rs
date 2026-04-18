/// Identifies which kind of lexeme the scanner produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals.
    Identifier,
    String,
    Number,
    // Keywords.
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Error,
    Eof,
}

/// A token is a view into the original source plus its kind and line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Token<'source> {
    pub(crate) token_type: TokenType,
    start: &'source str,
    length: usize,
    pub(crate) line: usize,
}

impl<'source> Token<'source> {
    pub(crate) fn lexeme(self) -> &'source str {
        &self.start[..self.length]
    }

    fn error(line: usize, message: &'source str) -> Self {
        Self {
            token_type: TokenType::Error,
            start: message,
            length: message.len(),
            line,
        }
    }
}

/// Tracks scanner progress through the current source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Scanner<'source> {
    // These slices always point at suffixes of the original source, mirroring
    // the C version's start/current pointers without owning any string data.
    start: &'source str,
    current: &'source str,
    line: usize,
}

impl<'source> Scanner<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            start: source,
            current: source,
            line: 1,
        }
    }

    fn make_token(self, token_type: TokenType) -> Token<'source> {
        Token {
            token_type,
            start: self.start,
            length: self.start.len() - self.current.len(),
            line: self.line,
        }
    }

    fn error_token(self, message: &'source str) -> Token<'source> {
        Token::error(self.line, message)
    }

    fn is_at_end(self) -> bool {
        self.current.is_empty()
    }

    /// Returns the next token in the source stream.
    pub(crate) fn scan_token(&mut self) -> Token<'source> {
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        self.error_token("Unexpected character.")
    }
}

/// Prepares scanner state for a new chunk of source text.
pub(crate) fn init_scanner(source: &str) -> Scanner<'_> {
    Scanner::new(source)
}

#[cfg(test)]
mod tests {
    use super::{TokenType, init_scanner};

    #[test]
    fn init_scanner_starts_at_the_first_character_on_line_one() {
        let source = "print 123;";
        let scanner = init_scanner(source);

        assert_eq!(scanner.start, source);
        assert_eq!(scanner.current, source);
        assert_eq!(scanner.line, 1);
    }

    #[test]
    fn initial_scan_token_returns_eof_at_the_current_position() {
        let source = "";
        let mut scanner = init_scanner(source);
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Eof);
        assert_eq!(token.lexeme(), "");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn non_empty_input_returns_the_placeholder_error_token() {
        let mut scanner = init_scanner("print 123;");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Error);
        assert_eq!(token.lexeme(), "Unexpected character.");
        assert_eq!(token.line, 1);
    }
}
