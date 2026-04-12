use crate::expr::{Expr, ExprArena};

pub(crate) struct AstPrinter;

impl AstPrinter {
    pub(crate) fn print(&self, expr: &Expr, expr_arena: &ExprArena) -> String {
        match expr {
            Expr::Assign { name, value } => format!(
                "(= {} {})",
                name.lexeme,
                self.print(expr_arena.get(*value), expr_arena)
            ),
            Expr::Binary {
                left,
                operator,
                right,
            } => self.parenthesize(
                operator.lexeme.as_ref(),
                [expr_arena.get(*left), expr_arena.get(*right)],
                expr_arena,
            ),
            Expr::Call {
                callee, arguments, ..
            } => self.parenthesize(
                "call",
                std::iter::once(expr_arena.get(*callee))
                    .chain(arguments.iter().map(|argument| expr_arena.get(*argument))),
                expr_arena,
            ),
            Expr::Get { object, name } => format!(
                "(. {} {})",
                self.print(expr_arena.get(*object), expr_arena),
                name.lexeme
            ),
            Expr::Grouping { expression } => {
                self.parenthesize("group", [expr_arena.get(*expression)], expr_arena)
            }
            Expr::Literal { value } => value.to_string(),
            Expr::Logical {
                left,
                operator,
                right,
            } => self.parenthesize(
                operator.lexeme.as_ref(),
                [expr_arena.get(*left), expr_arena.get(*right)],
                expr_arena,
            ),
            Expr::Set {
                object,
                name,
                value,
            } => {
                format!(
                    "(set {} {} {})",
                    self.print(expr_arena.get(*object), expr_arena),
                    name.lexeme,
                    self.print(expr_arena.get(*value), expr_arena)
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
                    expr_arena.get(*condition),
                    expr_arena.get(*then_branch),
                    expr_arena.get(*else_branch),
                ],
                expr_arena,
            ),
            Expr::This { keyword } => keyword.lexeme.to_string(),
            Expr::Variable { name } => name.lexeme.to_string(),
            Expr::Unary { operator, right } => self.parenthesize(
                operator.lexeme.as_ref(),
                [expr_arena.get(*right)],
                expr_arena,
            ),
        }
    }

    fn parenthesize<'a>(
        &self,
        name: &str,
        exprs: impl IntoIterator<Item = &'a Expr>,
        expr_arena: &ExprArena,
    ) -> String {
        let mut result = String::from("(");
        result.push_str(name);

        for expr in exprs {
            result.push(' ');
            result.push_str(&self.print(expr, expr_arena));
        }

        result.push(')');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::AstPrinter;
    use crate::expr::{Expr, ExprArena};
    use crate::token::{Literal, Token, TokenType};

    #[test]
    fn prints_ast_in_book_style() {
        let mut exprs = ExprArena::new();
        let unary_operand = exprs.alloc(Expr::Literal {
            value: Literal::Number(123.0),
        });
        let grouped_literal = exprs.alloc(Expr::Literal {
            value: Literal::Number(45.67),
        });
        let left = exprs.alloc(Expr::Unary {
            operator: token(TokenType::Minus, "-"),
            right: unary_operand,
        });
        let right = exprs.alloc(Expr::Grouping {
            expression: grouped_literal,
        });
        let expression = Expr::Binary {
            left,
            operator: token(TokenType::Star, "*"),
            right,
        };

        let printer = AstPrinter;
        assert_eq!(
            printer.print(&expression, &exprs),
            "(* (- 123) (group 45.67))"
        );
    }

    fn token(type_: TokenType, lexeme: &str) -> Token {
        Token::new(type_, lexeme.to_string(), None, 1)
    }
}
