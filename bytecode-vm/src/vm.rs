use crate::{
    chunk::{Chunk, OpCode},
    debug::disassemble_instruction,
    value::{Value, print_value},
};

const DEBUG_TRACE_EXECUTION: bool = false;

#[derive(Debug, Default)]
pub(crate) struct Vm {
    ip: usize,
    stack: Vec<Value>,
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
        // Reused VMs always start each interpretation with a clean stack and
        // the instruction pointer reset to the first byte.
        self.ip = 0;
        self.stack.clear();
        self.run(chunk)
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("vm stack underflow")
    }

    fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        loop {
            if DEBUG_TRACE_EXECUTION {
                self.trace_stack();
                disassemble_instruction(chunk, self.ip);
            }

            let Some(instruction) = self.read_byte(chunk) else {
                return InterpretResult::InterpretRuntimeError;
            };
            match OpCode::try_from(instruction) {
                Ok(OpCode::Constant) => {
                    let Some(constant) = self.read_constant(chunk) else {
                        return InterpretResult::InterpretRuntimeError;
                    };
                    self.push(constant);
                }
                Ok(OpCode::ConstantLong) => {
                    let Some(constant) = self.read_constant_long(chunk) else {
                        return InterpretResult::InterpretRuntimeError;
                    };
                    self.push(constant);
                }
                Ok(OpCode::Negate) => {
                    let value = self.pop();
                    self.push(-value);
                }
                Ok(OpCode::Return) => {
                    let value = self.pop();
                    print_value(value);
                    println!();
                    return InterpretResult::InterpretOk;
                }
                Err(_) => {
                    return InterpretResult::InterpretRuntimeError;
                }
            }
        }
    }

    fn trace_stack(&self) {
        print!("          ");
        for &value in &self.stack {
            print!("[ ");
            print_value(value);
            print!(" ]");
        }
        println!();
    }

    /// Reads the next bytecode operand and advances the instruction pointer.
    fn read_byte(&mut self, chunk: &Chunk) -> Option<u8> {
        let byte = chunk.code().get(self.ip).copied()?;
        self.ip += 1;
        Some(byte)
    }

    /// Reads the next byte as a constant-table index, then loads that value.
    fn read_constant(&mut self, chunk: &Chunk) -> Option<Value> {
        let index = self.read_byte(chunk)? as usize;
        chunk.constants().get(index).copied()
    }

    /// Reads a little-endian 24-bit constant-table index, then loads that value.
    fn read_constant_long(&mut self, chunk: &Chunk) -> Option<Value> {
        let low = self.read_byte(chunk)? as usize;
        let mid = self.read_byte(chunk)? as usize;
        let high = self.read_byte(chunk)? as usize;
        let index = low | (mid << 8) | (high << 16);

        chunk.constants().get(index).copied()
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
        chunk.write_constant(1.2, 1).unwrap();
        chunk.write_opcode(OpCode::Return, 1);

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretOk);
        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretOk);
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn invalid_opcode_returns_runtime_error() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_byte(255, 1);

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretRuntimeError);
    }

    #[test]
    fn constant_opcode_pushes_a_value_that_return_can_pop() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1).unwrap();
        chunk.write_opcode(OpCode::Return, 1);

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretOk);
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn constant_long_opcode_pushes_a_value_that_return_can_pop() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        for index in 0..=u8::MAX {
            chunk.add_constant(index as f64);
        }
        chunk.write_constant(256.0, 1).unwrap();
        chunk.write_opcode(OpCode::Return, 1);

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretOk);
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn negate_opcode_negates_the_top_stack_value_before_return() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1).unwrap();
        chunk.write_opcode(OpCode::Negate, 1);
        chunk.write_opcode(OpCode::Return, 1);

        assert_eq!(vm.interpret(&chunk), InterpretResult::InterpretOk);
        assert!(vm.stack.is_empty());
    }

    #[test]
    #[should_panic(expected = "vm stack underflow")]
    fn return_without_a_value_panics_on_stack_underflow() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Return, 1);

        let _ = vm.interpret(&chunk);
    }
}
