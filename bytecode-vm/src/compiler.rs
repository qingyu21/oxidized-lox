use crate::scanner;

/// Starts the front end by initializing the scanner over the source text.
pub(crate) fn compile(source: &str) {
    scanner::init_scanner(source);
}
