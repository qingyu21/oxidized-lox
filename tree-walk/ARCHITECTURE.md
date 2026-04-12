# Architecture

This file is a compact map of the `tree-walk` crate: the main data flow, the
core types, and the boundaries between frontend, semantic-analysis, and runtime
code.

## End-to-End Flow

```mermaid
flowchart LR
    Source["source code (&str)"]
    Scanner["Scanner"]
    Parser["Parser"]
    Program["ParsedProgram"]
    ReplExpr["ParsedExpression"]
    Resolver["Resolver"]
    Interpreter["Interpreter"]
    Env["EnvironmentRef -> Environment"]
    Value["Value"]
    RuntimeError["RuntimeError"]

    Source --> Scanner --> Parser
    Parser --> Program --> Resolver --> Interpreter
    Parser --> ReplExpr --> Resolver
    Interpreter --> Env
    Interpreter --> Value
    Interpreter --> RuntimeError
```

The same pipeline is reused in both modes:

- script mode: source text is parsed into `ParsedProgram`, resolved, and
  executed
- REPL bare-expression mode: source text is parsed into
  `ParsedExpression`, resolved, and evaluated directly

## Directory Layout vs. Theory

The project uses a slightly more practical directory split than a fully
textbook-style compiler layout.

- `src/frontend/` holds the scanner, parser, tokens, and AST types. This is
  the narrow frontend: it turns source text into syntax trees.
- `src/resolver/` is kept separate even though name binding is often taught as
  part of a broader frontend. In this codebase it already feels like a distinct
  semantic-analysis pass layered on top of the parsed AST.
- `src/interpreter/` and `src/runtime/` cover execution and runtime objects,
  which are clearly beyond the frontend.
- `src/lox.rs` coordinates the end-to-end pipeline for scripts and the REPL,
  while `src/lib.rs` and `src/main.rs` provide the crate and CLI entry
  points outside those stage-specific directories.

If you prefer a more theoretical mental model, you can read the current layout
like this:

- frontend: `src/frontend/`
- semantic analysis: `src/resolver/`
- execution/runtime: `src/interpreter/`, `src/runtime/`, `src/environment.rs`
- application orchestration: `src/lib.rs`, `src/main.rs`, `src/lox.rs`,
  `src/diagnostics.rs`

## Core Type Graph

```mermaid
flowchart TD
    TokenType["TokenType"]
    Literal["Literal"]
    Token["Token"]
    Scanner["Scanner"]
    ParseError["ParseError"]
    Parser["Parser"]
    ParsedProgram["ParsedProgram"]
    ParsedExpression["ParsedExpression"]
    ResolveError["ResolveError"]
    Resolver["Resolver"]
    Expr["Expr"]
    ExprArena["ExprArena"]
    ExprRef["ExprRef"]
    Stmt["Stmt"]
    Interpreter["Interpreter"]
    EnvironmentRef["EnvironmentRef = Rc<RefCell<Environment>>"]
    Environment["Environment"]
    Value["Value"]
    RuntimeError["RuntimeError"]

    TokenType --> Token
    Literal --> Token
    Scanner --> Token
    Scanner --> Parser
    Token --> Parser
    Parser --> ParseError
    Parser --> ParsedProgram
    Parser --> ParsedExpression
    Parser --> Expr
    Parser --> Stmt
    ExprArena --> ExprRef
    ExprRef --> Expr
    ParsedProgram --> Stmt
    ParsedExpression --> Expr
    Token --> Resolver
    Expr --> Resolver
    Stmt --> Resolver
    Resolver --> ResolveError
    Resolver --> Interpreter
    Literal --> Expr
    Token --> Expr
    Token --> Stmt
    Expr --> Stmt
    Stmt --> Interpreter
    Expr --> Interpreter
    Interpreter --> EnvironmentRef
    EnvironmentRef --> Environment
    Environment --> Value
    Interpreter --> Value
    Token --> RuntimeError
```

## Type Roles

### Frontend and Semantic Analysis

`TokenType`

- Enumerates lexical categories such as `Identifier`, `Number`, `If`, `And`,
  `LeftParen`, and `Eof`.
- The parser mainly makes decisions by looking at `TokenType`.

`Literal`

- Carries literal payloads recognized during scanning, such as string and
  number contents.
- Represents syntax-level literal data, not general runtime state.

`Token`

- Bundles `type_`, `lexeme`, optional `literal`, `line`, and a stable token id
  for the current thread.
- Acts as the common unit passed from scanner to parser.
- Is also kept inside AST nodes and runtime errors so later stages still know
  which source token they came from.
- The token id is assigned from a thread-local counter, which matches the
  crate's single-threaded `Rc` / `RefCell` design while still letting the
  resolver and interpreter associate lexical-binding results with variable-use
  sites without reshaping the AST.

`Scanner`

- Reads source text one character at a time.
- Produces `Token`s incrementally through `next_token()`, so the main
  parse/execute pipeline does not need to materialize a full `Vec<Token>`
  first.
