use crate::expr::Expr;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Stmt {
    Expression { expression: Expr },
    Print { expression: Expr },
}

#[allow(dead_code)]
impl Stmt {
    // Construct a statement that evaluates an expression for its side effects.
    pub fn expression(expression: Expr) -> Self {
        Stmt::Expression { expression }
    }

    // Construct a statement that evaluates and prints an expression.
    pub fn print(expression: Expr) -> Self {
        Stmt::Print { expression }
    }
}
