# branch

[![Release](https://img.shields.io/github/v/release/piotr-lebski/branch?display_name=tag&sort=semver)](https://github.com/piotr-lebski/branch/releases)
[![License](https://img.shields.io/github/license/piotr-lebski/branch)](https://github.com/piotr-lebski/branch/blob/main/LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-2ea44f)](https://github.com/piotr-lebski/branch/actions/workflows/ci.yml)

An interactive git branch and worktree navigator. Browse local or remote branches and worktrees, check out branches, cd
into worktrees, and delete what you no longer need — all from a single keyboard-driven picker.

## Install

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/piotr-lebski/branch/main/install.sh | bash
```

Installs the latest release to `~/.local/bin` and sets up shell integration automatically. To pass flags, pipe through
`bash -s --`:

```sh
curl -fsSL https://raw.githubusercontent.com/piotr-lebski/branch/main/install.sh \
  | bash -s -- --version v0.1.0 --no-shell-integration
```

Run `curl … | bash -s -- --help` to see all options.

### Windows (PowerShell)

```powershell
iwr -useb https://raw.githubusercontent.com/piotr-lebski/branch/main/install.ps1 | iex
```

Installs to `$env:LOCALAPPDATA\branch\bin` and updates your PowerShell profile.
To pass flags, download and run the script directly:

```powershell
iwr -useb https://raw.githubusercontent.com/piotr-lebski/branch/main/install.ps1 -OutFile install.ps1
.\install.ps1 -Version v0.1.0 -NoShellIntegration
```

### Build from source

Requires [Rust](https://rustup.rs):

```sh
cargo build --release
cp target/release/branch ~/.local/bin/branch   # any directory on your PATH
```

## Shell Integration

`branch` works by printing a selected worktree path to stdout, which the shell wrapper captures and passes to `cd`.
Without the wrapper, `branch` still navigates branches — the wrapper is what enables directory changes for worktrees.

Add the appropriate line to your shell config so it is sourced on every new session, then restart your shell or
re-source the config file to apply immediately (e.g. `source ~/.bashrc`).

### Bash

Add to `~/.bashrc`:

```sh
eval "$(branch --init)"
```

### Zsh

Add to `~/.zshrc`:

```sh
eval "$(branch --init)"
```

### Fish

Add to `~/.config/fish/config.fish`:

```fish
branch --init | source
```

### PowerShell

Add to your PowerShell profile (`$PROFILE`):

```powershell
Invoke-Expression ((& branch --init) -join "`n")
```

### Auto-detection

`branch --init` auto-detects your shell. If detection fails, pass the shell name explicitly:

```sh
branch --init bash      # or: zsh, fish, powershell
```

## Usage

```sh
branch             # Interactive picker — browse branches and worktrees
branch --remote    # Browse remote-tracking branches instead of local ones
```

### Interactive Actions

Select any item with the arrow keys and press Enter to choose an action:

| Item     | Actions                                                                                      |
| -------- | -------------------------------------------------------------------------------------------- |
| Branch   | **Checkout** the branch, **Delete** it, or Cancel                                            |
| Worktree | **cd** into the worktree directory, **Remove** it (with optional branch deletion), or Cancel |

The current branch is marked with `*`. Deleting a branch that is not fully merged will prompt for confirmation before
force-deleting.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution guidelines.
