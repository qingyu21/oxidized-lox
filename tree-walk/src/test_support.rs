use crate::{
    interpreter::Interpreter,
    parser::{ParsedProgram, Parser},
    resolver::Resolver,
};

pub(crate) fn parser_for(source: &str) -> Parser {
    Parser::new(source)
}

pub(crate) fn parse_statements(source: &str) -> ParsedProgram {
    let mut parser = parser_for(source);
    parser.parse()
}

pub(crate) fn resolve_statements(interpreter: &Interpreter, statements: &ParsedProgram) {
    let mut resolver = Resolver::new(interpreter);
    resolver
        .resolve_statements(statements.as_slice(), statements.expr_arena())
        .expect("test input should resolve successfully");
}
