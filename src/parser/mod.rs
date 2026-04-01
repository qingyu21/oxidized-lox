use crate::expr::Expr;
use crate::lox;
use crate::stmt::Stmt;
use crate::token::{Token, TokenType};

mod expressions;
mod statements;

#[derive(Debug, Clone, Copy)]
struct ParseError;

pub struct Parser {
    tokens: Vec<Token>,
    // Index of the next token to be parsed.
    current: usize,
    // Number of enclosing loop statements currently being parsed.
    loop_depth: usize,
}

type ParseRule = fn(&mut Parser) -> Result<Expr, ParseError>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            loop_depth: 0,
        }
    }

    // program -> declaration* EOF ;
    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => {
                    self.synchronize();
                }
            }
        }

        statements
    }

    // Parse a single expression that must consume the entire input.
    pub fn parse_expression_input(&mut self) -> Option<Expr> {
        let expr = self.expression().ok()?;

        if !self.is_at_end() {
            let _ = self.error(self.peek(), "Expect end of expression.");
            return None;
        }

        Some(expr)
    }

    // declaration -> varDecl | statement ;
    fn declaration(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(&[TokenType::Var]) {
            return self.var_declaration();
        }

        self.statement()
    }

    // varDecl -> "var" IDENTIFIER ( "=" expression )? ";" ;
    fn var_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self
            .consume(TokenType::Identifier, "Expect variable name.")?
            .clone();

        let initializer = if self.match_token(&[TokenType::Equal]) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;
        Ok(Stmt::var(name, initializer))
    }

    // Consume the expected token or report a parse error.
    fn consume(&mut self, type_: TokenType, message: &str) -> Result<&Token, ParseError> {
        if self.check(type_) {
            return Ok(self.advance());
        }

        Err(self.error(self.peek(), message))
    }

    fn error(&self, token: &Token, message: &str) -> ParseError {
        lox::token_error(token, message);
        ParseError
    }

    // Parse nested statements with loop context so `break;` is only accepted
    // inside the body of an enclosing `while` or `for`.
    fn in_loop<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        self.loop_depth += 1;
        let result = parse(self);
        self.loop_depth -= 1;
        result
    }

    // Discard tokens until the parser reaches a likely statement boundary.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            if self.current > 0 && self.previous().type_ == TokenType::Semicolon {
                return;
            }

            if self.check(TokenType::RightBrace) {
                return;
            }

            if self.can_start_declaration() {
                return;
            }

            self.advance();
        }
    }

    // Return whether the current token can begin a declaration in the current grammar.
    fn can_start_declaration(&self) -> bool {
        matches!(
            self.peek().type_,
            TokenType::Break
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::LeftBrace
                | TokenType::Identifier
                | TokenType::False
                | TokenType::True
                | TokenType::Nil
                | TokenType::Number
                | TokenType::String
                | TokenType::LeftParen
                | TokenType::Bang
                | TokenType::Minus
        )
    }

    // If the current token matches any candidate, consume it.
    fn match_token(&mut self, types: &[TokenType]) -> bool {
        for &type_ in types {
            if self.check(type_) {
                self.advance();
                return true;
            }
        }

        false
    }

    // Return whether the current token has the given type.
    fn check(&self, type_: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }

        self.peek().type_ == type_
    }

    // Consume the current token and move to the next one.
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }

        self.previous()
    }

    // Return whether the parser has reached the EOF token.
    fn is_at_end(&self) -> bool {
        self.peek().type_ == TokenType::Eof
    }

    // Borrow the current token without consuming it.
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    // Borrow the most recently consumed token.
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
}

#[cfg(test)]
mod tests;
