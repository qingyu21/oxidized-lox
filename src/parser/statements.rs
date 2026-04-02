use super::{ParseError, Parser};
use crate::expr::Expr;
use crate::stmt::Stmt;
use crate::token::{Literal, TokenType};

impl Parser {
    // statement -> breakStmt | forStmt | ifStmt | printStmt | whileStmt | block | exprStmt ;
    pub(super) fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(&[TokenType::Break]) {
            return self.break_statement();
        }

        if self.match_token(&[TokenType::For]) {
            return self.for_statement();
        }

        if self.match_token(&[TokenType::If]) {
            return self.if_statement();
        }

        if self.match_token(&[TokenType::Print]) {
            return self.print_statement();
        }

        if self.match_token(&[TokenType::While]) {
            return self.while_statement();
        }

        if self.match_token(&[TokenType::LeftBrace]) {
            return Ok(Stmt::block(self.block()?));
        }

        self.expression_statement()
    }

    // breakStmt -> "break" ";" ;
    pub(super) fn break_statement(&mut self) -> Result<Stmt, ParseError> {
        let keyword = self.previous().clone();
        self.consume(TokenType::Semicolon, "Expect ';' after 'break'.")?;

        if self.loop_depth == 0 {
            return Err(self.error(&keyword, "Can't use 'break' outside of a loop."));
        }

        Ok(Stmt::break_stmt())
    }

    // forStmt -> "for" "(" ( varDecl | exprStmt | ";" ) expression? ";" expression? ")" statement ;
    pub(super) fn for_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.")?;

        // Parse the initializer clause, which may be omitted, a `var`
        // declaration, or a plain expression statement.
        let initializer = if self.match_token(&[TokenType::Semicolon]) {
            None
        } else if self.match_token(&[TokenType::Var]) {
            Some(self.var_declaration()?)
        } else {
            Some(self.expression_statement()?)
        };

        // Parse the loop condition and increment clauses. Either may be
        // omitted in a C-style `for` loop.
        let condition = if !self.check(TokenType::Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::Semicolon, "Expect ';' after loop condition.")?;

        let increment = if !self.check(TokenType::RightParen) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::RightParen, "Expect ')' after for clauses.")?;

        // Parse the original loop body, then desugar the whole construct into
        // the primitive statements the interpreter already knows how to run.
        let mut body = self.in_loop(Self::statement)?;

        if let Some(increment) = increment {
            // TODO(control-flow): If Lox later grows `continue`, desugared
            // `for` loops need to preserve the increment clause on continue.
            // A naive continue that exits this block early would skip the
            // increment, which is not the behavior users expect from `for`.
            body = Stmt::block(vec![body, Stmt::expression(increment)]);
        }

        let condition = condition.unwrap_or_else(|| Expr::literal(Literal::Bool(true)));
        body = Stmt::while_stmt(condition, body);

        if let Some(initializer) = initializer {
            body = Stmt::block(vec![initializer, body]);
        }

        Ok(body)
    }

    // ifStmt -> "if" "(" expression ")" statement ( "else" statement )? ;
    pub(super) fn if_statement(&mut self) -> Result<Stmt, ParseError> {
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

    // printStmt -> "print" expression ";" ;
    pub(super) fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(Stmt::print(value))
    }

    // whileStmt -> "while" "(" expression ")" statement ;
    pub(super) fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let body = self.in_loop(Self::statement)?;
        Ok(Stmt::while_stmt(condition, body))
    }

    // block -> "{" declaration* "}" ;
    pub(super) fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();

        // Keep parsing nested declarations until the matching `}` so blocks
        // can contain the same mix of statements and declarations as the top level.
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => self.synchronize(),
            }
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.")?;
        Ok(statements)
    }

    // exprStmt -> expression ";" ;
    pub(super) fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        Ok(Stmt::expression(expr))
    }
}
