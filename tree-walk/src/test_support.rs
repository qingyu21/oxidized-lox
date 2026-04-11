use crate::{
    interpreter::Interpreter,
    parser::{ParsedProgram, Parser},
    resolver::Resolver,
    scanner::Scanner,
    token::Token,
};

pub(crate) fn scan_tokens(source: &str) -> Vec<Token> {
    Scanner::new(source).scan_tokens()
}

pub(crate) fn parser_for(source: &str) -> Parser {
    Parser::new(scan_tokens(source))
}

pub(crate) fn parse_statements(source: &str) -> ParsedProgram {
    let mut parser = parser_for(source);
    parser.parse()
}

pub(crate) fn resolve_statements(interpreter: &Interpreter, statements: &[crate::stmt::Stmt]) {
    let mut resolver = Resolver::new(interpreter);
    resolver
        .resolve_statements(statements)
        .expect("test input should resolve successfully");
}
