use crate::token::{Literal, Token};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Expr {
    Binary {
        // TODO(perf): Boxing child nodes keeps the recursive enum sized, but
        // it also adds heap allocations per node. An arena/index-based AST
        // can reduce allocation overhead for larger trees.
        left: Box<Expr>,
        // TODO(perf): Storing the full token is convenient, but it carries
        // owned lexeme/literal data. A leaner AST could store only the token
        // kind plus source span information.
        operator: Token,
        right: Box<Expr>,
    },
    Grouping {
        expression: Box<Expr>,
    },
    Literal {
        value: Literal,
    },
    Variable {
        name: Token,
    },
    Conditional {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Unary {
        // TODO(perf): Storing the full token is convenient, but it carries
        // owned lexeme/literal data. A leaner AST could store only the token
        // kind plus source span information.
        operator: Token,
        right: Box<Expr>,
    },
}

impl Expr {
    // Construct a literal expression from an already-parsed literal value.
    pub fn literal(value: Literal) -> Self {
        Expr::Literal { value }
    }

    // Construct a variable expression that refers to a named binding.
    pub fn variable(name: Token) -> Self {
        Expr::Variable { name }
    }

    // Construct a grouping expression that preserves explicit parentheses.
    pub fn grouping(expression: Expr) -> Self {
        Expr::Grouping {
            expression: Box::new(expression),
        }
    }

    // Construct a conditional expression with then/else branches.
    pub fn conditional(condition: Expr, then_branch: Expr, else_branch: Expr) -> Self {
        Expr::Conditional {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    // Construct a unary expression with one operand.
    pub fn unary(operator: Token, right: Expr) -> Self {
        Expr::Unary {
            operator,
            right: Box::new(right),
        }
    }

    // Construct a binary expression with left and right operands.
    pub fn binary(left: Expr, operator: Token, right: Expr) -> Self {
        Expr::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }
    }
}
