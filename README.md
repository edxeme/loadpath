# loadpath

Small polyglot experiment: critical-path analysis over a weighted task DAG,
implemented independently in Java, Rust and C++ with the same test vectors.

| Module | Language | Build | Test |
| --- | --- | --- | --- |
| `java/` | Java 21+ (Maven) | `mvn -q package` | `mvn test` |
| `rust/` | Rust 2021 (Cargo) | `cargo build` | `cargo test` |
| `cpp/`  | C++17 (CMake) | `cmake -B build && cmake --build build` | `ctest --test-dir build` |

## Algorithm

Given tasks with durations and dependency edges, compute the earliest finish
time of every task and the longest (critical) path through the DAG. Cycles are
rejected with an error.

## Layout

Each module is self-contained and mirrors the same API: `addTask`, `addDependency`,
`criticalPath`. Shared test vectors live in `fixtures/`.
