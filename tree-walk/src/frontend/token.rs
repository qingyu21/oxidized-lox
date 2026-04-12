use std::{
    cell::Cell,
    fmt::{self, Debug, Display},
    ops::Range,
    rc::Rc,
};

thread_local! {
    static NEXT_TOKEN_ID: Cell<u64> = const { Cell::new(1) };
}

fn next_token_id() -> u64 {
    NEXT_TOKEN_ID.with(|next_id| {
        let id = next_id.get();
        next_id.set(id + 1);
        id
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Colon,
    Question,
    Semicolon,
    Slash,
    Star,

    // One or two character tokens.
    Bang,      // !
    BangEqual, // !=
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
    Break,
    Class,
    Else,
    False,
    Fun,
    For,
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

    Eof, // End of input.
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Literal {
    String(Rc<str>),
    Number(f64),
    Bool(bool),
    Nil,
}

#[derive(Clone)]
pub(crate) struct Lexeme {
    source: Rc<String>,
    span: Range<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub(crate) id: u64,
    pub(crate) type_: TokenType,
    pub(crate) lexeme: Lexeme,
    pub(crate) literal: Option<Literal>,
    pub(crate) line: u32,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::String(value) => write!(f, "{value}"),
            Literal::Number(value) => write!(f, "{value}"),
            Literal::Bool(value) => write!(f, "{value}"),
            Literal::Nil => write!(f, "nil"),
        }
    }
}

impl Lexeme {
    fn new(source: Rc<String>, span: Range<usize>) -> Self {
        Self { source, span }
    }

    pub(crate) fn to_rc(&self) -> Rc<str> {
        self.as_ref().into()
    }
}

impl AsRef<str> for Lexeme {
    fn as_ref(&self) -> &str {
        &self.source[self.span.clone()]
    }
}

impl Display for Lexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl Debug for Lexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_ref(), f)
    }
}

impl PartialEq for Lexeme {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Eq for Lexeme {}

impl Token {
    pub(crate) fn new(
        type_: TokenType,
        lexeme: impl Into<String>,
        literal: Option<Literal>,
        line: u32,
    ) -> Self {
        let source = Rc::new(lexeme.into());
        let span = 0..source.len();

        Self::from_source_span(type_, source, span, literal, line)
    }

    pub(crate) fn from_source_span(
        type_: TokenType,
        source: Rc<String>,
        span: Range<usize>,
        literal: Option<Literal>,
        line: u32,
    ) -> Self {
        Token {
            id: next_token_id(),
            type_,
            lexeme: Lexeme::new(source, span),
            literal,
            line,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.literal {
            Some(literal) => write!(f, "{} {} {}", self.type_, self.lexeme, literal),
            None => write!(f, "{} {} null", self.type_, self.lexeme),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Scanner;

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
    fn scanner_tokens_share_the_same_source_backing() {
        let tokens = scan("print tea;");

        assert!(
            Rc::ptr_eq(&tokens[0].lexeme.source, &tokens[1].lexeme.source),
            "scanner tokens should reference the same shared source buffer"
        );
        assert!(
            Rc::ptr_eq(&tokens[1].lexeme.source, &tokens[2].lexeme.source),
            "punctuation tokens should also reuse the shared source buffer"
        );
    }
}
