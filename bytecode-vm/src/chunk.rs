use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum OpCode {
    /// Encoding: [OP_CONSTANT][constant_index:u8]
    /// Stack: pushes constants[constant_index].
    /// Meaning: load a literal value from the chunk's constant table.
    Constant,

    /// Encoding: [OP_RETURN]
    /// Stack: finishes the current chunk.
    /// Meaning: return from the current function or script.
    Return,
}

impl OpCode {
    /// Returns the human-readable opcode name used by the disassembler.
    pub(crate) fn mnemonic(self) -> &'static str {
        match self {
            Self::Constant => "OP_CONSTANT",
            Self::Return => "OP_RETURN",
        }
    }
}

impl From<OpCode> for u8 {
    fn from(opcode: OpCode) -> Self {
        opcode as u8
    }
}

impl TryFrom<u8> for OpCode {
    type Error = u8;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            value if value == u8::from(Self::Constant) => Ok(Self::Constant),
            value if value == u8::from(Self::Return) => Ok(Self::Return),
            _ => Err(byte),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Chunk {
    // `code` and `lines` stay in lockstep: each byte in the instruction stream
    // records the source line it came from.
    code: Vec<u8>,
    lines: Vec<usize>,
    constants: Vec<Value>,
}

impl Chunk {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub(crate) fn write_opcode(&mut self, opcode: OpCode, line: usize) {
        self.write_byte(opcode.into(), line)
    }

    pub(crate) fn code(&self) -> &[u8] {
        &self.code
    }

    /// Returns the source line recorded for the byte at `offset`.
    pub(crate) fn line_at(&self, offset: usize) -> Option<usize> {
        self.lines.get(offset).copied()
    }

    pub(crate) fn constants(&self) -> &[Value] {
        &self.constants
    }

    /// Adds a value to the constant table and returns its index.
    pub(crate) fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::{Chunk, OpCode};

    #[test]
    fn new_chunk_starts_with_no_code() {
        let chunk = Chunk::new();

        assert!(chunk.code().is_empty());
        assert_eq!(chunk.line_at(0), None);
        assert!(chunk.constants().is_empty());
    }

    #[test]
    fn write_opcode_and_bytes_append_in_order_with_line_info() {
        let mut chunk = Chunk::new();

        chunk.write_opcode(OpCode::Return, 123);
        chunk.write_byte(42, 123);

        assert_eq!(chunk.code(), &[u8::from(OpCode::Return), 42]);
        assert_eq!(chunk.line_at(0), Some(123));
        assert_eq!(chunk.line_at(1), Some(123));
    }

    #[test]
    fn add_constant_returns_the_inserted_index_and_stores_the_value() {
        let mut chunk = Chunk::new();

        let first = chunk.add_constant(1.2);
        let second = chunk.add_constant(3.4);

        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(chunk.constants(), &[1.2, 3.4]);
    }

    #[test]
    fn opcode_round_trips_through_its_byte_encoding() {
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Constant)),
            Ok(OpCode::Constant)
        );
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Return)),
            Ok(OpCode::Return)
        );
    }
}
