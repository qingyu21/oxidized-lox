use super::{ParseError, ParseRule, Parser};
use crate::expr::Expr;
use crate::token::{Literal, TokenType};

impl Parser {
    // expression -> comma ;
    pub(super) fn expression(&mut self) -> Result<Expr, ParseError> {
        self.comma()
    }

    // comma -> assignment ( "," assignment )* ;
    pub(super) fn comma(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn assignment(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn conditional(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn logic_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.logic_and()?;

        while self.match_token(&[TokenType::Or]) {
            let operator = self.previous().clone();
            let right = self.logic_and()?;
            expr = Expr::logical(expr, operator, right);
        }

        Ok(expr)
    }

    // logic_and -> equality ( "and" equality )* ;
    pub(super) fn logic_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;

        while self.match_token(&[TokenType::And]) {
            let operator = self.previous().clone();
            let right = self.equality()?;
            expr = Expr::logical(expr, operator, right);
        }

        Ok(expr)
    }

    // equality -> comparison ( ( "!=" | "==" ) comparison )* ;
    pub(super) fn equality(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn comparison(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn term(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn factor(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn unary(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn primary(&mut self) -> Result<Expr, ParseError> {
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
    pub(super) fn missing_left_operand_rule(&self) -> Option<ParseRule> {
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
    pub(super) fn missing_left_operand(
        &mut self,
        right_operand: ParseRule,
    ) -> Result<Expr, ParseError> {
        let operator = self.advance().clone();
        let error = self.error(&operator, "Missing left-hand operand.");
        let _ = right_operand(self);
        Err(error)
    }
}
