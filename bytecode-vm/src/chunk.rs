use crate::value::Value;

const SHORT_CONSTANT_MAX_INDEX: usize = u8::MAX as usize;
const LONG_CONSTANT_MAX_INDEX: usize = 0xFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum OpCode {
    /// Encoding: [OP_CONSTANT][constant_index:u8]
    /// Stack: pushes constants[constant_index].
    /// Meaning: load a literal value from the chunk's constant table.
    Constant,

    /// Encoding: [OP_CONSTANT_LONG][index_low:u8][index_mid:u8][index_high:u8]
    /// Stack: pushes constants[constant_index].
    /// Meaning: load a literal value when the constant table index needs 24 bits.
    ConstantLong,

    /// Encoding: [OP_NIL]
    /// Stack: pushes nil.
    /// Meaning: load the nil literal without using the constant table.
    Nil,

    /// Encoding: [OP_TRUE]
    /// Stack: pushes true.
    /// Meaning: load the true literal without using the constant table.
    True,

    /// Encoding: [OP_FALSE]
    /// Stack: pushes false.
    /// Meaning: load the false literal without using the constant table.
    False,

    /// Encoding: [OP_ADD]
    /// Stack: pops two values, then pushes their sum.
    /// Meaning: implement binary `+` for numeric values.
    Add,

    /// Encoding: [OP_SUBTRACT]
    /// Stack: pops two values, then pushes left - right.
    /// Meaning: implement binary `-` for numeric values.
    Subtract,

    /// Encoding: [OP_MULTIPLY]
    /// Stack: pops two values, then pushes their product.
    /// Meaning: implement binary `*` for numeric values.
    Multiply,

    /// Encoding: [OP_DIVIDE]
    /// Stack: pops two values, then pushes left / right.
    /// Meaning: implement binary `/` for numeric values.
    Divide,

    /// Encoding: [OP_NOT]
    /// Stack: pops one value, then pushes whether it is falsey.
    /// Meaning: implement unary logical not.
    Not,

    /// Encoding: [OP_NEGATE]
    /// Stack: pops one value, then pushes its arithmetic negation.
    /// Meaning: implement unary minus for numeric values.
    Negate,

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
            Self::ConstantLong => "OP_CONSTANT_LONG",
            Self::Nil => "OP_NIL",
            Self::True => "OP_TRUE",
            Self::False => "OP_FALSE",
            Self::Add => "OP_ADD",
            Self::Subtract => "OP_SUBTRACT",
            Self::Multiply => "OP_MULTIPLY",
            Self::Divide => "OP_DIVIDE",
            Self::Not => "OP_NOT",
            Self::Negate => "OP_NEGATE",
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
            value if value == u8::from(Self::ConstantLong) => Ok(Self::ConstantLong),
            value if value == u8::from(Self::Nil) => Ok(Self::Nil),
            value if value == u8::from(Self::True) => Ok(Self::True),
            value if value == u8::from(Self::False) => Ok(Self::False),
            value if value == u8::from(Self::Add) => Ok(Self::Add),
            value if value == u8::from(Self::Subtract) => Ok(Self::Subtract),
            value if value == u8::from(Self::Multiply) => Ok(Self::Multiply),
            value if value == u8::from(Self::Divide) => Ok(Self::Divide),
            value if value == u8::from(Self::Not) => Ok(Self::Not),
            value if value == u8::from(Self::Negate) => Ok(Self::Negate),
            value if value == u8::from(Self::Return) => Ok(Self::Return),
            _ => Err(byte),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConstantIndexTooLarge {
    pub(crate) index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstantEncoding {
    Short(u8),
    Long([u8; 3]),
}

/// Chooses the most compact operand encoding that can represent `index`.
fn encode_constant_index(index: usize) -> Result<ConstantEncoding, ConstantIndexTooLarge> {
    if index <= SHORT_CONSTANT_MAX_INDEX {
        return Ok(ConstantEncoding::Short(index as u8));
    }

    if index <= LONG_CONSTANT_MAX_INDEX {
        return Ok(ConstantEncoding::Long([
            (index & 0xFF) as u8,
            ((index >> 8) & 0xFF) as u8,
            ((index >> 16) & 0xFF) as u8,
        ]));
    }

    Err(ConstantIndexTooLarge { index })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineRun {
    line: usize,
    count: usize,
}

#[derive(Debug, Default)]
pub(crate) struct Chunk {
    code: Vec<u8>,
    // Each run stores a source line plus how many consecutive bytes came from it.
    line_runs: Vec<LineRun>,
    constants: Vec<Value>,
}

impl Chunk {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Appends one byte of code and records its source line using run-length encoding.
    pub(crate) fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        match self.line_runs.last_mut() {
            Some(run) if run.line == line => run.count += 1,
            _ => self.line_runs.push(LineRun { line, count: 1 }),
        }
    }

    /// Appends a single opcode byte tagged with its source line.
    pub(crate) fn write_opcode(&mut self, opcode: OpCode, line: usize) {
        self.write_byte(opcode.into(), line)
    }

    /// Adds a value to the constant table and returns its index.
    pub(crate) fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// Adds a constant and emits the matching load instruction for its index width.
    pub(crate) fn write_constant(
        &mut self,
        value: Value,
        line: usize,
    ) -> Result<(), ConstantIndexTooLarge> {
        let index = self.constants.len();
        let encoding = encode_constant_index(index)?;

        // Delay mutating the constant table until we know the index can be encoded.
        self.constants.push(value);
        match encoding {
            ConstantEncoding::Short(index) => {
                self.write_opcode(OpCode::Constant, line);
                self.write_byte(index, line);
            }
            ConstantEncoding::Long(bytes) => {
                self.write_opcode(OpCode::ConstantLong, line);
                for byte in bytes {
                    self.write_byte(byte, line);
                }
            }
        }

        Ok(())
    }

    pub(crate) fn code(&self) -> &[u8] {
        &self.code
    }

    /// Returns the source line recorded for the byte at `offset`.
    pub(crate) fn line_at(&self, offset: usize) -> Option<usize> {
        let mut seen = 0;

        for run in &self.line_runs {
            if offset < seen + run.count {
                return Some(run.line);
            }
            seen += run.count;
        }

        None
    }

    pub(crate) fn constants(&self) -> &[Value] {
        &self.constants
    }
}

#[cfg(test)]
mod tests {
    use super::{Chunk, ConstantEncoding, ConstantIndexTooLarge, OpCode, encode_constant_index};
    use crate::value::Value;

    fn number(value: f64) -> Value {
        Value::number(value)
    }

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
    fn line_at_walks_across_compressed_line_runs() {
        let mut chunk = Chunk::new();

        chunk.write_byte(1, 10);
        chunk.write_byte(2, 10);
        chunk.write_byte(3, 20);
        chunk.write_byte(4, 20);
        chunk.write_byte(5, 30);

        assert_eq!(chunk.line_at(0), Some(10));
        assert_eq!(chunk.line_at(1), Some(10));
        assert_eq!(chunk.line_at(2), Some(20));
        assert_eq!(chunk.line_at(3), Some(20));
        assert_eq!(chunk.line_at(4), Some(30));
        assert_eq!(chunk.line_at(5), None);
    }

    #[test]
    fn add_constant_returns_the_inserted_index_and_stores_the_value() {
        let mut chunk = Chunk::new();

        let first = chunk.add_constant(number(1.2));
        let second = chunk.add_constant(number(3.4));

        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(chunk.constants(), &[number(1.2), number(3.4)]);
    }

    #[test]
    fn write_constant_uses_short_instruction_with_one_byte_index() {
        let mut chunk = Chunk::new();

        chunk.write_constant(number(1.2), 7).unwrap();

        assert_eq!(chunk.code(), &[u8::from(OpCode::Constant), 0]);
        assert_eq!(chunk.constants(), &[number(1.2)]);
        assert_eq!(chunk.line_at(0), Some(7));
        assert_eq!(chunk.line_at(1), Some(7));
    }

    #[test]
    fn write_constant_uses_long_instruction_after_short_index_range() {
        let mut chunk = Chunk::new();
        for index in 0..=u8::MAX {
            chunk.add_constant(number(index as f64));
        }

        chunk.write_constant(number(256.0), 9).unwrap();

        assert_eq!(chunk.code(), &[u8::from(OpCode::ConstantLong), 0, 1, 0]);
        assert_eq!(chunk.constants()[256], number(256.0));
        assert_eq!(chunk.line_at(0), Some(9));
        assert_eq!(chunk.line_at(1), Some(9));
        assert_eq!(chunk.line_at(2), Some(9));
        assert_eq!(chunk.line_at(3), Some(9));
    }

    #[test]
    fn encode_constant_index_rejects_indexes_that_do_not_fit_in_24_bits() {
        assert_eq!(
            encode_constant_index(0x1_00_00_00),
            Err(ConstantIndexTooLarge {
                index: 0x1_00_00_00
            })
        );
    }

    #[test]
    fn encode_constant_index_switches_between_short_and_long_forms() {
        assert_eq!(encode_constant_index(0), Ok(ConstantEncoding::Short(0)));
        assert_eq!(encode_constant_index(255), Ok(ConstantEncoding::Short(255)));
        assert_eq!(
            encode_constant_index(256),
            Ok(ConstantEncoding::Long([0, 1, 0]))
        );
        assert_eq!(
            encode_constant_index(0xFF_FFFF),
            Ok(ConstantEncoding::Long([0xFF, 0xFF, 0xFF]))
        );
    }

    #[test]
    fn opcode_round_trips_through_its_byte_encoding() {
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Constant)),
            Ok(OpCode::Constant)
        );
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::ConstantLong)),
            Ok(OpCode::ConstantLong)
        );
        assert_eq!(OpCode::try_from(u8::from(OpCode::Nil)), Ok(OpCode::Nil));
        assert_eq!(OpCode::try_from(u8::from(OpCode::True)), Ok(OpCode::True));
        assert_eq!(OpCode::try_from(u8::from(OpCode::False)), Ok(OpCode::False));
        assert_eq!(OpCode::try_from(u8::from(OpCode::Add)), Ok(OpCode::Add));
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Subtract)),
            Ok(OpCode::Subtract)
        );
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Multiply)),
            Ok(OpCode::Multiply)
        );
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Divide)),
            Ok(OpCode::Divide)
        );
        assert_eq!(OpCode::try_from(u8::from(OpCode::Not)), Ok(OpCode::Not));
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Negate)),
            Ok(OpCode::Negate)
        );
        assert_eq!(
            OpCode::try_from(u8::from(OpCode::Return)),
            Ok(OpCode::Return)
        );
    }
}