- Stores each token lexeme as a shared source buffer plus byte span, rather
  than allocating a fresh standalone string per token.
- Owns the lexical rules of the language.

`ParseError`

- Lightweight marker type used inside the parser to unwind after a syntax
  failure.
- User-facing parse diagnostics are emitted through `diagnostics.rs`, while
  `lox.rs` decides when the pipeline should stop after those flags are set.

`ResolveError`

- Lightweight marker type used inside the resolver to stop after a static
  binding error.
- User-facing resolver diagnostics are also emitted through `diagnostics.rs`,
  with `lox.rs` coordinating whether execution continues.

`Parser`

- Owns a `Scanner` plus the current and previous `Token`, consuming the token
  stream incrementally while parsing.
- Produces either `ParsedProgram` or `ParsedExpression`, lightweight wrappers
  that keep the shared expression arena alive alongside the parsed syntax.
- Is split into a small root module plus `statements.rs` and
  `expressions.rs`, so statement parsing and expression precedence logic stay
  separated as the grammar grows.
- Encodes precedence and associativity through recursive-descent methods such
  as `assignment()`, `conditional()`, `logic_or()`, `call()`, and `term()`.
- Desugars `for` loops into existing `Stmt::Block` and `Stmt::While` nodes
  instead of introducing a separate runtime-only statement form.
- Tracks loop and function nesting so `break` and `return` can be validated
  against the current parsing context.
- Performs local error recovery with `synchronize()`.

`ParsedProgram` and `ParsedExpression`

- Own parsed syntax plus the shared `ExprArenaRef` that backs any nested
  expression references.
- Let later stages borrow parsed statements/expressions normally without
  copying the tree or manually threading arena lifetimes everywhere.

`Resolver`

- Walks the parsed AST before interpretation and performs static name binding.
- Is split into a small root module plus `expr.rs`, `stmt.rs`, and `scope.rs`,
  so expression resolution, statement resolution, and lexical-scope helpers
  stay separated as the binding logic grows.
- Semantically it sits just after parsing: it does not execute code, but it is
  no longer part of raw syntax construction either.
- Tracks local lexical scopes with a stack of
  `HashMap<Rc<str>, BindingInfo>`-backed `Scope` values, where each entry
  remembers the binding's token, kind, slot, definition state, and whether it
  was ever read.
- Detects semantic errors such as reading a local variable inside its own
  initializer, redeclaring a local name in the same scope, using `this`
  outside of a class, using `super` outside of a subclass, returning a value
  from an initializer, and leaving a local variable unused.
- Records lexical distances in the interpreter so runtime lookup can jump
  straight to the correct environment.

`Expr`

- Expression AST nodes.
- Represents syntax that evaluates to a value: literals, variables, unary and
  binary operators, assignment, logical operators, `?:`, call expressions,
  `this`, `super`, and instance property get/set expressions.
- Stores most child links as `ExprRef`, so nested expressions point into a
  shared arena by handle instead of recursively owning boxed child nodes.
- Call expressions already evaluate through the interpreter's runtime call
  dispatch, which handles callable values and class construction in one place.

`ExprArena` and `ExprRef`

- `ExprArena` stores expression nodes in a `Vec<Expr>` for the life of one
  parsed input and tags each arena with its own id.
- `ExprRef` is a lightweight `{ arena_id, index }` handle into that arena, and
  is what most nested expression fields store.
- `ExprArena::get` asserts that a handle is being resolved against its
  originating arena before indexing into the backing node vector.

`Stmt`

- Statement AST nodes.
- Represents syntax that executes for effect: variable declarations, function
  declarations, print statements, `return`, blocks, `if`, `while`, `break`,
  and expression statements.
- `for` does not have its own `Stmt` variant because the parser lowers it to
  more primitive statements during parsing.

### Runtime

`Value`

- Runtime value produced by evaluation.
- Current variants are `String`, `Number`, `Bool`, `Nil`, callable values, and
  class and instance objects.
- This is the value type stored in environments and returned by expression
  evaluation.
- Is defined in `src/runtime/value.rs` and re-exported through
  `src/runtime.rs` so environments and interpreter submodules can share it
  without depending on the runtime file layout.

`LoxCallable`

- Runtime trait implemented by anything Lox can invoke with `()`.
- Defines the callable contract used by native functions and user-defined
  functions.
- Is defined in `src/runtime/object.rs` and re-exported through
  `src/runtime.rs`, while concrete callable implementations live in
  `src/interpreter/callable.rs`.

`LoxFunction`

- Runtime object created when a `fun` declaration executes.
- Captures the surrounding environment so declared functions can keep using the
  scope they were defined in.
- Also represents bound methods and initializers, carrying the extra runtime
  state needed for `this`, `super`, and constructor-return semantics.

`LoxClass`

- Runtime object created when a `class` declaration executes.
- Stores the class name, an optional superclass reference, and a method table
  mapping method names to user-defined callable objects.
