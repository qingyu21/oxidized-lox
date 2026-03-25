use crate::lox;
use crate::token::{Literal, Token, TokenType};
use std::{collections::HashMap, sync::LazyLock};

static KEYWORDS: LazyLock<HashMap<&'static str, TokenType>> = LazyLock::new(|| {
    let mut keywords = HashMap::new();
    keywords.insert("and", TokenType::And);
    keywords.insert("class", TokenType::Class);
    keywords.insert("else", TokenType::Else);
    keywords.insert("false", TokenType::False);
    keywords.insert("for", TokenType::For);
    keywords.insert("fun", TokenType::Fun);
    keywords.insert("if", TokenType::If);
    keywords.insert("nil", TokenType::Nil);
    keywords.insert("or", TokenType::Or);
    keywords.insert("print", TokenType::Print);
    keywords.insert("return", TokenType::Return);
    keywords.insert("super", TokenType::Super);
    keywords.insert("this", TokenType::This);
    keywords.insert("true", TokenType::True);
    keywords.insert("var", TokenType::Var);
    keywords.insert("while", TokenType::While);
    keywords
});

pub struct Scanner {
    // TODO(perf): Borrow `&str` here instead of owning a `String` to avoid
    // copying the entire source text when constructing the scanner.
    source: String,
    // TODO(perf): Preallocate token capacity once token density is clearer.
    tokens: Vec<Token>,
    // Byte offset of the current lexeme's first byte in `source`.
    start: usize,
    // Byte offset of the next character to read in `source`.
    current: usize,
    // 1-based source line number used for error reporting.
    line: u32,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_tokens(mut self) -> Vec<Token> {
        while !self.is_at_end() {
            // Mark the start of the next lexeme before scanning it.
            self.start = self.current;
            self.scan_token();
        }

        self.tokens
            .push(Token::new(TokenType::Eof, String::new(), None, self.line));

        self.tokens
    }

