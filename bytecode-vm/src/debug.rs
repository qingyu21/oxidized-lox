use crate::chunk::{Chunk, OpCode};

pub(crate) fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {name} ==");

    let mut offset = 0_usize;
    while offset < chunk.code().len() {
        offset = disassemble_instruction(chunk, offset);
    }
}

pub(crate) fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    let instruction = chunk.code()[offset];

    print_line_prefix(chunk, offset);

    match OpCode::try_from(instruction) {
        Ok(opcode @ OpCode::Constant) => constant_instruction(opcode, chunk, offset),
        Ok(opcode @ OpCode::Return) => simple_instruction(opcode, offset),
        Err(unknown) => {
            println!("Unknown opcode {unknown}");
            offset + 1
        }
    }
}

/// Prints the disassembly prefix: byte offset plus source line marker.
fn print_line_prefix(chunk: &Chunk, offset: usize) {
    let line = chunk.line_at(offset);

    print!("{offset:04} ");
    match line {
        Some(current_line) if offset > 0 && chunk.line_at(offset - 1) == Some(current_line) => {
            print!("   | ")
        }
        Some(current_line) => print!("{current_line:4} "),
        None => print!("   ? "),
    }
}

/// Disassembles an instruction encoded as opcode + single-byte constant index.
fn constant_instruction(opcode: OpCode, chunk: &Chunk, offset: usize) -> usize {
    let Some(&constant_index) = chunk.code().get(offset + 1) else {
        println!("{:<16} <missing constant index>", opcode.mnemonic());
        return offset + 1;
    };

    match chunk.constants().get(constant_index as usize) {
        Some(value) => println!("{:<16} {constant_index:>4} '{value}'", opcode.mnemonic()),
        None => println!(
            "{:<16} {constant_index:>4} <invalid constant index>",
            opcode.mnemonic()
        ),
    }

    offset + 2
}

fn simple_instruction(opcode: OpCode, offset: usize) -> usize {
    println!("{}", opcode.mnemonic());
    offset + 1
}

#[cfg(test)]
mod tests {
    use super::{OpCode, disassemble_instruction};
    use crate::chunk::Chunk;

    #[test]
    fn return_instruction_advances_by_one_byte() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Return, 1);

        assert_eq!(disassemble_instruction(&chunk, 0), 1);
    }

    #[test]
    fn unknown_instruction_still_advances_by_one_byte() {
        let mut chunk = Chunk::new();
        chunk.write_byte(255, 1);

        assert_eq!(disassemble_instruction(&chunk, 0), 1);
    }

    #[test]
    fn constant_instruction_advances_by_opcode_and_operand() {
        let mut chunk = Chunk::new();
        let index = chunk.add_constant(1.2);

        chunk.write_opcode(OpCode::Constant, 7);
        chunk.write_byte(index as u8, 7);

        assert_eq!(disassemble_instruction(&chunk, 0), 2);
    }

    #[test]
    fn malformed_constant_without_operand_advances_safely() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Constant, 9);

        assert_eq!(disassemble_instruction(&chunk, 0), 1);
    }

    #[test]
    fn constant_with_invalid_index_does_not_panic() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Constant, 11);
        chunk.write_byte(7, 11);

        assert_eq!(disassemble_instruction(&chunk, 0), 2);
    }
}
