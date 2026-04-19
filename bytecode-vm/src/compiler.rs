use crate::{
    scanner::{self, TokenType},
    vm::InterpretResult,
};

/// Temporarily drives the scanner and prints tokens until real compilation arrives.
pub(crate) fn compile(source: &str) -> InterpretResult {
    let mut scanner = scanner::init_scanner(source);
    let mut last_line = None;

    loop {
        let token = scanner.scan_token();
        if last_line != Some(token.line) {
            print!("{:4} ", token.line);
            last_line = Some(token.line);
        } else {
            print!("   | ");
        }

        println!("{:2} '{}'", token.token_type as u8, token.lexeme());

        match token.token_type {
            TokenType::Eof => return InterpretResult::InterpretOk,
            TokenType::Error => return InterpretResult::InterpretCompileError,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compile;
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
    fn compile_reports_compile_error_for_placeholder_scanner_errors() {
        assert_eq!(compile("print 1;"), InterpretResult::InterpretCompileError);
    }
}
