# Contributing to OmniOwn

Thank you for your interest in contributing to OmniOwn. This guide explains how to set up the project, run local checks, and prepare a pull request.

## Development Setup

### Required Tools

- **Rust** stable, edition 2024: https://rustup.rs
- **Node.js** >= 20: https://nodejs.org
- **npm**, installed with Node.js

### Clone and Install

```bash
git clone https://github.com/zj-rrissh/omniown.git
cd omniown

# Rust
cargo build

# Node.js API
npm --prefix server install
npm --prefix server run build

# Vue frontend
npm --prefix ui install
```

## Development Workflow

### Branching

1. Create a feature branch from `main`: `git checkout -b feat/your-feature`
2. Make your changes and commit them.
3. Push the branch: `git push origin feat/your-feature`
4. Open a pull request into `main`.

### Local Verification

Run the full check suite before submitting:

```bash
# Rust
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings

# Node.js API
npm --prefix server run build

# Vue frontend
npm --prefix ui run build

# Tauri desktop
cargo test --manifest-path src-tauri/Cargo.toml
```

If available in your local environment, use the project `pr-ready` check script to run the standard Rust checks together.

## Code Style

### Rust

- Use Rust edition 2024.
- Keep tests in source files with `#[cfg(test)] mod tests { ... }`; do not add a separate `tests/` directory unless there is a clear reason.
- Format with `cargo fmt`; CI enforces formatting.
- Keep `cargo clippy -- -D warnings` clean; CI treats warnings as failures.
- Do not commit temporary debugging code such as `dbg!()` or `todo!()`.

### TypeScript: `server/`

- Strict mode is enabled in `tsconfig.json`.
- The server uses ESM with `"type": "module"`; local imports should include the `.js` extension.
- Module resolution is `NodeNext`.
- Prisma is pinned to v5 for now; do not upgrade to v6/v7 without a migration plan.
- Database fields use camelCase in TypeScript and map to snake_case columns with `@map("snake_case")`.

### TypeScript: `ui/`

- Strict mode is enabled in `tsconfig.json`.
- The production build is `vue-tsc --noEmit && vite build`.
- Vue components should use `<script setup lang="ts">` and the Composition API.
- Shared state should live in Pinia stores.
- API calls should go through the `services/` layer.

## Commit Messages

Use concise, structured commit messages:

```text
<type>: <short summary>

- <specific change 1>
- <specific change 2>
```

Common types:

| Type | Use case |
|:---|:---|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Refactoring |
| `docs` | Documentation |
| `chore` | Build or tooling |
| `test` | Tests |

## Project Architecture

OmniOwn is split into four layers:

| Layer | Technology | Responsibility |
|:---|:---|:---|
| `src/` | Rust | Text extraction, file import, SQLite/FTS, MCP server, filesystem watching |
| `server/` | Node.js / TypeScript | REST API, Prisma ORM, FTS5 search, AI search orchestration |
| `ui/` | Vue 3 / TypeScript | Search, documents, configuration, and status views |
| `src-tauri/` | Tauri v2 | Desktop shell, tray, floating panel, bundled Node runtime, subprocess management |

## Pull Request Checklist

1. Run the local checks.
2. Add or update tests for behavior changes.
3. Update related files under `docs/` when changing public behavior, APIs, or configuration.
4. In the PR description, explain what changed, why it changed, and how you verified it.
5. Wait for CI to pass before requesting review.

## Reporting Issues

- Bugs: open an issue with the Bug Report template.
- Feature requests: open an issue with the Feature Request template.
- Questions or design discussions: open an issue describing the scenario and expected behavior.
