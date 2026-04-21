use crate::{
    scanner::{self, TokenType},
    vm::InterpretResult,
};

/// Temporarily drives the scanner and prints tokens until real compilation arrives.
pub(crate) fn compile(source: &str) -> InterpretResult {
    dump_tokens(source)
}

/// Placeholder front-end used until chapter 17 wires scanned tokens into bytecode emission.
fn dump_tokens(source: &str) -> InterpretResult {
    let mut scanner = scanner::init_scanner(source);
    let mut last_line = None;

    loop {
        let token = scanner.scan_token();
        print_token_line_prefix(token.line, &mut last_line);
        print_scanned_token(token.token_type, token.lexeme());

        match token.token_type {
            TokenType::Eof => return InterpretResult::InterpretOk,
            TokenType::Error => return InterpretResult::InterpretCompileError,
            _ => {}
        }
    }
}

fn print_token_line_prefix(line: usize, last_line: &mut Option<usize>) {
    if *last_line != Some(line) {
        print!("{line:4} ");
        *last_line = Some(line);
    } else {
        print!("   | ");
    }
}

fn print_scanned_token(token_type: TokenType, lexeme: &str) {
    println!("{token_type:?} '{lexeme}'");
}

#[cfg(test)]
mod tests {
    use super::{compile, print_token_line_prefix};
    use crate::vm::InterpretResult;

    #[test]
    fn compile_reports_ok_for_empty_source() {
        assert_eq!(compile(""), InterpretResult::InterpretOk);
    }

    #[test]
    fn compile_reports_ok_for_supported_punctuation() {
        assert_eq!(
            compile(" // comment\n(){},.-+;/*!===<=>=\n"),
            InterpretResult::InterpretOk
        );
    }

    #[test]
    fn compile_reports_ok_for_supported_literals() {
        assert_eq!(
            compile("123 45.67 \"lox\" \"multi\nline\""),
            InterpretResult::InterpretOk
        );
    }

    #[test]
    fn compile_reports_ok_for_identifiers() {
        assert_eq!(
            compile("print foo _tmp var123"),
            InterpretResult::InterpretOk
        );
    }

    #[test]
    fn compile_reports_compile_error_for_placeholder_scanner_errors() {
        assert_eq!(compile("@"), InterpretResult::InterpretCompileError);
    }

    #[test]
    fn line_prefix_tracks_the_most_recent_line() {
        let mut last_line = None;

        print_token_line_prefix(7, &mut last_line);
        assert_eq!(last_line, Some(7));

        print_token_line_prefix(7, &mut last_line);
        assert_eq!(last_line, Some(7));

        print_token_line_prefix(8, &mut last_line);
        assert_eq!(last_line, Some(8));
    }
}
