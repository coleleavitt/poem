---
name: add-or-update-mcpserver-feature
description: Workflow command scaffold for add-or-update-mcpserver-feature in poem.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /add-or-update-mcpserver-feature

Use this workflow when working on **add-or-update-mcpserver-feature** in `poem`.

## Goal

Implements or updates a feature in poem-mcpserver, often with changes to protocol, server logic, macros, and corresponding tests/examples.

## Common Files

- `poem-mcpserver/src/*.rs`
- `poem-mcpserver/src/protocol/*.rs`
- `poem-mcpserver-macros/src/*.rs`
- `poem-mcpserver/tests/*.rs`
- `examples/mcpserver/*/Cargo.toml`
- `examples/mcpserver/*/src/*.rs`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Edit or create files in poem-mcpserver/src/ (such as protocol/*.rs, server.rs, streamable_http.rs, stdio.rs, tool.rs, resources.rs, lib.rs)
- Update or add macros in poem-mcpserver-macros/src/
- Update or add tests in poem-mcpserver/tests/
- Update or add example projects in examples/mcpserver/
- Update Cargo.toml if necessary

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.