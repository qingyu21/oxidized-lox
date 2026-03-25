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
                lox::error(self.line, "Unexpected character.");
            }
        }
    }

    fn add_token(&mut self, type_: TokenType) {
        self.add_token_literal(type_, None);
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
}
