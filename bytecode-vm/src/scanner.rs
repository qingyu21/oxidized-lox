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
    Question,
    Colon,
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

    fn is_alpha(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    fn is_digit(c: char) -> bool {
        c.is_ascii_digit()
    }

    /// After the first byte narrows the candidate set, compare only the
    /// remaining suffix to separate keywords from plain identifiers.
    fn check_keyword(&self, start: usize, rest: &str, token_type: TokenType) -> TokenType {
        let length = rest.len();
        let lexeme_length = self.current - self.start;
        let rest_start = self.start + start;
        let rest_end = rest_start + length;

        if lexeme_length == start + length && &self.source[rest_start..rest_end] == rest {
            return token_type;
        }

        TokenType::Identifier
    }

    /// Mirrors the book's trie-like dispatch without allocating a temporary string.
    fn identifier_type(&self) -> TokenType {
        let lexeme = &self.source.as_bytes()[self.start..self.current];

        match lexeme[0] {
            b'a' => self.check_keyword(1, "nd", TokenType::And),
            b'c' => self.check_keyword(1, "lass", TokenType::Class),
            b'e' => self.check_keyword(1, "lse", TokenType::Else),
            b'f' => {
                if lexeme.len() > 1 {
                    match lexeme[1] {
                        b'a' => self.check_keyword(2, "lse", TokenType::False),
                        b'o' => self.check_keyword(2, "r", TokenType::For),
                        b'u' => self.check_keyword(2, "n", TokenType::Fun),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            b'i' => self.check_keyword(1, "f", TokenType::If),
            b'n' => self.check_keyword(1, "il", TokenType::Nil),
            b'o' => self.check_keyword(1, "r", TokenType::Or),
            b'p' => self.check_keyword(1, "rint", TokenType::Print),
            b'r' => self.check_keyword(1, "eturn", TokenType::Return),
            b's' => self.check_keyword(1, "uper", TokenType::Super),
            b't' => {
                if lexeme.len() > 1 {
                    match lexeme[1] {
                        b'h' => self.check_keyword(2, "is", TokenType::This),
                        b'r' => self.check_keyword(2, "ue", TokenType::True),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            b'v' => self.check_keyword(1, "ar", TokenType::Var),
            b'w' => self.check_keyword(1, "hile", TokenType::While),
            _ => TokenType::Identifier,
        }
    }

    fn identifier(&mut self) -> Token<'source> {
        while matches!(self.peek(), Some(c) if Self::is_alpha(c) || Self::is_digit(c)) {
            self.advance();
        }

        self.make_token(self.identifier_type())
    }

    /// Consumes a full string literal and keeps embedded newlines in sync with diagnostics.
    fn string(&mut self) -> Token<'source> {
        while !matches!(self.peek(), Some('"') | None) {
            if self.peek() == Some('\n') {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        }

        self.advance();
        self.make_token(TokenType::String)
    }

    fn number(&mut self) -> Token<'source> {
        while matches!(self.peek(), Some(c) if Self::is_digit(c)) {
            self.advance();
        }

        if self.peek() == Some('.') && matches!(self.peek_next(), Some(c) if Self::is_digit(c)) {
            self.advance();

            while matches!(self.peek(), Some(c) if Self::is_digit(c)) {
                self.advance();
            }
        }

        self.make_token(TokenType::Number)
    }

    /// Skips insignificant trivia, including `//` comments, before scanning the next token.
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
    /// This chapter stage skips leading whitespace and recognizes punctuation,
    /// literal, and identifier tokens, including the book's trie-style
    /// keyword matching.
    pub(crate) fn scan_token(&mut self) -> Token<'source> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        let current = self.advance();

        if Self::is_alpha(current) {
            return self.identifier();
        }

        if Self::is_digit(current) {
            return self.number();
        }

        match current {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            ';' => self.make_token(TokenType::Semicolon),
            '?' => self.make_token(TokenType::Question),
            ':' => self.make_token(TokenType::Colon),
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
            '"' => self.string(),
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
    fn unsupported_character_returns_an_error_token() {
        let mut scanner = init_scanner("@");
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
            ("?", TokenType::Question),
            (":", TokenType::Colon),
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

    #[test]
    fn identifiers_scan_as_identifier_tokens() {
        let mut scanner = init_scanner("lox");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Identifier);
        assert_eq!(token.lexeme(), "lox");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn identifiers_can_start_with_an_underscore() {
        let mut scanner = init_scanner("_tmp");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Identifier);
        assert_eq!(token.lexeme(), "_tmp");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn digits_are_allowed_after_the_first_identifier_character() {
        let mut scanner = init_scanner("var123");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Identifier);
        assert_eq!(token.lexeme(), "var123");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn keywords_scan_as_their_keyword_token_types() {
        let cases = [
            ("and", TokenType::And),
            ("class", TokenType::Class),
            ("else", TokenType::Else),
            ("false", TokenType::False),
            ("for", TokenType::For),
            ("fun", TokenType::Fun),
            ("if", TokenType::If),
            ("nil", TokenType::Nil),
            ("or", TokenType::Or),
            ("print", TokenType::Print),
            ("return", TokenType::Return),
            ("super", TokenType::Super),
            ("this", TokenType::This),
            ("true", TokenType::True),
            ("var", TokenType::Var),
            ("while", TokenType::While),
        ];

        for (source, expected) in cases {
            let mut scanner = init_scanner(source);
            let token = scanner.scan_token();

            assert_eq!(token.token_type, expected);
            assert_eq!(token.lexeme(), source);
        }
    }

    #[test]
    fn identifiers_that_extend_keywords_stay_identifiers() {
        let cases = ["andy", "classy", "format", "superb", "trueish"];

        for source in cases {
            let mut scanner = init_scanner(source);
            let token = scanner.scan_token();

            assert_eq!(token.token_type, TokenType::Identifier);
            assert_eq!(token.lexeme(), source);
        }
    }

    #[test]
    fn integer_numbers_scan_as_number_tokens() {
        let mut scanner = init_scanner("123");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Number);
        assert_eq!(token.lexeme(), "123");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn fractional_numbers_scan_as_number_tokens() {
        let mut scanner = init_scanner("123.45");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Number);
        assert_eq!(token.lexeme(), "123.45");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn dot_without_trailing_digit_is_not_consumed_as_part_of_a_number() {
        let mut scanner = init_scanner("123.");

        let number = scanner.scan_token();
        let dot = scanner.scan_token();

        assert_eq!(number.token_type, TokenType::Number);
        assert_eq!(number.lexeme(), "123");
        assert_eq!(dot.token_type, TokenType::Dot);
        assert_eq!(dot.lexeme(), ".");
    }

    #[test]
    fn strings_scan_as_string_tokens() {
        let mut scanner = init_scanner("\"lox\"");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::String);
        assert_eq!(token.lexeme(), "\"lox\"");
        assert_eq!(token.line, 1);
    }

    #[test]
    fn multiline_strings_increment_the_scanner_line() {
        let mut scanner = init_scanner("\"lox\nlang\"");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::String);
        assert_eq!(token.lexeme(), "\"lox\nlang\"");
        assert_eq!(token.line, 2);
    }

    #[test]
    fn unterminated_strings_return_an_error_token() {
        let mut scanner = init_scanner("\"unterminated");
        let token = scanner.scan_token();

        assert_eq!(token.token_type, TokenType::Error);
        assert_eq!(token.lexeme(), "Unterminated string.");
        assert_eq!(token.line, 1);
    }
}
