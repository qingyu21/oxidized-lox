use crate::diagnostics;
use crate::expr::{Expr, ExprArena, ExprArenaRef};
use crate::scanner::Scanner;
use crate::stmt::{FunctionDecl, Stmt};
use crate::token::{Token, TokenType};
use std::{mem, ops::Deref};

mod expressions;
mod statements;

#[derive(Debug, Clone, Copy)]
struct ParseError;

const MAX_ARITY: usize = 255;

pub(crate) struct ParsedProgram {
    // Owns the arena that backs every nested `ExprRef` inside this statement
    // list, so later resolver/interpreter passes can traverse safely.
    expr_arena: ExprArenaRef,
    statements: Vec<Stmt>,
}

pub(crate) struct ParsedExpression {
    // Owns the arena that backs every nested `ExprRef` inside this root
    // expression, so later passes can resolve child handles safely.
    expr_arena: ExprArenaRef,
    expression: Expr,
}

pub(crate) struct Parser {
    scanner: Scanner,
    exprs: ExprArena,
    // Token currently being considered by the parser.
    current_token: Token,
    // Most recently consumed token, used by Pratt parsing and error recovery.
    previous_token: Token,
    has_previous: bool,
    // Number of enclosing loop statements currently being parsed.
    loop_depth: usize,
    // Number of enclosing function bodies currently being parsed.
    function_depth: usize,
}

type ParseRule = fn(&mut Parser) -> Result<Expr, ParseError>;

impl Parser {
    pub(crate) fn new(source: impl Into<String>) -> Self {
        let mut scanner = Scanner::new(source);
        let current_token = scanner.next_token();

        Self {
            scanner,
            exprs: ExprArena::new(),
            previous_token: Token::new(TokenType::Eof, "", None, current_token.line),
            current_token,
            has_previous: false,
            loop_depth: 0,
            function_depth: 0,
        }
    }

