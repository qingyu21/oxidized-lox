use crate::expr::Expr;

#[allow(dead_code)]
pub struct AstPrinter;

#[allow(dead_code)]
impl AstPrinter {
    pub fn print(&self, expr: &Expr) -> String {
        match expr {
            Expr::Assign { name, value } => format!("(= {} {})", name.lexeme, self.print(value)),
            Expr::Binary {
                left,
                operator,
                right,
            } => self.parenthesize(&operator.lexeme, [left.as_ref(), right.as_ref()]),
            Expr::Grouping { expression } => self.parenthesize("group", [expression.as_ref()]),
            Expr::Literal { value } => value.to_string(),
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => self.parenthesize(
                "?:",
                [
                    condition.as_ref(),
                    then_branch.as_ref(),
                    else_branch.as_ref(),
                ],
            ),
            Expr::Variable { name } => name.lexeme.clone(),
            Expr::Unary { operator, right } => {
                self.parenthesize(&operator.lexeme, [right.as_ref()])
            }
        }
    }

    fn parenthesize<'a>(&self, name: &str, exprs: impl IntoIterator<Item = &'a Expr>) -> String {
        let mut result = String::from("(");
        result.push_str(name);

        for expr in exprs {
            result.push(' ');
            result.push_str(&self.print(expr));
        }

        result.push(')');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::AstPrinter;
    use crate::expr::Expr;
    use crate::token::{Literal, Token, TokenType};

    #[test]
    fn prints_ast_in_book_style() {
        let expression = Expr::Binary {
            left: Box::new(Expr::Unary {
                operator: token(TokenType::Minus, "-"),
                right: Box::new(Expr::Literal {
                    value: Literal::Number(123.0),
                }),
            }),
            operator: token(TokenType::Star, "*"),
            right: Box::new(Expr::Grouping {
                expression: Box::new(Expr::Literal {
                    value: Literal::Number(45.67),
                }),
            }),
        };

        let printer = AstPrinter;
        assert_eq!(printer.print(&expression), "(* (- 123) (group 45.67))");
    }

    fn token(type_: TokenType, lexeme: &str) -> Token {
        Token::new(type_, lexeme.to_string(), None, 1)
    }
}
