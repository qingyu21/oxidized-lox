use crate::lox;
use crate::token::{Literal, Token, TokenType};

#[allow(dead_code)]
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
            // We are at the beginning of the next lexeme.
            self.start = self.current;
            self.scan_token();
        }

        self.tokens
            .push(Token::new(TokenType::Eof, String::new(), None, self.line));

        self.tokens
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
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
            _ => {
                lox::error(self.line, "Unexpected character.");
            }
        }
    }

    fn advance(&mut self) -> char {
        // TODO(perf): For an ASCII-first scanner, a byte-oriented fast path
        // would be cheaper than creating a `chars()` iterator each time.
        // `start` and `current` are byte offsets, but we still decode one
        // Unicode scalar value at a time and advance by its UTF-8 width.
        let rest = &self.source[self.current..];
        let ch = rest
            .chars()
            .next()
            .expect("advance() called at the end of source");

        self.current += ch.len_utf8();

        ch
    }

    fn add_token(&mut self, type_: TokenType) {
        self.add_token_literal(type_, None);
    }

    // Scan operators like `!`/`!=` or `=`/`==` where the current character
    // may optionally be followed by one more expected character.
    fn add_conditional_token(
        &mut self,
        expected: char,
        matched: TokenType,
        unmatched: TokenType,
    ) {
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

    // If the next character matches `expected`, consume it and return `true`.
    // Otherwise leave the scanner position unchanged and return `false`.
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        let rest = &self.source[self.current..];
        let ch = rest
            .chars()
            .next()
            .expect("match_char() called at the end of source");

        if ch != expected {
            return false;
        }

        self.current += ch.len_utf8();
        true
    }
}
