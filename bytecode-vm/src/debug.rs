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

    print!("{offset:04} ");
    match instruction {
        value if value == OpCode::Return as u8 => simple_instruction("OP_RETURN", offset),
        _ => {
            println!("Unknown opcode {instruction}");
            offset + 1
        }
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{name}");
    offset + 1
}

#[cfg(test)]
mod tests {
    use super::{OpCode, disassemble_instruction};
    use crate::chunk::Chunk;

    #[test]
    fn return_instruction_advances_by_one_byte() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(OpCode::Return as u8);

        assert_eq!(disassemble_instruction(&chunk, 0), 1);
    }

    #[test]
    fn unknown_instruction_still_advances_by_one_byte() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(255);

        assert_eq!(disassemble_instruction(&chunk, 0), 1);
    }
}
