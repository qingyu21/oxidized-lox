# tree-walk

The `tree-walk` crate contains the current working Rust implementation of Lox's
tree-walk interpreter from Chapters 4 through 13 of *Crafting Interpreters*.

It includes classes, inheritance, `this`, and `super`, while a few optional
chapter challenge features remain intentionally deferred.

## Current Milestone

- implemented: scanning, parsing, AST construction, lexical resolution,
  functions, closures, classes, inheritance, `this`, and `super`
- intentionally deferred: selected chapter challenge features that would grow
  the language beyond the main chapter path
- not started here: the bytecode VM from the second half of the book, which
  now lives in the sibling `bytecode-vm/` workspace member

## Running

From the workspace root, run a script with:

```bash
cargo run -p tree-walk -- examples/print_demo.lox
```

Start the REPL with:

```bash
cargo run -p tree-walk --
```

More example scripts live in the shared workspace `examples/` directory,
including:

- `fibonacci_for_demo.lox` and `fibonacci_recursive_demo.lox` for iterative and recursive control flow
- `cons_list_demo.lox`, `merge_sort_list_demo.lox`, and `bst_demo.lox` for recursive data structures and algorithms
- `expr_tree_demo.lox` for symbolic expression trees, evaluation, and simplification
- `mandelbrot_ascii_demo.lox`, `rule30_demo.lox`, `sierpinski_carpet_demo.lox`, and `hilbert_curve_demo.lox` for ASCII pattern generation

## Development

From the workspace root:

```bash
cargo test -p tree-walk
cargo clippy -p tree-walk --all-targets --all-features -- -D warnings
cargo fmt --all
```

## Source Map

- `src/lib.rs`: library crate root and compatibility re-exports for frontend modules
- `src/main.rs`: CLI entry point for script mode and the REPL
- `src/frontend.rs`: small aggregation module for frontend-only pieces
- `src/frontend/scanner.rs`: turns source text into `Vec<Token>`
- `src/frontend/parser.rs`: parser entry points, declarations, token helpers, and error recovery
- `src/frontend/parser/statements.rs`: statement parsing, including `if`, `while`, `for`, `break`, and `return`
- `src/frontend/parser/expressions.rs`: expression parsing and precedence handling, including call and property access syntax
- `src/frontend/expr.rs`: expression AST definitions
- `src/frontend/stmt.rs`: statement AST definitions, including function declarations and `return`
- `src/frontend/token.rs`: token and literal data types
- `src/resolver.rs`: resolver entry point and shared resolver state
- `src/resolver/expr.rs`: expression-side static scope resolution and lexical binding checks
- `src/resolver/stmt.rs`: statement-side static scope resolution, including class and function handling
- `src/resolver/scope.rs`: resolver scope-stack helpers, binding bookkeeping, and shared diagnostics
- `src/runtime.rs`: small re-export hub for runtime-facing types
- `src/runtime/value.rs`: runtime `Value` representation and conversions from literals
- `src/runtime/object.rs`: runtime callable/class/instance objects and method lookup
- `src/runtime/error.rs`: runtime error payloads
- `src/interpreter.rs`: interpreter entry points, environment handles, and resolver binding cache
- `src/interpreter/execute.rs`: statement execution and control-flow propagation
- `src/interpreter/evaluate.rs`: expression evaluation and runtime operator semantics
- `src/interpreter/callable.rs`: native/user-defined callable runtime objects
- `src/environment.rs`: lexical scope chain and variable storage
- `src/lox.rs`: top-level run modes, REPL flow, and error reporting
- `src/diagnostics.rs`: shared syntax/runtime diagnostic flags and reporting helpers
- `src/test_support.rs`: shared parser/resolver helpers for unit tests

## Limitations

- The crate is still in the tree-walk stage and does not include the bytecode
  VM from later parts of the book.
- The REPL evaluates one input line at a time and does not yet buffer
  incomplete multi-line statements.
- Optional chapter challenge features such as static methods, getters, and the
  Chapter 13 extension challenges are still TODO.
- Runtime object cycles are not reclaimed yet because this interpreter does
  not implement a tracing garbage collector.

## More Detail

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the detailed pipeline, type graph,
and runtime boundary notes for this crate.
