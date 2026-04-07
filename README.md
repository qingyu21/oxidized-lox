# oxidized-lox

A workspace that tracks two Rust implementations of Lox from *Crafting
Interpreters*:

- `tree-walk/`: the tree-walk interpreter path from Chapters 4 through 13
- `bytecode-vm/`: a fresh workspace member reserved for the bytecode VM path
  from the second half of the book

## Workspace Layout

```text
oxidized-lox/
  examples/        shared Lox programs used by both implementations
  tree-walk/       completed tree-walk interpreter crate
  bytecode-vm/     bytecode VM skeleton crate
```

The shared `examples/` directory stays at the workspace root on purpose so the
same Lox programs can later be run against both implementations.

## Current Milestone

- `tree-walk` is the active, working implementation today.
- `bytecode-vm` is present only as a starting skeleton.
- Optional chapter challenge features and the full bytecode VM remain TODO.

## Running

Run the tree-walk interpreter on a script:

```bash
cargo run -p tree-walk -- examples/print_demo.lox
```

Start the tree-walk REPL:

```bash
cargo run -p tree-walk --
```

The `bytecode-vm` crate currently contains only a placeholder binary:

```bash
cargo run -p bytecode-vm --
```

## Development

Run the full workspace test suite:

```bash
cargo test --workspace
```

Run clippy with warnings treated as errors for every crate:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Format the workspace:

```bash
cargo fmt --all
```

The GitHub Actions workflow mirrors those checks on pushes and pull requests.

## Docs

- [`tree-walk/README.md`](./tree-walk/README.md): tree-walk interpreter usage
  and module map
- [`tree-walk/ARCHITECTURE.md`](./tree-walk/ARCHITECTURE.md): detailed
  pipeline, type, and boundary notes for the tree-walk implementation

## Notes

- The tree-walk interpreter currently supports scanning, parsing, AST
  construction, lexical resolution, closures, classes, inheritance, `this`,
  and `super`.
- Runtime object cycles are still not reclaimed because the tree-walk
  interpreter does not implement a tracing garbage collector yet.
