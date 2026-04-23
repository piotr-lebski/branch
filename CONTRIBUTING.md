# Contributing to goto

Contributions are welcome. This project is small, so the process is intentionally lightweight.

## Before You Start

Small fixes, tests, and documentation improvements can go straight to a pull request.

For larger changes, new features, or behavior changes, open an issue first so the direction can be agreed before you
spend time implementing it.

If you are unsure whether something is a good fit, open a draft pull request.

## Good Contributions

- Bug fixes with a clear reproduction or failing case
- Tests that cover missing behavior or prevent regressions
- Documentation improvements
- Small UX improvements that keep the CLI focused and predictable

## Development Setup

This is a standard Rust project.

```sh
cargo build
cargo test
```

Before opening a pull request, run:

```sh
cargo fmt
cargo test
```

If you have Clippy installed, it is also worth running:

```sh
cargo clippy --all-targets --all-features
```

## Pull Request Guidelines

- Keep pull requests focused on one change
- Add or update tests when behavior changes
- Update README or CLI help text when user-facing behavior changes
- Explain the problem, the approach, and any tradeoffs in the pull request description
- Mention platform or shell-specific impact when relevant

## Project Notes

- Shell integration changes should consider all supported schells
- Avoid broad refactors unless they are necessary for the change being made

## Reporting Bugs

When filing a bug, include:

- What you expected to happen
- What actually happened
- Your OS and shell
- Steps to reproduce the issue
- Example output or error messages, if available

## License

By contributing, you agree that your contributions will be licensed under the MIT License used by this repository.
