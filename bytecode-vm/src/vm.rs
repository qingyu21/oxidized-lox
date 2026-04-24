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
    Ok,
    CompileError,
    RuntimeError,
}

impl Vm {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Resets transient VM state so a single instance can execute multiple chunks.
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

    /// Returns a value from the stack without popping it.
    fn peek(&self, distance: usize) -> Option<Value> {
        self.stack
            .len()
            .checked_sub(1 + distance)
            .and_then(|index| self.stack.get(index).copied())
    }

    /// Clears transient stack state after a runtime error.
    fn reset_stack(&mut self) {
        self.stack.clear();
    }

    /// Reports a runtime error and points at the source line of the failed instruction.
    fn runtime_error(&mut self, chunk: &Chunk, message: &str) {
        eprintln!("{message}");

        let instruction = self.ip.saturating_sub(1);
        if let Some(line) = chunk.line_at(instruction) {
            eprintln!("[line {line}] in script");
        }

        self.reset_stack();
    }

    /// Negates the current top-of-stack value in place without changing stack height.
    fn negate_top(&mut self) -> bool {
        let value = self.stack.last_mut().expect("vm stack underflow");
        let Value::Number(number) = value else {
            return false;
        };
        *number = -*number;
        true
    }

    /// Validates both operands before popping them, then pushes the wrapped result.
    fn binary_op<T>(
        &mut self,
        wrap: impl FnOnce(T) -> Value,
        op: impl FnOnce(f64, f64) -> T,
    ) -> bool {
        let (Some(a), Some(b)) = (
            self.peek(1).and_then(Value::as_number),
            self.peek(0).and_then(Value::as_number),
        ) else {
            return false;
        };

        let _ = self.pop();
        let _ = self.pop();
        self.push(wrap(op(a, b)));
        true
    }

