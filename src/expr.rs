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
    Unary {
        // TODO(perf): Storing the full token is convenient, but it carries
        // owned lexeme/literal data. A leaner AST could store only the token
        // kind plus source span information.
        operator: Token,
        right: Box<Expr>,
    },
}
