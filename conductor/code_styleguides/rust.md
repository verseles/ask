# Rust Code Style & Best Practices

This document outlines specific coding principles and best practices for Rust development within this project.

## 1. Safety & Security

-   **Minimize `unsafe`**: Avoid `unsafe` blocks unless absolutely necessary for FFI or performance-critical sections where the safety can be manually verified and documented. Always wrap `unsafe` code in safe abstractions.
-   **Input Validation**: Validate all external inputs at the boundary of your system. Use the type system to enforce valid states (e.g., "Parse, don't validate").
-   **Dependency Management**: Keep dependencies updated. Regularly run `cargo audit` to check for security vulnerabilities.

## 2. Idiomatic Rust

-   **Ownership & Borrowing**: Embrace the ownership model. Prefer borrowing (`&T`, `&mut T`) over taking ownership when data doesn't need to be consumed.
-   **Type System**: Use the type system to express intent. Avoid "stringly-typed" APIs; use `enum`s and `struct`s to represent distinct states and data.
-   **Iterators**: Prefer iterators (`.iter()`, `.map()`, `.filter()`) over raw `for` loops for complex transformations. They are often faster (zero-cost abstractions) and more readable.
-   **Option & Result**: Use `Option` for values that may be absent and `Result` for operations that can fail. Never use nulls (Rust doesn't have them, but don't emulate them with magic values).
-   **Clippy**: Treat `cargo clippy` warnings as errors. They often point out non-idiomatic or inefficient code.

## 3. Error Handling

-   **Recoverable vs. Unrecoverable**: Use `Result` for recoverable errors. Reserve `panic!` for unrecoverable bugs or broken invariants.
-   **Propagation**: Use the `?` operator to propagate errors upwards. It reduces boilerplate compared to `match`.
-   **Avoid `unwrap()`/`expect()`**: In production code, avoid `.unwrap()` and `.expect()`. Handle errors gracefully. If you must use them (e.g., in tests or when you are 100% sure), use `.expect()` with a descriptive message.
-   **Custom Errors**:
    -   Use `thiserror` for library code to define specific, structural error types.
    -   Use `anyhow` for application code (CLI mains) where you need to handle various errors uniformly.

## 4. Performance

-   **Async/Await**: Use `async`/`await` for I/O-bound operations. Be mindful of blocking the async runtime; use `tokio::task::spawn_blocking` for CPU-intensive tasks.
-   **Clone Responsibly**: Avoid unnecessary `.clone()` calls. If cloning is expensive, consider using `Arc` or `Rc` for shared ownership, or unnecessary clones might indicate an ownership design issue.
-   **Release Builds**: Always measure performance in release mode (`cargo build --release`).

## 5. Code Style & Formatting

-   **Rustfmt**: Always run `cargo fmt` before committing. We follow standard community formatting rules.
-   **Naming Conventions**:
    -   Types (Structs, Enums, Traits): `PascalCase`
    -   Variables, Functions, Modules: `snake_case`
    -   Constants, Statics: `SCREAMING_SNAKE_CASE`
-   **Imports**: Group imports logically (std, external crates, internal modules).

## 6. API Design

-   **Documentation**: Document all public structs, enums, functions, and modules using `///` comments. Include examples where possible.
-   **Builder Pattern**: Use the Builder pattern for constructing complex structs with many optional configuration parameters.
-   **Traits**: Implement common traits like `Debug`, `Clone`, `Default`, `Display` where appropriate. Use `From`/`TryFrom` for type conversions.

## 7. Testing

-   **Unit Tests**: Place unit tests in a `tests` module within the same file as the code they test.
-   **Integration Tests**: Place integration tests in the `tests/` directory at the project root.
-   **Doc Tests**: Examples in documentation are automatically run as tests; ensure they compile and run correctly.
