use crate::expr::Expr;

pub(crate) struct AstPrinter;

impl AstPrinter {
    pub(crate) fn print(&self, expr: &Expr) -> String {
        match expr {
            Expr::Assign { name, value } => format!("(= {} {})", name.lexeme, self.print(value)),
            Expr::Binary {
                left,
                operator,
                right,
            } => self.parenthesize(&operator.lexeme, [left.as_ref(), right.as_ref()]),
            Expr::Call {
                callee, arguments, ..
            } => self.parenthesize(
                "call",
                std::iter::once(callee.as_ref()).chain(arguments.iter()),
            ),
            Expr::Get { object, name } => format!("(. {} {})", self.print(object), name.lexeme),
            Expr::Grouping { expression } => self.parenthesize("group", [expression.as_ref()]),
            Expr::Literal { value } => value.to_string(),
            Expr::Logical {
                left,
                operator,
                right,
            } => self.parenthesize(&operator.lexeme, [left.as_ref(), right.as_ref()]),
            Expr::Set {
                object,
                name,
                value,
            } => {
                format!(
                    "(set {} {} {})",
                    self.print(object),
                    name.lexeme,
                    self.print(value)
                )
            }
            Expr::Super { method, .. } => format!("(super {})", method.lexeme),
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
            Expr::This { keyword } => keyword.lexeme.to_string(),
            Expr::Variable { name } => name.lexeme.to_string(),
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
