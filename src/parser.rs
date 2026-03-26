use crate::expr::Expr;
use crate::token::{Literal, Token, TokenType};

#[allow(dead_code)]
pub struct Parser {
    tokens: Vec<Token>,
    // Index of the next token to be parsed.
    current: usize,
}

#[allow(dead_code)]
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Expr {
        let expr = self.expression();

        if !self.is_at_end() {
            panic!("Expect end of expression.");
        }

        expr
    }

    // expression -> equality ;
    fn expression(&mut self) -> Expr {
        self.equality()
    }

    // equality -> comparison ( ( "!=" | "==" ) comparison )* ;
    fn equality(&mut self) -> Expr {
        let mut expr = self.comparison();

        while self.match_token(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.comparison();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    // comparison -> term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
    fn comparison(&mut self) -> Expr {
        let mut expr = self.term();

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
            let right = self.term();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    // term -> factor ( ( "-" | "+" ) factor )* ;
    fn term(&mut self) -> Expr {
        let mut expr = self.factor();

        while self.match_token(&[TokenType::Minus, TokenType::Plus]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.factor();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    // factor -> unary ( ( "/" | "*" ) unary )* ;
    fn factor(&mut self) -> Expr {
        let mut expr = self.unary();

        while self.match_token(&[TokenType::Slash, TokenType::Star]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.unary();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    // unary -> ( "!" | "-" ) unary | primary ;
    fn unary(&mut self) -> Expr {
        if self.match_token(&[TokenType::Bang, TokenType::Minus]) {
            // TODO(perf): Cloning the full operator token copies its owned
            // lexeme/literal data. A leaner AST could store only the token
            // kind plus source span information.
            let operator = self.previous().clone();
            let right = self.unary();

            return Expr::Unary {
                operator,
                right: Box::new(right),
            };
        }

        self.primary()
    }

    // primary -> NUMBER | STRING | "true" | "false" | "nil" | "(" expression ")" ;
    fn primary(&mut self) -> Expr {
        if self.match_token(&[TokenType::False]) {
            return Expr::Literal {
                value: Literal::Bool(false),
            };
        }

        if self.match_token(&[TokenType::True]) {
            return Expr::Literal {
                value: Literal::Bool(true),
            };
        }

        if self.match_token(&[TokenType::Nil]) {
            return Expr::Literal {
                value: Literal::Nil,
            };
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

            return Expr::Literal { value };
        }

        if self.match_token(&[TokenType::LeftParen]) {
            let expr = self.expression();
            self.consume(TokenType::RightParen, "Expect ')' after expression.");

            return Expr::Grouping {
                expression: Box::new(expr),
            };
        }

        panic!("Expect expression.");
    }

    // Consume the expected token or report a parse error.
    fn consume(&mut self, type_: TokenType, message: &str) -> &Token {
        if self.check(type_) {
            return self.advance();
        }

        panic!("{message}");
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
    fn parses_unary_and_grouping() {
        assert_eq!(parse_to_string("!(false == true)"), "(! (group (== false true)))");
    }

    #[test]
    fn parses_grouped_binary_expression() {
        assert_eq!(parse_to_string("(1 + 2) * 3"), "(* (group (+ 1 2)) 3)");
    }

    fn parse_to_string(source: &str) -> String {
        let tokens = Scanner::new(source).scan_tokens();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();

        AstPrinter.print(&expr)
    }
}