    /// Dispatches bytecode instructions until execution finishes or errors out.
    fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        loop {
            if DEBUG_TRACE_EXECUTION {
                self.trace_stack();
                disassemble_instruction(chunk, self.ip);
            }

            let Some(instruction) = self.read_byte(chunk) else {
                return InterpretResult::RuntimeError;
            };
            match OpCode::try_from(instruction) {
                Ok(OpCode::Constant) => {
                    let Some(constant) = self.read_constant(chunk) else {
                        return InterpretResult::RuntimeError;
                    };
                    self.push(constant);
                }
                Ok(OpCode::ConstantLong) => {
                    let Some(constant) = self.read_constant_long(chunk) else {
                        return InterpretResult::RuntimeError;
                    };
                    self.push(constant);
                }
                Ok(OpCode::Nil) => self.push(Value::Nil),
                Ok(OpCode::True) => self.push(Value::Bool(true)),
                Ok(OpCode::False) => self.push(Value::Bool(false)),
                Ok(OpCode::Add) => {
                    if !self.binary_op(Value::number, |a, b| a + b) {
                        self.runtime_error(chunk, "Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                }
                Ok(OpCode::Subtract) => {
                    if !self.binary_op(Value::number, |a, b| a - b) {
                        self.runtime_error(chunk, "Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                }
                Ok(OpCode::Multiply) => {
                    if !self.binary_op(Value::number, |a, b| a * b) {
                        self.runtime_error(chunk, "Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                }
                Ok(OpCode::Divide) => {
                    if !self.binary_op(Value::number, |a, b| a / b) {
                        self.runtime_error(chunk, "Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                }
                Ok(OpCode::Not) => {
                    let value = self.pop();
                    self.push(Value::Bool(value.is_falsey()));
                }
                Ok(OpCode::Negate) => {
                    if !matches!(self.peek(0), Some(Value::Number(_))) {
                        self.runtime_error(chunk, "Operand must be a number.");
                        return InterpretResult::RuntimeError;
                    }
                    if !self.negate_top() {
                        return InterpretResult::RuntimeError;
                    }
                }
                Ok(OpCode::Return) => {
                    let value = self.pop();
                    print_value(value);
                    println!();
                    return InterpretResult::Ok;
                }
                Err(_) => {
                    return InterpretResult::RuntimeError;
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
    use crate::value::Value;

    fn number(value: f64) -> Value {
        Value::number(value)
    }

    fn assert_interpret_ok_and_empties_stack(vm: &mut Vm, chunk: &Chunk) {
        assert_eq!(vm.interpret(chunk), InterpretResult::Ok);
        assert!(vm.stack.is_empty());
    }

    fn returning_constant_chunk(value: f64) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.write_constant(number(value), 1).unwrap();
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

        assert_eq!(vm.interpret(&chunk), InterpretResult::RuntimeError);
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
            chunk.add_constant(number(index as f64));
        }
        chunk.write_constant(number(256.0), 1).unwrap();
        chunk.write_opcode(OpCode::Return, 1);

        assert_interpret_ok_and_empties_stack(&mut vm, &chunk);
    }

    #[test]
    fn literal_opcodes_leave_expected_value_on_stack() {
        let cases = [
            (OpCode::Nil, Value::Nil),
            (OpCode::True, Value::Bool(true)),
            (OpCode::False, Value::Bool(false)),
        ];

        for (opcode, expected) in cases {
            let mut vm = Vm::new();
            let mut chunk = Chunk::new();
            chunk.write_opcode(opcode, 1);

            vm.ip = 0;
            vm.stack.clear();

            assert_eq!(vm.run(&chunk), InterpretResult::RuntimeError);
            assert_eq!(vm.stack, vec![expected]);
        }
    }

    #[test]
    fn negate_opcode_negates_the_top_stack_value_before_return() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_constant(number(1.2), 1).unwrap();
        chunk.write_opcode(OpCode::Negate, 1);
        chunk.write_opcode(OpCode::Return, 1);

        assert_interpret_ok_and_empties_stack(&mut vm, &chunk);
    }

    #[test]
    fn not_opcode_pushes_whether_value_is_falsey() {
        let cases = [
            (Value::Bool(false), Value::Bool(true)),
            (Value::Nil, Value::Bool(true)),
            (Value::Bool(true), Value::Bool(false)),
            (number(0.0), Value::Bool(false)),
        ];

        for (input, expected) in cases {
            let mut vm = Vm::new();
            let mut chunk = Chunk::new();
            chunk.add_constant(input);
            chunk.write_opcode(OpCode::Constant, 1);
            chunk.write_byte(0, 1);
            chunk.write_opcode(OpCode::Not, 1);

            vm.ip = 0;
            vm.stack.clear();

            assert_eq!(vm.run(&chunk), InterpretResult::RuntimeError);
            assert_eq!(vm.stack, vec![expected]);
        }
    }

    #[test]
    fn binary_op_uses_left_then_right_operand_order() {
        let mut vm = Vm::new();
        vm.push(number(3.0));
        vm.push(number(1.0));

        assert!(vm.binary_op(Value::number, |a, b| a - b));

        assert_eq!(vm.pop(), number(2.0));
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn binary_op_rejects_non_numbers_without_popping_operands() {
        let mut vm = Vm::new();
        vm.push(number(1.0));
        vm.push(Value::Bool(false));

        assert!(!vm.binary_op(Value::number, |a, b| a + b));

        assert_eq!(vm.stack, vec![number(1.0), Value::Bool(false)]);
    }

    #[test]
    fn negate_top_keeps_stack_height_the_same() {
        let mut vm = Vm::new();
        vm.push(number(1.2));

        assert!(vm.negate_top());

        assert_eq!(vm.stack, vec![number(-1.2)]);
    }

    #[test]
    fn peek_reads_values_without_popping_them() {
        let mut vm = Vm::new();
        vm.push(number(1.0));
        vm.push(number(2.0));

        assert_eq!(vm.peek(0), Some(number(2.0)));
        assert_eq!(vm.peek(1), Some(number(1.0)));
        assert_eq!(vm.stack, vec![number(1.0), number(2.0)]);
    }

    #[test]
    fn negate_reports_runtime_error_for_non_number_operand() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_constant(Value::Bool(false), 123).unwrap();
        chunk.write_opcode(OpCode::Negate, 123);

        assert_eq!(vm.interpret(&chunk), InterpretResult::RuntimeError);
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn binary_arithmetic_reports_runtime_error_for_non_number_operands() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();
        chunk.write_constant(number(1.0), 123).unwrap();
        chunk.write_constant(Value::Bool(false), 123).unwrap();
        chunk.write_opcode(OpCode::Add, 123);

        assert_eq!(vm.interpret(&chunk), InterpretResult::RuntimeError);
        assert!(vm.stack.is_empty());
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
            chunk.write_constant(number(left), 1).unwrap();
            chunk.write_constant(number(right), 1).unwrap();
            chunk.write_opcode(opcode, 1);

            // Leave off OP_RETURN on purpose so we can inspect the intermediate
            // stack result after run() stops at the end of the chunk.
            vm.ip = 0;
            vm.stack.clear();

            assert_eq!(vm.run(&chunk), InterpretResult::RuntimeError);
            assert_eq!(vm.stack, vec![number(expected)]);
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
