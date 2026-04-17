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

    /// Negates the current top-of-stack value in place without changing stack height.
    fn negate_top(&mut self) {
        let value = self.stack.last_mut().expect("vm stack underflow");
        *value = -*value;
    }

    /// Pops the right operand first, then the left, matching stack-based evaluation order.
    fn binary_op(&mut self, op: impl FnOnce(Value, Value) -> Value) {
        let b = self.pop();
        let a = self.pop();
        self.push(op(a, b));
    }

    /// Dispatches bytecode instructions until execution finishes or errors out.
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
                Ok(OpCode::Add) => self.binary_op(|a, b| a + b),
                Ok(OpCode::Subtract) => self.binary_op(|a, b| a - b),
                Ok(OpCode::Multiply) => self.binary_op(|a, b| a * b),
                Ok(OpCode::Divide) => self.binary_op(|a, b| a / b),
                Ok(OpCode::Negate) => self.negate_top(),
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

    /// Prints the stack from bottom to top before the next instruction executes.
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

    fn assert_interpret_ok_and_empties_stack(vm: &mut Vm, chunk: &Chunk) {
        assert_eq!(vm.interpret(chunk), InterpretResult::InterpretOk);
        assert!(vm.stack.is_empty());
    }

    fn returning_constant_chunk(value: f64) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.write_constant(value, 1).unwrap();
        chunk.write_opcode(OpCode::Return, 1);
        chunk
    }

    #[test]
    fn interpret_can_reuse_the_same_vm() {
        let mut vm = Vm::new();
        let chunk = returning_constant_chunk(1.2);

        assert_interpret_ok_and_empties_stack(&mut vm, &chunk);
        assert_interpret_ok_and_empties_stack(&mut vm, &chunk);
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
        let chunk = returning_constant_chunk(1.2);

        assert_interpret_ok_and_empties_stack(&mut vm, &chunk);
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

        assert_interpret_ok_and_empties_stack(&mut vm, &chunk);
    }

    #[test]
    fn negate_opcode_negates_the_top_stack_value_before_return() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1).unwrap();
        chunk.write_opcode(OpCode::Negate, 1);
        chunk.write_opcode(OpCode::Return, 1);

        assert_interpret_ok_and_empties_stack(&mut vm, &chunk);
    }

    #[test]
    fn binary_op_uses_left_then_right_operand_order() {
        let mut vm = Vm::new();
        vm.push(3.0);
        vm.push(1.0);

        vm.binary_op(|a, b| a - b);

        assert_eq!(vm.pop(), 2.0);
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn negate_top_keeps_stack_height_the_same() {
        let mut vm = Vm::new();
        vm.push(1.2);

        vm.negate_top();

        assert_eq!(vm.stack, vec![-1.2]);
    }

    #[test]
    fn binary_arithmetic_opcodes_leave_expected_result_on_stack() {
        let cases = [
            (OpCode::Add, 1.0, 2.0, 3.0),
            (OpCode::Subtract, 3.0, 1.0, 2.0),
            (OpCode::Multiply, 3.0, 2.0, 6.0),
            (OpCode::Divide, 8.0, 2.0, 4.0),
        ];

        for (opcode, left, right, expected) in cases {
            let mut vm = Vm::new();
            let mut chunk = Chunk::new();
            chunk.write_constant(left, 1).unwrap();
            chunk.write_constant(right, 1).unwrap();
            chunk.write_opcode(opcode, 1);

            // Leave off OP_RETURN on purpose so we can inspect the intermediate
            // stack result after run() stops at the end of the chunk.
            vm.ip = 0;
            vm.stack.clear();

            assert_eq!(vm.run(&chunk), InterpretResult::InterpretRuntimeError);
            assert_eq!(vm.stack, vec![expected]);
        }
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
