use crate::{
    chunk::{Chunk, OpCode},
    debug::disassemble_instruction,
};

const DEBUG_TRACE_EXECUTION: bool = false;

#[derive(Debug, Default)]
pub(crate) struct Vm {
    ip: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

impl Vm {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        // Reused VMs always start each interpretation at the first byte.
        self.ip = 0;
        self.run(chunk)
    }

    fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        loop {
            if DEBUG_TRACE_EXECUTION {
                let _ = disassemble_instruction(chunk, self.ip);
            }

            let Some(instruction) = self.read_byte(chunk) else {
                return InterpretResult::InterpretRuntimeError;
            };
            match OpCode::try_from(instruction) {
                // The value stack has not been added yet, so constant-loading
                // instructions are still reported as runtime errors for now.
                Ok(OpCode::Constant | OpCode::ConstantLong) => {
                    return InterpretResult::InterpretRuntimeError;
                }
                Ok(OpCode::Return) => {
                    return InterpretResult::InterpretOk;
                }
                Err(_) => {
                    return InterpretResult::InterpretRuntimeError;
                }
            }
        }
    }

    /// Reads the next bytecode operand and advances the instruction pointer.
    fn read_byte(&mut self, chunk: &Chunk) -> Option<u8> {
        let byte = chunk.code().get(self.ip).copied()?;
        self.ip += 1;
        Some(byte)
    }
}

#[cfg(test)]
mod tests {
    use super::{InterpretResult, Vm};
    use crate::chunk::{Chunk, OpCode};

    #[test]
    fn interpret_can_reuse_the_same_vm() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Return, 1);

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretOk);
        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretOk);
    }

    #[test]
    fn invalid_opcode_returns_runtime_error() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_byte(255, 1);

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretRuntimeError);
    }

    #[test]
    fn unimplemented_constant_opcode_returns_runtime_error() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1).unwrap();

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretRuntimeError);
    }

    #[test]
    fn unimplemented_constant_long_opcode_returns_runtime_error() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        for index in 0..=u8::MAX {
            chunk.add_constant(index as f64);
        }
        chunk.write_constant(256.0, 1).unwrap();

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretRuntimeError);
    }
}