- Is itself callable, creating a new instance and then running `init(...)`
  when that initializer method is present.
- Method lookup walks up the superclass chain, which gives subclasses inherited
  behavior and provides the runtime basis for `super`.

`LoxInstance`

- Runtime object created by calling a `LoxClass`.
- Stores its class reference plus an open `HashMap<Rc<str>, Value>` of fields,
  matching the book's "instances are bags of state" model.
- Property reads first check instance fields and then fall back to class
  methods, binding `this` to the original receiver when a method is retrieved.
- Because class lookup walks superclasses too, inherited methods are exposed
  through the same property-read path.
- Property writes always target instance fields.
- The current tree-walk runtime keeps these ownership edges strong on purpose:
  fields own stored `Value`s, classes own methods, and bound methods own their
  receiver through captured `this`. That keeps normal object references and
  escaped method values behaving like ordinary Lox values.
- The tradeoff is that cyclic object graphs are not reclaimed yet. Self-cycles,
  mutually-referential instances, and storing a bound method back onto the
  instance will keep those objects alive until process exit in long-lived
  sessions. This is treated as a known runtime limitation for now, not
  something to "fix" with a small local `Weak` substitution.

`RuntimeError`

- Error raised during execution rather than parsing.
- Carries both a message and the relevant `Token` for source-location
  reporting.
- Is defined in `src/runtime/error.rs` and re-exported through `src/runtime.rs`
  for the same reason as `Value`.

`EnvironmentRef`

- Shared, mutable handle to an environment:
  `Rc<RefCell<Environment>>`.
- Lets the interpreter keep the current environment, while nested scopes still
  point to enclosing ones.

`Environment`

- Stores lexical bindings in two layers: a `HashMap<Rc<str>, usize>` from name
  to slot index, plus a parallel `Vec<Value>` that holds the actual values.
- Optionally points to an enclosing environment to implement lexical scope and
  shadowing.
- Handles `define`, `assign`, and `get`.
- Also provides ancestor-based `get_at` / `assign_at` used by resolved local
  variable access.

`Interpreter`

- Walks the AST and turns syntax into behavior.
- Executes `Stmt` nodes and evaluates `Expr` nodes.
- Owns the current environment and implements the runtime semantics of the
  language.
- Evaluates call expressions by first evaluating the callee and argument
  expressions, then dispatching either through `LoxCallable` or through the
  class-construction path for `LoxClass` values.
- Evaluates property get/set expressions by first evaluating the receiver
  expression, then operating on `LoxInstance` field storage.
- Executes class declarations by lowering parsed methods into a runtime method
  table stored on `LoxClass`.
- When a class has a superclass, creates an extra closure environment so
  methods capture `super` alongside the later method-specific `this` binding.
- Calls classes by allocating a `LoxInstance` that keeps sharing the original
  `Rc<LoxClass>`, then forwards arguments into `init(...)` when present.
- Evaluates `super.method` expressions by reading the captured superclass and
  the current receiver from the environment chain, then rebinding the resolved
  method to that receiver.
- Seeds the global environment with the native `clock()` function.
- Turns function declarations into `LoxFunction` runtime values and binds them
  into the current environment.
- Threads `break` and `return` upward through an internal control-flow enum so
  nested statements can unwind without host-language exceptions.
- Stores the resolver's binding decisions keyed by token id and uses them for
  direct local/global variable lookup at runtime.
- Is implemented as a small module tree:
  `src/interpreter.rs`, `src/interpreter/execute.rs`,
  `src/interpreter/evaluate.rs`, and `src/interpreter/callable.rs`.

## Important Boundaries

`Literal` vs `Value`

- `Literal` belongs to the frontend and describes literal payloads extracted
  from source code.
- `Value` belongs to the runtime and is what the interpreter actually computes
  with.

`Expr` vs `Stmt`

- `Expr` is evaluated for a result.
- `Stmt` is executed for side effects or control flow.

`Environment` vs `EnvironmentRef`

- `Environment` is the scope object itself.
- `EnvironmentRef` is the shared handle used to store and pass environments
  around safely in Rust.

`ParseError` vs `ResolveError` vs `RuntimeError`

- `ParseError` means the source code could not be parsed.
- `ResolveError` means the parsed program failed static binding analysis.
- `RuntimeError` means the parsed program failed while executing.

## Coordination

`src/lox.rs` ties the pipeline together:

- `run_file()` handles script execution
- `run_prompt()` handles the REPL
- `run()` parses a source string into `ParsedProgram`, then feeds it through
  the resolver and into the interpreter
- `run_repl()` classifies REPL input, then either parses a `ParsedExpression`
  for bare-expression echoing or parses a `ParsedProgram` for statement mode
- error flags and reporting helpers keep parse/runtime failures separated

The REPL reuses the same interpreter instance across inputs, so state such as
defined variables survives between prompt entries, and it can buffer
multi-line inputs until braces, strings, comments, and obvious continuation
operators are balanced enough to run.
