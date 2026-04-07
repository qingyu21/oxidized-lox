#[cfg(test)]
mod ast_printer;
mod diagnostics;
mod environment;
mod frontend;
mod interpreter;
mod lox;
mod resolver;
mod runtime;
#[cfg(test)]
mod test_support;

// Keep the historical `crate::expr`, `crate::parser`, and similar paths
// working even though the concrete files now live under `src/frontend/`.
pub(crate) use frontend::{expr, parser, scanner, stmt, token};

pub use lox::{run_file, run_prompt};
