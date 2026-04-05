# oxidized-lox

A Rust implementation of the Lox language from *Crafting Interpreters*.

This repository currently contains a tree-walk interpreter for a growing subset
of Lox.

## Overview

The codebase follows a simple frontend/runtime pipeline:

```mermaid
flowchart LR
    Source["source code (&str)"]
    Scanner["Scanner"]
    Tokens["Vec<Token>"]
    Parser["Parser"]
    Ast["Vec<Stmt> / Expr"]
    Resolver["Resolver"]
    Interpreter["Interpreter"]
    Env["Environment chain"]
    Result["Value / RuntimeError"]

    Source --> Scanner --> Tokens --> Parser --> Ast --> Resolver --> Interpreter
    Interpreter --> Env
    Interpreter --> Result
```

More detail is documented in [ARCHITECTURE.md](./ARCHITECTURE.md).

## Current Status

Implemented today:

- scanning for punctuation, operators, identifiers, keywords, strings, numbers,
  line comments, and block comments
- recursive-descent parsing for expressions and statements
- call expressions with runtime dispatch through a callable abstraction
- user-defined functions, local functions, closures, and `return`
- variables, assignment, block scope, `if`, `while`, `for`, `break`,
  logical `and` / `or`,
  and `?:`
- a resolver pass for lexical scope binding and static name checks such as
  local self-initializer errors, duplicate local declarations, and unused
  local variables
- class declarations, instance methods, callable class objects, and first-draft
  instances with open fields plus property get/set
- a tree-walk interpreter with a small REPL
- one native callable, `clock()`

Later book stages still missing:

- bound methods, `this`, and `super`
- inheritance
- bytecode VM stages from later in the book

## Running

Build the project:

```bash
cargo build
```

Run a script:

```bash
cargo run -- examples/print_demo.lox
```

Try the block-scope example:

```bash
cargo run -- examples/block_scope_demo.lox
```

Start the REPL:

```bash
cargo run
```

REPL notes:

- bare expressions are evaluated and printed automatically
- multi-line incomplete input is not buffered yet

## Development

Run the test suite:

```bash
cargo test
```

Run lints:

```bash
cargo clippy
```

Format the codebase:

```bash
cargo fmt
```

## Source Map

- `src/scanner.rs`: turns source text into `Vec<Token>`
- `src/parser.rs`: parser entry points, declarations, token helpers, and error recovery
- `src/parser/statements.rs`: statement parsing, including `if`, `while`, `for`, `break`, and `return`
- `src/parser/expressions.rs`: expression parsing and precedence handling, including call and property access syntax
- `src/expr.rs`: expression AST definitions
- `src/stmt.rs`: statement AST definitions, including function declarations and `return`
- `src/resolver.rs`: static scope resolution and lexical binding analysis
- `src/runtime.rs`: shared runtime types such as `Value`, `RuntimeError`, and the callable trait
- `src/interpreter.rs`: interpreter entry points, environment handles, and resolver binding cache
- `src/interpreter/execute.rs`: statement execution and control-flow propagation
- `src/interpreter/evaluate.rs`: expression evaluation and runtime operator semantics
- `src/interpreter/callable.rs`: native/user-defined callable runtime objects
- `src/environment.rs`: lexical scope chain and variable storage
- `src/lox.rs`: top-level run modes, REPL flow, and error reporting
- `src/token.rs`: token and literal data types

## Key Distinctions

- `Literal` is syntax-level data carried through tokens and literal AST nodes.
- `Value` is the runtime value type produced by the interpreter.
- `Expr` nodes are evaluated for values.
- `Stmt` nodes are executed for side effects and control flow.

## Current Limitations

- The interpreter is still in the tree-walk stage and does not include the
  bytecode VM from later parts of the book.
- The REPL evaluates one input line at a time and does not yet buffer
  incomplete multi-line statements.
- The language implementation is still a subset of full Lox and does not yet
  support method binding, `this`, inheritance, or the later VM stages.

## Roadmap

Near-term goals:

- continue into the classes and methods chapters
- continue expanding parser and interpreter test coverage
- keep the code structure aligned with the book while documenting Rust-specific
  implementation choices

Longer-term goals:

- keep extending class support through methods, `this`, and inheritance
- explore the later bytecode VM stages

## References

- Bob Nystrom, *Crafting Interpreters*
- This project currently follows the tree-walk interpreter path and adapts the
  implementation to Rust
- More internal notes and type/data-flow diagrams live in
  [ARCHITECTURE.md](./ARCHITECTURE.md)