    // program -> declaration* EOF ;
    pub(crate) fn parse(&mut self) -> ParsedProgram {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => {
                    self.synchronize();
                }
            }
        }
        let exprs = self.take_exprs();
        attach_exprs_to_statements(&exprs, &mut statements);
        ParsedProgram::new(exprs, statements)
    }

    // Parse a single expression that must consume the entire input.
    pub(crate) fn parse_expression_input(&mut self) -> Option<ParsedExpression> {
        let expr = self.expression().ok()?;

        if !self.is_at_end() {
            let _ = self.error(self.peek(), "Expect end of expression.");
            return None;
        }

        Some(ParsedExpression::new(self.take_exprs(), expr))
    }

    // declaration -> classDecl | funDecl | varDecl | statement ;
    fn declaration(&mut self) -> Result<Stmt, ParseError> {
        if self.match_one(TokenType::Class) {
            return self.class_declaration();
        }

        // TODO(function-expr): If Lox gains anonymous function expressions
        // like `fun (...) { ... }`, this branch will need one-token lookahead.
        // `fun` followed by an identifier stays a declaration, while `fun`
        // followed by `(` should fall through to expression parsing so
        // statement forms like `fun () {};` are treated as expression statements.
        if self.match_one(TokenType::Fun) {
            return Ok(Stmt::function(self.function_declaration("function")?));
        }

        if self.match_one(TokenType::Var) {
            return self.var_declaration();
        }

        self.statement()
    }

    // classDecl -> "class" IDENTIFIER ( "<" IDENTIFIER )? "{" function* "}" ;
    fn class_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self
            .consume(TokenType::Identifier, "Expect class name.")?
            .clone();
        let superclass = if self.match_one(TokenType::Less) {
            let name = self
                .consume(TokenType::Identifier, "Expect superclass name.")?
                .clone();
            Some(Expr::variable(name))
        } else {
            None
        };
        self.consume(TokenType::LeftBrace, "Expect '{' before class body.")?;

        let mut methods = Vec::new();
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            methods.push(self.function_declaration("method")?);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after class body.")?;
        Ok(Stmt::class(name, superclass, methods))
    }

    // funDecl -> "fun" function ;
    // function -> IDENTIFIER "(" parameters? ")" block ;
    fn function_declaration(&mut self, kind: &str) -> Result<FunctionDecl, ParseError> {
        // Parse the declared name and the opening parenthesis before the
        // parameter list.
        let name = self
            .consume(TokenType::Identifier, &format!("Expect {kind} name."))?
            .clone();
        self.consume(
            TokenType::LeftParen,
            &format!("Expect '(' after {kind} name."),
        )?;

        // Parse zero or more comma-separated parameter names, while enforcing
        // the same maximum arity limit used for calls.
        let mut params = Vec::new();
        if !self.check(TokenType::RightParen) {
            loop {
                if params.len() >= MAX_ARITY {
                    let message = format!("Can't have more than {MAX_ARITY} parameters.");
                    let _ = self.error(self.peek(), &message);
                }

                params.push(
                    self.consume(TokenType::Identifier, "Expect parameter name.")?
                        .clone(),
                );

                if !self.match_one(TokenType::Comma) {
                    break;
                }
            }
        }

        // Parse the braced function body and wrap the whole declaration into
        // a function statement node.
        self.consume(TokenType::RightParen, "Expect ')' after parameters.")?;
        self.consume(
            TokenType::LeftBrace,
            &format!("Expect '{{' before {kind} body."),
        )?;
        let body = self.in_function(Self::block)?;
        Ok(FunctionDecl::new(name, params, body))
    }

    // varDecl -> "var" IDENTIFIER ( "=" expression )? ";" ;
    fn var_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self
            .consume(TokenType::Identifier, "Expect variable name.")?
            .clone();

        let initializer = if self.match_one(TokenType::Equal) {
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
        diagnostics::token_error(token, message);
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

    // Parse nested declarations with function context so `return;` is only
    // accepted inside the body of an enclosing function declaration.
    fn in_function<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        self.function_depth += 1;
        let result = parse(self);
        self.function_depth -= 1;
        result
    }

    // Discard tokens until the parser reaches a likely statement boundary.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            if self.has_previous && self.previous().type_ == TokenType::Semicolon {
                return;
            }

            if self.check(TokenType::RightBrace) {
                return;
            }

            if self.can_resume_after_error() {
                return;
            }

            self.advance();
        }
    }

    // Return whether the current token is a plausible place to resume parsing
    // after panic-mode error recovery.
    fn can_resume_after_error(&self) -> bool {
        matches!(
            self.peek().type_,
            TokenType::Break
                | TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::Return
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

    fn match_one(&mut self, type_: TokenType) -> bool {
        if self.check(type_) {
            self.advance();
            true
        } else {
            false
        }
    }

    // If the current token matches any candidate, consume it.
    fn match_token(&mut self, types: &[TokenType]) -> bool {
        for &type_ in types {
            if self.match_one(type_) {
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
            self.previous_token = mem::replace(&mut self.current_token, self.scanner.next_token());
        } else {
            self.previous_token = self.current_token.clone();
        }

        self.has_previous = true;
        self.previous()
    }

    // Return whether the parser has reached the EOF token.
    fn is_at_end(&self) -> bool {
        self.peek().type_ == TokenType::Eof
    }

    // Borrow the current token without consuming it.
    fn peek(&self) -> &Token {
        &self.current_token
    }

    // Borrow the most recently consumed token.
    fn previous(&self) -> &Token {
        debug_assert!(self.has_previous, "previous() called before advance()");
        &self.previous_token
    }

    fn take_exprs(&mut self) -> ExprArenaRef {
        mem::take(&mut self.exprs).into_shared()
    }
}

impl ParsedProgram {
    fn new(exprs: ExprArenaRef, statements: Vec<Stmt>) -> Self {
        Self {
            expr_arena: exprs,
            statements,
        }
    }

    pub(crate) fn as_slice(&self) -> &[Stmt] {
        &self.statements
    }

    pub(crate) fn expr_arena(&self) -> &crate::expr::ExprArena {
        self.expr_arena.as_ref()
    }
}

impl Deref for ParsedProgram {
    type Target = [Stmt];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl ParsedExpression {
    fn new(exprs: ExprArenaRef, expression: Expr) -> Self {
        Self {
            expr_arena: exprs,
            expression,
        }
    }

    pub(crate) fn as_expr(&self) -> &Expr {
        &self.expression
    }

    pub(crate) fn expr_arena(&self) -> &crate::expr::ExprArena {
        self.expr_arena.as_ref()
    }
}

fn attach_exprs_to_statements(exprs: &ExprArenaRef, statements: &mut [Stmt]) {
    for statement in statements {
        attach_exprs_to_statement(exprs, statement);
    }
}

fn attach_exprs_to_statement(exprs: &ExprArenaRef, statement: &mut Stmt) {
    match statement {
        Stmt::Block { statements } => attach_exprs_to_statements(exprs, statements),
        Stmt::Class { methods, .. } => {
            for method in methods {
                attach_exprs_to_function(exprs, method);
            }
        }
        Stmt::Function(function) => attach_exprs_to_function(exprs, function),
        Stmt::If {
            then_branch,
            else_branch,
            ..
        } => {
            attach_exprs_to_statement(exprs, then_branch);
            if let Some(else_branch) = else_branch {
                attach_exprs_to_statement(exprs, else_branch);
            }
        }
        Stmt::While { body, .. } => attach_exprs_to_statement(exprs, body),
        Stmt::Break
        | Stmt::Expression { .. }
        | Stmt::Print { .. }
        | Stmt::Return { .. }
        | Stmt::Var { .. } => {}
    }
}

fn attach_exprs_to_function(exprs: &ExprArenaRef, function: &mut FunctionDecl) {
    debug_assert!(
        function.expr_arena.is_none(),
        "parser should attach each function arena exactly once before later passes run"
    );
    function.expr_arena = Some(exprs.clone());
    attach_exprs_to_statements(exprs, &mut function.body);
}

impl Deref for ParsedExpression {
    type Target = Expr;

    fn deref(&self) -> &Self::Target {
        self.as_expr()
    }
}

#[cfg(test)]
mod tests;
