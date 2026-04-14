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
    code: Vec<u8>,
    constants: Vec<Value>,
}

impl Chunk {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn write_byte(&mut self, byte: u8) {
        self.code.push(byte)
    }

    pub(crate) fn write_opcode(&mut self, opcode: OpCode) {
        self.write_byte(opcode.into())
    }

    pub(crate) fn code(&self) -> &[u8] {
        &self.code
    }

    pub(crate) fn constants(&self) -> &[Value] {
        &self.constants
    }

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
        assert!(chunk.constants().is_empty());
    }

    #[test]
    fn write_opcode_and_bytes_append_in_order() {
        let mut chunk = Chunk::new();

        chunk.write_opcode(OpCode::Return);
        chunk.write_byte(42);

        assert_eq!(chunk.code(), &[u8::from(OpCode::Return), 42]);
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
