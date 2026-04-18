/// Tracks scanner progress through the current source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Scanner<'source> {
    // These slices always point at suffixes of the original source, mirroring
    // the C version's start/current pointers without owning any string data.
    start: &'source str,
    current: &'source str,
    line: usize,
}

impl<'source> Scanner<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            start: source,
            current: source,
            line: 1,
        }
    }
}

/// Prepares scanner state for a new chunk of source text.
pub(crate) fn init_scanner(source: &str) -> Scanner<'_> {
    Scanner::new(source)
}

#[cfg(test)]
mod tests {
    use super::init_scanner;

    #[test]
    fn init_scanner_starts_at_the_first_character_on_line_one() {
        let source = "print 123;";
        let scanner = init_scanner(source);

        assert_eq!(scanner.start, source);
        assert_eq!(scanner.current, source);
        assert_eq!(scanner.line, 1);
    }
}
