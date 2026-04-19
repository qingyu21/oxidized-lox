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
    /// Returns the token's lexeme as a slice of the original source text.
    pub(crate) fn lexeme(&self) -> &'source str {
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
    source: &'source str,
    start: usize,
    current: usize,
    line: usize,
}

impl<'source> Scanner<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    fn make_token(&self, token_type: TokenType) -> Token<'source> {
        Token {
            token_type,
            start: &self.source[self.start..],
            length: self.current - self.start,
            line: self.line,
        }
    }

    fn error_token(&self, message: &'source str) -> Token<'source> {
        Token::error(self.line, message)
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn peek(&self) -> Option<char> {
        self.source[self.current..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        self.source[self.current..].chars().nth(1)
    }

    fn advance(&mut self) -> char {
        let next = self.source[self.current..].chars().next().unwrap();
        self.current += next.len_utf8();
        next
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || !self.source[self.current..].starts_with(expected) {
            return false;
        }

        self.current += expected.len_utf8();
        true
    }

    fn make_conditional_token(
        &mut self,
        expected: char,
        matched: TokenType,
        fallback: TokenType,
    ) -> Token<'source> {
        let token_type = if self.match_char(expected) {
            matched
        } else {
            fallback
        };
        self.make_token(token_type)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\r' | '\t') => {
                    self.advance();
                }
                Some('\n') => {
                    self.line += 1;
                    self.advance();
                }
                Some('/') if self.peek_next() == Some('/') => {
                    while !matches!(self.peek(), Some('\n') | None) {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    /// Returns the next token in the source stream.
    /// This chapter stage skips leading whitespace, recognizes punctuation
    /// tokens, and reports everything else as an error until longer lexemes
    /// land.
    pub(crate) fn scan_token(&mut self) -> Token<'source> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        let current = self.advance();
        match current {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            ';' => self.make_token(TokenType::Semicolon),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            '-' => self.make_token(TokenType::Minus),
            '+' => self.make_token(TokenType::Plus),
            '/' => self.make_token(TokenType::Slash),
            '*' => self.make_token(TokenType::Star),
            '!' => self.make_conditional_token('=', TokenType::BangEqual, TokenType::Bang),
            '=' => self.make_conditional_token('=', TokenType::EqualEqual, TokenType::Equal),
            '<' => self.make_conditional_token('=', TokenType::LessEqual, TokenType::Less),
            '>' => self.make_conditional_token('=', TokenType::GreaterEqual, TokenType::Greater),
            _ => self.error_token("Unexpected character."),
        }
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

        assert_eq!(scanner.source, source);
        assert_eq!(scanner.start, 0);
        assert_eq!(scanner.current, 0);
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
    fn non_punctuation_input_returns_an_error_token() {
        let mut scanner = init_scanner("print 123;");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Error);
        assert_eq!(token.lexeme(), "Unexpected character.");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn single_character_punctuation_scans_as_one_token() {
        let cases = [
            ("(", TokenType::LeftParen),
            (")", TokenType::RightParen),
            ("{", TokenType::LeftBrace),
            ("}", TokenType::RightBrace),
            (";", TokenType::Semicolon),
            (",", TokenType::Comma),
            (".", TokenType::Dot),
            ("-", TokenType::Minus),
            ("+", TokenType::Plus),
            ("/", TokenType::Slash),
            ("*", TokenType::Star),
        ];

        for (source, expected) in cases {
            let mut scanner = init_scanner(source);
            let token = scanner.scan_token();

            assert_eq!(token.token_type, expected);
            assert_eq!(token.lexeme(), source);
        }
    }

    #[test]
    fn one_or_two_character_punctuation_scans_correctly() {
        let cases = [
            ("!", TokenType::Bang),
            ("!=", TokenType::BangEqual),
            ("=", TokenType::Equal),
            ("==", TokenType::EqualEqual),
            ("<", TokenType::Less),
            ("<=", TokenType::LessEqual),
            (">", TokenType::Greater),
            (">=", TokenType::GreaterEqual),
        ];

        for (source, expected) in cases {
            let mut scanner = init_scanner(source);
            let token = scanner.scan_token();

            assert_eq!(token.token_type, expected);
            assert_eq!(token.lexeme(), source);
        }
    }

    #[test]
    fn leading_whitespace_is_skipped_before_scanning_the_next_token() {
        let mut scanner = init_scanner(" \r\t(");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::LeftParen);
        assert_eq!(token.lexeme(), "(");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn newlines_are_skipped_and_increment_the_token_line() {
        let mut scanner = init_scanner(" \n\n(");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::LeftParen);
        assert_eq!(token.lexeme(), "(");
        assert_eq!(token.line, 3);
    }

    #[test]
    fn input_with_only_whitespace_returns_eof_on_the_last_line() {
        let mut scanner = init_scanner(" \n\t\n");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Eof);
        assert_eq!(token.lexeme(), "");
        assert_eq!(token.line, 3);
    }

    #[test]
    fn line_comments_are_skipped_before_scanning_the_next_token() {
        let mut scanner = init_scanner("// comment\n(");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::LeftParen);
        assert_eq!(token.lexeme(), "(");
        assert_eq!(token.line, 2);
    }

    #[test]
    fn single_slash_is_still_scanned_as_a_token() {
        let mut scanner = init_scanner("/");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Slash);
        assert_eq!(token.lexeme(), "/");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn comment_without_trailing_newline_is_skipped_until_eof() {
        let mut scanner = init_scanner("// comment");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Eof);
        assert_eq!(token.lexeme(), "");
        assert_eq!(token.line, 1);
    }
}
