use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
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

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Number(f64),
    #[allow(dead_code)]
    Bool(bool),
    #[allow(dead_code)]
    Nil,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub type_: TokenType,
    pub lexeme: String,
    pub literal: Option<Literal>,
    #[allow(dead_code)]
    pub line: u32,
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

impl Token {
    pub(crate) fn new(
        type_: TokenType,
        lexeme: String,
        literal: Option<Literal>,
        line: u32,
    ) -> Self {
        Token {
            type_,
            lexeme,
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
