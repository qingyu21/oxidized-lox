use crate::diagnostics;
use crate::token::{Literal, Token, TokenType};
use std::{collections::HashMap, rc::Rc, sync::LazyLock};

static KEYWORDS: LazyLock<HashMap<&'static str, TokenType>> = LazyLock::new(|| {
    let mut keywords = HashMap::new();
    keywords.insert("and", TokenType::And);
    keywords.insert("break", TokenType::Break);
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

pub(crate) struct Scanner {
    // Every token now points back into this shared source buffer by span,
    // which avoids allocating a fresh lexeme string for each token.
    source: Rc<String>,
    // Whole-input ASCII mode lets the hot scanning helpers read bytes
    // directly instead of decoding a one-character iterator each time.
    ascii_only: bool,
    // Byte offset of the current lexeme's first byte in `source`.
    start: usize,
    // Byte offset of the next character to read in `source`.
    current: usize,
    // 1-based source line number used for error reporting.
    line: u32,
}

impl Scanner {
    pub(crate) fn new(source: impl Into<String>) -> Self {
        let source: String = source.into();
        let ascii_only = source.is_ascii();

        Self {
            source: Rc::new(source),
            ascii_only,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub(crate) fn next_token(&mut self) -> Token {
        loop {
            self.start = self.current;

            if self.is_at_end() {
                return Token::from_source_span(
                    TokenType::Eof,
                    self.source.clone(),
                    self.current..self.current,
                    None,
                    self.line,
                );
            }

            if let Some(token) = self.scan_token() {
                return token;
            }
        }
    }

    fn scan_token(&mut self) -> Option<Token> {
        let c = self.advance();

        match c {
            '(' => Some(self.add_token(TokenType::LeftParen)),
            ')' => Some(self.add_token(TokenType::RightParen)),
            '{' => Some(self.add_token(TokenType::LeftBrace)),
            '}' => Some(self.add_token(TokenType::RightBrace)),
            ',' => Some(self.add_token(TokenType::Comma)),
            '.' => Some(self.add_token(TokenType::Dot)),
            '-' => Some(self.add_token(TokenType::Minus)),
            '+' => Some(self.add_token(TokenType::Plus)),
            ':' => Some(self.add_token(TokenType::Colon)),
            '?' => Some(self.add_token(TokenType::Question)),
            ';' => Some(self.add_token(TokenType::Semicolon)),
            '*' => Some(self.add_token(TokenType::Star)),
            '!' => Some(self.add_conditional_token('=', TokenType::BangEqual, TokenType::Bang)),
            '=' => Some(self.add_conditional_token('=', TokenType::EqualEqual, TokenType::Equal)),
            '<' => Some(self.add_conditional_token('=', TokenType::LessEqual, TokenType::Less)),
            '>' => {
                Some(self.add_conditional_token('=', TokenType::GreaterEqual, TokenType::Greater))
            }
            '/' => {
                if self.match_char('/') {
                    // A comment goes until the end of the line.
                    while !self.is_at_end() && self.peek() != Some('\n') {
                        self.advance();
                    }
                    None
                } else if self.match_char('*') {
                    self.block_comment();
                    None
                } else {
                    Some(self.add_token(TokenType::Slash))
                }
            }
            '"' => self.string(),
            ' ' | '\r' | '\t' => None,
            '\n' => {
                self.line += 1;
                None
            }
            _ => {
                if Self::is_digit(c) {
                    Some(self.number())
                } else if Self::is_alpha(c) {
                    Some(self.identifier())
                } else {
                    diagnostics::error(self.line, "Unexpected character.");
                    None
                }
            }
        }
    }

    fn add_token(&self, type_: TokenType) -> Token {
        self.add_token_literal(type_, None)
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

    fn string(&mut self) -> Option<Token> {
        while self.peek() != Some('"') && !self.is_at_end() {
            if self.peek() == Some('\n') {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            diagnostics::error(self.line, "Unterminated string.");
            return None;
        }

        // The closing quote.
        self.advance();

        // Trim the surrounding quotes.
        let value = self.source[self.start + 1..self.current - 1].to_string();
        Some(self.add_token_literal(TokenType::String, Some(Literal::String(value.into()))))
    }

    fn number(&mut self) -> Token {
        while self.peek().is_some_and(Self::is_digit) {
            self.advance();
        }

        // Look for a fractional part.
        if self.peek() == Some('.') && self.peek_next().is_some_and(Self::is_digit) {
            // Consume the ".".
            self.advance();

            while self.peek().is_some_and(Self::is_digit) {
                self.advance();
            }
        }

        let value = self.source[self.start..self.current]
            .parse::<f64>()
            .expect("scanner produced an invalid number literal");

        self.add_token_literal(TokenType::Number, Some(Literal::Number(value)))
    }

    fn identifier(&mut self) -> Token {
        while self.peek().is_some_and(Self::is_alpha_numeric) {
            self.advance();
        }

        let text = &self.source[self.start..self.current];
        let type_ = KEYWORDS.get(text).copied().unwrap_or(TokenType::Identifier);
        self.add_token(type_)
    }

    fn block_comment(&mut self) {
        // Consume until the first terminating `*/`, updating line numbers
        // along the way. Nested block comments are not supported yet.
        while !self.is_at_end() {
            if self.peek() == Some('\n') {
                self.line += 1;
                self.advance();
            } else if self.peek() == Some('*') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                return;
            } else {
                self.advance();
            }
        }

        diagnostics::error(self.line, "Unterminated block comment.");
    }

    // Scan operators like `!`/`!=` or `=`/`==` where the current character
    // may optionally be followed by one more expected character.
    fn add_conditional_token(
        &mut self,
        expected: char,
        matched: TokenType,
        unmatched: TokenType,
    ) -> Token {
        let type_ = if self.match_char(expected) {
            matched
        } else {
            unmatched
        };

        self.add_token(type_)
    }

    fn add_token_literal(&self, type_: TokenType, literal: Option<Literal>) -> Token {
        Token::from_source_span(
            type_,
            self.source.clone(),
            self.start..self.current,
            literal,
            self.line,
        )
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn ascii_byte_at(&self, offset: usize) -> Option<u8> {
        self.ascii_only
            .then_some(self.source.as_bytes())
            .and_then(|bytes| bytes.get(offset).copied())
    }

    // Return the character at the current byte offset, if any.
    fn current_char(&self) -> Option<char> {
        if let Some(byte) = self.ascii_byte_at(self.current) {
            return Some(byte as char);
        }

        self.source[self.current..].chars().next()
    }

    fn advance(&mut self) -> char {
        if let Some(byte) = self.ascii_byte_at(self.current) {
            self.current += 1;
            return byte as char;
        }

        // `start` and `current` are byte offsets, so non-ASCII input still
        // advances one Unicode scalar value at a time by UTF-8 width.
        let ch = self
            .current_char()
            .expect("advance() called at the end of source");

        self.current += ch.len_utf8();

        ch
    }

    // If the next character matches `expected`, consume it and return `true`.
    // Otherwise leave the scanner position unchanged and return `false`.
    fn match_char(&mut self, expected: char) -> bool {
        if let Some(byte) = self.ascii_byte_at(self.current) {
            if expected.is_ascii() && byte == expected as u8 {
                self.current += 1;
                return true;
            }

            return false;
        }

        let Some(ch) = self.current_char() else {
            return false;
        };

        if ch != expected {
            return false;
        }

        self.current += ch.len_utf8();
        true
    }

    // Return the current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.current_char()
    }

    // Return the next lookahead character, if there is one.
    fn peek_next(&self) -> Option<char> {
        if let Some(byte) = self
            .current
            .checked_add(1)
            .and_then(|offset| self.ascii_byte_at(offset))
        {
            return Some(byte as char);
        }

        self.source[self.current..].chars().nth(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(source: &str) -> Vec<Token> {
        let mut scanner = Scanner::new(source);
        let mut tokens = Vec::new();

        loop {
            let token = scanner.next_token();
            let is_eof = token.type_ == TokenType::Eof;
            tokens.push(token);

            if is_eof {
                return tokens;
            }
        }
    }

    #[test]
    fn scans_string_literal() {
        let tokens = scan("\"hello\"");

        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].lexeme.as_ref(), "\"hello\"");
        assert_eq!(tokens[0].literal, Some(Literal::String("hello".into())));
        assert_eq!(tokens[1].type_, TokenType::Eof);
    }

    #[test]
    fn scans_unicode_string_literal() {
        let tokens = scan("\"茶\"");

        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].lexeme.as_ref(), "\"茶\"");
        assert_eq!(tokens[0].literal, Some(Literal::String("茶".into())));
    }

    #[test]
    fn scans_number_literal() {
        let tokens = scan("123.45");

        assert_eq!(tokens[0].type_, TokenType::Number);
        assert_eq!(tokens[0].lexeme.as_ref(), "123.45");

        match &tokens[0].literal {
            Some(Literal::Number(value)) => assert_eq!(*value, 123.45),
            other => panic!("expected number literal, got {other:?}"),
        }
    }

    #[test]
    fn distinguishes_keywords_from_identifiers() {
        let tokens = scan("and break breakfast var");
        let types = tokens.iter().map(|token| token.type_).collect::<Vec<_>>();

        assert_eq!(
            types,
            vec![
                TokenType::And,
                TokenType::Break,
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

    #[test]
    fn skips_block_comments_and_tracks_line_numbers() {
        let tokens = scan("/* block\ncomment */print");

        assert_eq!(tokens[0].type_, TokenType::Print);
        assert_eq!(tokens[0].line, 2);
        assert_eq!(tokens[1].type_, TokenType::Eof);
        assert_eq!(tokens[1].line, 2);
    }

    #[test]
    fn nested_block_comments_end_at_the_first_closing_delimiter() {
        let tokens = scan("/* outer /* inner */ */");
        let types = tokens.iter().map(|token| token.type_).collect::<Vec<_>>();

        assert_eq!(
            types,
            vec![TokenType::Star, TokenType::Slash, TokenType::Eof]
        );
        assert_eq!(tokens[0].lexeme.as_ref(), "*");
        assert_eq!(tokens[1].lexeme.as_ref(), "/");
    }

    #[test]
    fn peek_returns_none_at_end_of_input() {
        let scanner = Scanner::new("");

        assert_eq!(scanner.peek(), None);
    }

    #[test]
    fn peek_next_returns_none_without_a_second_character() {
        let scanner = Scanner::new("a");

        assert_eq!(scanner.peek_next(), None);
    }

    #[test]
    fn scans_question_colon_and_two_character_operators() {
        let tokens = scan("?: != == <= >=");
        let types = tokens.iter().map(|token| token.type_).collect::<Vec<_>>();

        assert_eq!(
            types,
            vec![
                TokenType::Question,
                TokenType::Colon,
                TokenType::BangEqual,
                TokenType::EqualEqual,
                TokenType::LessEqual,
                TokenType::GreaterEqual,
                TokenType::Eof,
            ]
        );
    }

    #[test]
    fn keeps_dot_separate_when_fraction_has_no_digits() {
        let tokens = scan("123.");
        let types = tokens.iter().map(|token| token.type_).collect::<Vec<_>>();

        assert_eq!(
            types,
            vec![TokenType::Number, TokenType::Dot, TokenType::Eof]
        );

        match &tokens[0].literal {
            Some(Literal::Number(value)) => assert_eq!(*value, 123.0),
            other => panic!("expected number literal, got {other:?}"),
        }
    }
}