    fn scan_token(&mut self) {
        let c = self.advance();

        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            ';' => self.add_token(TokenType::Semicolon),
            '*' => self.add_token(TokenType::Star),
            '!' => self.add_conditional_token('=', TokenType::BangEqual, TokenType::Bang),
            '=' => self.add_conditional_token('=', TokenType::EqualEqual, TokenType::Equal),
            '<' => self.add_conditional_token('=', TokenType::LessEqual, TokenType::Less),
            '>' => self.add_conditional_token('=', TokenType::GreaterEqual, TokenType::Greater),
            '/' => {
                if self.match_char('/') {
                    // A comment goes until the end of the line.
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash);
                }
            }
            '"' => self.string(),
            ' ' | '\r' | '\t' => {}
            '\n' => self.line += 1,
            _ => {
                if Self::is_digit(c) {
                    self.number();
                } else if Self::is_alpha(c) {
                    self.identifier();
                } else {
                    lox::error(self.line, "Unexpected character.");
                }
            }
        }
    }

    fn add_token(&mut self, type_: TokenType) {
        self.add_token_literal(type_, None);
    }

    fn is_digit(c: char) -> bool {
        c.is_ascii_digit()
    }

    fn is_alpha(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    fn is_alpha_numeric(c: char) -> bool {
        Self::is_alpha(c) || Self::is_digit(c)
    }

    fn string(&mut self) {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            lox::error(self.line, "Unterminated string.");
            return;
        }

        // The closing quote.
        self.advance();

        // Trim the surrounding quotes.
        let value = self.source[self.start + 1..self.current - 1].to_string();
        self.add_token_literal(TokenType::String, Some(Literal::String(value)));
    }

    fn number(&mut self) {
        while Self::is_digit(self.peek()) {
            self.advance();
        }

        // Look for a fractional part.
        if self.peek() == '.' && Self::is_digit(self.peek_next()) {
            // Consume the ".".
            self.advance();

            while Self::is_digit(self.peek()) {
                self.advance();
            }
        }

        let value = self.source[self.start..self.current]
            .parse::<f64>()
            .expect("scanner produced an invalid number literal");

        self.add_token_literal(TokenType::Number, Some(Literal::Number(value)));
    }

    fn identifier(&mut self) {
        while Self::is_alpha_numeric(self.peek()) {
            self.advance();
        }

        let text = &self.source[self.start..self.current];
        let type_ = KEYWORDS.get(text).copied().unwrap_or(TokenType::Identifier);
        self.add_token(type_);
    }

    // Scan operators like `!`/`!=` or `=`/`==` where the current character
    // may optionally be followed by one more expected character.
    fn add_conditional_token(&mut self, expected: char, matched: TokenType, unmatched: TokenType) {
        let type_ = if self.match_char(expected) {
            matched
        } else {
            unmatched
        };

        self.add_token(type_);
    }

    fn add_token_literal(&mut self, type_: TokenType, literal: Option<Literal>) {
        // TODO(perf): This allocates a new `String` for every token lexeme.
        // A performance-oriented design could store spans or `&str` slices.
        let text = self.source[self.start..self.current].to_string();
        self.tokens
            .push(Token::new(type_, text, literal, self.line));
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    // Return the character at the current byte offset, if any.
    fn current_char(&self) -> Option<char> {
        self.source[self.current..].chars().next()
    }

    fn advance(&mut self) -> char {
        // TODO(perf): For an ASCII-first scanner, a byte-oriented fast path
        // would be cheaper than creating a `chars()` iterator each time.
        // `start` and `current` are byte offsets, but we still decode one
        // Unicode scalar value at a time and advance by its UTF-8 width.
        let ch = self
            .current_char()
            .expect("advance() called at the end of source");

        self.current += ch.len_utf8();

        ch
    }

    // If the next character matches `expected`, consume it and return `true`.
    // Otherwise leave the scanner position unchanged and return `false`.
    fn match_char(&mut self, expected: char) -> bool {
        let Some(ch) = self.current_char() else {
            return false;
        };

        if ch != expected {
            return false;
        }

        self.current += ch.len_utf8();
        true
    }

    // TODO(rust-idiom): Returning `Option<char>` would model EOF more
    // explicitly than using `'\0'` as a sentinel value.
    fn peek(&self) -> char {
        self.current_char().unwrap_or('\0')
    }

    // TODO(rust-idiom): Returning `Option<char>` would model the absence
    // of a second lookahead character more explicitly than using `'\0'`.
    fn peek_next(&self) -> char {
        self.source[self.current..].chars().nth(1).unwrap_or('\0')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(source: &str) -> Vec<Token> {
        Scanner::new(source).scan_tokens()
    }

    #[test]
    fn scans_string_literal() {
        let tokens = scan("\"hello\"");

        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].lexeme, "\"hello\"");
        assert_eq!(
            tokens[0].literal,
            Some(Literal::String("hello".to_string()))
        );
        assert_eq!(tokens[1].type_, TokenType::Eof);
    }

    #[test]
    fn scans_number_literal() {
        let tokens = scan("123.45");

        assert_eq!(tokens[0].type_, TokenType::Number);
        assert_eq!(tokens[0].lexeme, "123.45");

        match &tokens[0].literal {
            Some(Literal::Number(value)) => assert_eq!(*value, 123.45),
            other => panic!("expected number literal, got {other:?}"),
        }
    }

    #[test]
    fn distinguishes_keywords_from_identifiers() {
        let tokens = scan("and breakfast var");
        let types = tokens.iter().map(|token| token.type_).collect::<Vec<_>>();

        assert_eq!(
            types,
            vec![
                TokenType::And,
                TokenType::Identifier,
                TokenType::Var,
                TokenType::Eof,
            ]
        );
    }

    #[test]
    fn skips_comments_and_tracks_line_numbers() {
        let tokens = scan("// comment\nprint");

        assert_eq!(tokens[0].type_, TokenType::Print);
        assert_eq!(tokens[0].line, 2);
        assert_eq!(tokens[1].type_, TokenType::Eof);
        assert_eq!(tokens[1].line, 2);
    }
}
