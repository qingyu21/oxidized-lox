#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum OpCode {
    Return,
}

#[derive(Debug, Default)]
pub(crate) struct Chunk {
    code: Vec<u8>,
}

impl Chunk {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn write_chunk(&mut self, byte: u8) {
        self.code.push(byte)
    }

    pub(crate) fn code(&self) -> &[u8] {
        self.code.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::{Chunk, OpCode};

    #[test]
    fn new_chunk_starts_with_no_code() {
        let chunk = Chunk::new();

        assert!(chunk.code().is_empty());
    }

    #[test]
    fn write_chunk_appends_bytes_in_order() {
        let mut chunk = Chunk::new();

        chunk.write_chunk(OpCode::Return as u8);
        chunk.write_chunk(42);

        assert_eq!(chunk.code(), &[OpCode::Return as u8, 42]);
    }
}
