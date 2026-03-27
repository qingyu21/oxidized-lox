use crate::expr::Expr;
use crate::lox;
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

    pub fn parse(&mut self) -> Option<Expr> {
        let expr = self.expression().ok()?;

        if !self.is_at_end() {
            self.error(self.peek(), "Expect end of expression.");
            return None;
        }

        Some(expr)
    }

    // expression -> comma ;
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.comma()
    }

    // comma -> conditional ( "," conditional )* ;
    fn comma(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.conditional()?;

        while self.match_token(&[TokenType::Comma]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.conditional()?;
            expr = Expr::binary(expr, operator, right);
        }

        Ok(expr)
    }

    // conditional -> equality ( "?" expression ":" conditional )? ;
    fn conditional(&mut self) -> Result<Expr, ParseError> {
        let expr = self.equality()?;

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

    // primary -> NUMBER | STRING | "true" | "false" | "nil" | "(" expression ")" ;
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

        Err(self.error(self.peek(), "Expect expression."))
    }

    // Return the operand parser for a binary operator missing its left operand.
    fn missing_left_operand_rule(&self) -> Option<ParseRule> {
        match self.peek().type_ {
            TokenType::Comma => Some(Self::conditional),
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
        self.advance();

        while !self.is_at_end() {
            if self.previous().type_ == TokenType::Semicolon {
                return;
            }

            match self.peek().type_ {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,
                _ => {
                    self.advance();
                }
            }
        }
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
mod tests {
    use super::Parser;
    use crate::ast_printer::AstPrinter;
    use crate::scanner::Scanner;

    #[test]
    fn parses_binary_precedence() {
        assert_eq!(parse_to_string("1 + 2 * 3"), "(+ 1 (* 2 3))");
    }

    #[test]
    fn parses_comma_with_lowest_precedence() {
        assert_eq!(parse_to_string("1 + 2, 3 * 4"), "(, (+ 1 2) (* 3 4))");
    }

    #[test]
    fn parses_comma_as_left_associative() {
        assert_eq!(parse_to_string("1, 2, 3"), "(, (, 1 2) 3)");
    }

    #[test]
    fn parses_conditional_as_right_associative() {
        assert_eq!(
            parse_to_string("false ? 1 : true ? 2 : 3"),
            "(?: false 1 (?: true 2 3))"
        );
    }

    #[test]
    fn parses_full_expression_in_then_branch() {
        assert_eq!(parse_to_string("true ? 1, 2 : 3"), "(?: true (, 1 2) 3)");
    }

    #[test]
    fn parses_unary_and_grouping() {
        assert_eq!(
            parse_to_string("!(false == true)"),
            "(! (group (== false true)))"
        );
    }

    #[test]
    fn parses_grouped_binary_expression() {
        assert_eq!(parse_to_string("(1 + 2) * 3"), "(* (group (+ 1 2)) 3)");
    }

    #[test]
    fn discards_factor_expression_after_missing_left_operand() {
        assert_parse_error_consumes_to_end("+ 1 * 2");
    }

    #[test]
    fn discards_comparison_expression_after_missing_left_operand() {
        assert_parse_error_consumes_to_end("== 1 < 2");
    }

    #[test]
    fn discards_conditional_expression_after_missing_left_comma() {
        assert_parse_error_consumes_to_end(", false ? 1 : 2");
    }

    fn parse_to_string(source: &str) -> String {
        let tokens = Scanner::new(source).scan_tokens();
        let mut parser = Parser::new(tokens);
        let expr = parser
            .parse()
            .expect("parser should successfully parse the test input");

        AstPrinter.print(&expr)
    }

    fn assert_parse_error_consumes_to_end(source: &str) {
        let tokens = Scanner::new(source).scan_tokens();
        let mut parser = Parser::new(tokens);

        assert!(parser.parse().is_none());
        assert!(parser.is_at_end());
    }
}
