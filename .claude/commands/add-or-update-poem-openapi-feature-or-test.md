---
name: add-or-update-poem-openapi-feature-or-test
description: Workflow command scaffold for add-or-update-poem-openapi-feature-or-test in poem.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /add-or-update-poem-openapi-feature-or-test

Use this workflow when working on **add-or-update-poem-openapi-feature-or-test** in `poem`.

## Goal

Implements or updates a feature in poem-openapi, often with changes to source, tests, and sometimes documentation.

## Common Files

- `poem-openapi/src/*.rs`
- `poem-openapi/src/registry/*.rs`
- `poem-openapi/src/types/external/*.rs`
- `poem-openapi/tests/*.rs`
- `poem-openapi/src/docs/*.md`
- `poem-openapi-derive/src/*.rs`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Edit or create files in poem-openapi/src/ (such as openapi.rs, registry/mod.rs, types/external/*.rs)
- Update or add tests in poem-openapi/tests/
- Update or add documentation in poem-openapi/src/docs/
- Update or add derive macros in poem-openapi-derive/src/
- Update Cargo.toml if necessary

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.