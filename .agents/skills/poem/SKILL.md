```markdown
# poem Development Patterns

> Auto-generated skill from repository analysis

## Overview

This skill teaches you how to contribute to the `poem` Rust repository, which is organized as a multi-crate project focused on web frameworks and OpenAPI support. You'll learn the project's coding conventions, commit patterns, and the main workflows for adding features, updating dependencies, and maintaining example projects. The guide includes step-by-step instructions, code style examples, and recommended commands for common tasks.

## Coding Conventions

- **File Naming:** Use `camelCase` for file names.
  - Example: `streamableHttp.rs`, `toolServer.rs`
- **Import Style:** Use relative imports within modules.
  - Example:
    ```rust
    mod protocol;
    use super::streamableHttp;
    ```
- **Export Style:** Use named exports.
  - Example:
    ```rust
    pub mod protocol;
    pub use self::toolServer::ToolServer;
    ```
- **Commit Messages:** Follow [Conventional Commits](https://www.conventionalcommits.org/) with these prefixes:
  - `feat`: New feature
  - `fix`: Bug fix
  - `chore`: Maintenance
  - `perf`: Performance improvement
  - Average commit message length: ~52 characters

## Workflows

### Add or Update MCPServer Feature
**Trigger:** When you want to add a new capability or resource type to `poem-mcpserver`.
**Command:** `/add-mcpserver-feature`

1. Edit or create files in `poem-mcpserver/src/` (e.g., `protocol/*.rs`, `server.rs`, `streamableHttp.rs`, `stdio.rs`, `tool.rs`, `resources.rs`, `lib.rs`).
2. Update or add macros in `poem-mcpserver-macros/src/`.
3. Update or add tests in `poem-mcpserver/tests/`.
4. Update or add example projects in `examples/mcpserver/`.
5. Update `Cargo.toml` if necessary.
6. Commit with a conventional message, e.g.:
    ```
    feat(mcpserver): add support for new resource type
    ```

**Example:**
```rust
// poem-mcpserver/src/protocol/newResource.rs
pub struct NewResource { /* ... */ }
```

### Add or Update Poem OpenAPI Feature or Test
**Trigger:** When you want to add a new OpenAPI capability or fix/extend OpenAPI support.
**Command:** `/add-openapi-feature`

1. Edit or create files in `poem-openapi/src/` (e.g., `openapi.rs`, `registry/mod.rs`, `types/external/*.rs`).
2. Update or add tests in `poem-openapi/tests/`.
3. Update or add documentation in `poem-openapi/src/docs/`.
4. Update or add derive macros in `poem-openapi-derive/src/`.
5. Update `Cargo.toml` if necessary.
6. Commit with a conventional message, e.g.:
    ```
    feat(openapi): support for custom schema types
    ```

**Example:**
```rust
// poem-openapi/src/types/external/customType.rs
pub struct CustomType { /* ... */ }
```

### Bump Crate Version or Dependency
**Trigger:** When you want to release a new version or update a dependency.
**Command:** `/bump-version`

1. Edit `Cargo.toml` in one or more crates to bump the version or update a dependency.
2. Optionally update related source files if there are API changes.
3. Commit with a message indicating the version bump or dependency update, e.g.:
    ```
    chore: bump poem-openapi to 2.1.0
    ```

**Example:**
```toml
# poem-openapi/Cargo.toml
[dependencies]
serde = "1.0.160"
```

### Add or Update Example Project
**Trigger:** When you want to provide a new usage example or update an existing one.
**Command:** `/add-example`

1. Create or edit `examples/*/Cargo.toml`.
2. Create or edit `examples/*/src/*.rs`.
3. Optionally update `README.md` or add assets (e.g., images, HTML) in the example directory.
4. Commit with a message referencing the example, e.g.:
    ```
    feat(example): add websocket chat example
    ```

**Example:**
```rust
// examples/websocketChat/src/main.rs
fn main() {
    println!("WebSocket chat example");
}
```

## Testing Patterns

- **Framework:** Not explicitly specified, but Rust's built-in test framework is likely used.
- **Test File Pattern:** Test files are named with the `.test.ts` extension (possibly for TypeScript-based tests or integration with JS tooling).
- **Location:** Tests are located in `*/tests/*.rs`.
- **Example:**
    ```rust
    // poem-mcpserver/tests/protocolTest.rs
    #[test]
    fn test_protocol_behavior() {
        // test implementation
    }
    ```

## Commands

| Command                 | Purpose                                                      |
|-------------------------|--------------------------------------------------------------|
| /add-mcpserver-feature  | Add or update a feature in poem-mcpserver                    |
| /add-openapi-feature    | Add or update a feature or test in poem-openapi              |
| /bump-version           | Bump crate version or update dependency                      |
| /add-example            | Add or update an example project                             |
```
