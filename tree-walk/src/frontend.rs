// Group syntax-only pieces in one place: tokens, scanner, parser, and AST
// definitions all belong to the source-to-AST frontend pipeline.
pub(crate) mod expr;
pub(crate) mod parser;
pub(crate) mod scanner;
pub(crate) mod stmt;
pub(crate) mod token;
