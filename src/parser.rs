use crate::expr::Expr;
use crate::lox;
use crate::stmt::Stmt;
use crate::token::{Literal, Token, TokenType};

#[derive(Debug, Clone, Copy)]
struct ParseError;

#[allow(dead_code)]
pub struct Parser {
    tokens: Vec<Token>,
    // Index of the next token to be parsed.
    current: usize,
}

type ParseRule = fn(&mut Parser) -> Result<Expr, ParseError>;

#[allow(dead_code)]
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
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

    // statement -> ifStmt | printStmt | block | exprStmt ;
    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(&[TokenType::If]) {
            return self.if_statement();
        }

        if self.match_token(&[TokenType::Print]) {
            return self.print_statement();
        }

        if self.match_token(&[TokenType::LeftBrace]) {
            return Ok(Stmt::block(self.block()?));
        }

        self.expression_statement()
    }

    // ifStmt -> "if" "(" expression ")" statement ( "else" statement )? ;
    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after if condition.")?;

        let then_branch = self.statement()?;
        let else_branch = if self.match_token(&[TokenType::Else]) {
            Some(self.statement()?)
        } else {
            None
        };

        Ok(Stmt::if_stmt(condition, then_branch, else_branch))
    }

    // block -> "{" declaration* "}" ;
    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => self.synchronize(),
            }
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.")?;
        Ok(statements)
    }

    // printStmt -> "print" expression ";" ;
    fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(Stmt::print(value))
    }

    // exprStmt -> expression ";" ;
    fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        Ok(Stmt::expression(expr))
    }

    // expression -> comma ;
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.comma()
    }

    // comma -> assignment ( "," assignment )* ;
    fn comma(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.assignment()?;

        while self.match_token(&[TokenType::Comma]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.assignment()?;
            expr = Expr::binary(expr, operator, right);
        }

        Ok(expr)
    }

    // assignment -> conditional ( "=" assignment )? ;
    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.conditional()?;

        if self.match_token(&[TokenType::Equal]) {
            let equals = self.previous().clone();
            let value = self.assignment()?;

            if let Expr::Variable { name } = expr {
                return Ok(Expr::assign(name, value));
            }

            let _ = self.error(&equals, "Invalid assignment target.");
        }

        Ok(expr)
    }

    // conditional -> logic_or ( "?" expression ":" conditional )? ;
    fn conditional(&mut self) -> Result<Expr, ParseError> {
        let expr = self.logic_or()?;

        if self.match_token(&[TokenType::Question]) {
            let then_branch = self.expression()?;
            self.consume(
                TokenType::Colon,
                "Expect ':' after then branch of conditional expression.",
            )?;
            let else_branch = self.conditional()?;
            return Ok(Expr::conditional(expr, then_branch, else_branch));
        }

        Ok(expr)
    }

    // logic_or -> logic_and ( "or" logic_and )* ;
    fn logic_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.logic_and()?;

        while self.match_token(&[TokenType::Or]) {
            let operator = self.previous().clone();
            let right = self.logic_and()?;
            expr = Expr::logical(expr, operator, right);
        }

        Ok(expr)
    }

    // logic_and -> equality ( "and" equality )* ;
    fn logic_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;

        while self.match_token(&[TokenType::And]) {
            let operator = self.previous().clone();
            let right = self.equality()?;
            expr = Expr::logical(expr, operator, right);
        }

        Ok(expr)
    }

    // equality -> comparison ( ( "!=" | "==" ) comparison )* ;
    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;

        while self.match_token(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.comparison()?;
            expr = Expr::binary(expr, operator, right);
        }

        Ok(expr)
    }

    // comparison -> term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;

        while self.match_token(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.term()?;
            expr = Expr::binary(expr, operator, right);
        }

        Ok(expr)
    }

    // term -> factor ( ( "-" | "+" ) factor )* ;
    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;

        while self.match_token(&[TokenType::Minus, TokenType::Plus]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.factor()?;
            expr = Expr::binary(expr, operator, right);
        }

        Ok(expr)
    }

    // factor -> unary ( ( "/" | "*" ) unary )* ;
    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;

        while self.match_token(&[TokenType::Slash, TokenType::Star]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.unary()?;
            expr = Expr::binary(expr, operator, right);
        }

        Ok(expr)
    }

    // unary -> ( "!" | "-" ) unary | primary ;
    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(&[TokenType::Bang, TokenType::Minus]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.unary()?;
            return Ok(Expr::unary(operator, right));
        }

        self.primary()
    }

    // primary -> "true" | "false" | "nil" | NUMBER | STRING | "(" expression ")" | IDENTIFIER ;
    fn primary(&mut self) -> Result<Expr, ParseError> {
        if let Some(right_operand) = self.missing_left_operand_rule() {
            return self.missing_left_operand(right_operand);
        }

        if self.match_token(&[TokenType::False]) {
            return Ok(Expr::literal(Literal::Bool(false)));
        }

        if self.match_token(&[TokenType::True]) {
            return Ok(Expr::literal(Literal::Bool(true)));
        }

        if self.match_token(&[TokenType::Nil]) {
            return Ok(Expr::literal(Literal::Nil));
        }

        if self.match_token(&[TokenType::Number, TokenType::String]) {
            // TODO(perf): Cloning literal payloads duplicates owned data such
            // as string contents. A leaner AST could store spans or interned
            // values instead of copying each literal.
            let value = self
                .previous()
                .literal
                .clone()
                .expect("literal token should carry a literal value");

            return Ok(Expr::literal(value));
        }

        if self.match_token(&[TokenType::LeftParen]) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression.")?;
            return Ok(Expr::grouping(expr));
        }

        if self.match_token(&[TokenType::Identifier]) {
            return Ok(Expr::variable(self.previous().clone()));
        }

        Err(self.error(self.peek(), "Expect expression."))
    }

    // Return the operand parser for a binary operator missing its left operand.
    fn missing_left_operand_rule(&self) -> Option<ParseRule> {
        match self.peek().type_ {
            TokenType::Comma => Some(Self::conditional),
            TokenType::Or => Some(Self::logic_and),
            TokenType::And => Some(Self::equality),
            TokenType::BangEqual | TokenType::EqualEqual => Some(Self::comparison),
            TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Less
            | TokenType::LessEqual => Some(Self::term),
            TokenType::Plus => Some(Self::factor),
            TokenType::Slash | TokenType::Star => Some(Self::unary),
            _ => None,
        }
    }

    // Report a missing left operand and discard the right operand of the operator.
    fn missing_left_operand(&mut self, right_operand: ParseRule) -> Result<Expr, ParseError> {
        let operator = self.advance().clone();
        let error = self.error(&operator, "Missing left-hand operand.");
        let _ = right_operand(self);
        Err(error)
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
            TokenType::Var
                | TokenType::If
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
