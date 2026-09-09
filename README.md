# devkit-claude-hooks

`devkit-hook <name>`: the Claude Code hooks of every devkit-managed project. `devkit
setup-project` adds this package to the project's dev group at the newest release the
installed devkit accepts and writes the five hook lines into `.claude/settings.local.json`,
pointing at the venv's `devkit-hook`. Payload on stdin, at most one JSON line on stdout,
always exits 0: every failure path degrades to silence, because a non-zero exit is shown
as a hook error in every session.

- **`pre-edit-protect`** - Denies Edit/Write to `.env` and `uv.lock`, matched on the
  basename with Windows name normalization.
- **`pre-bash-protect-deps`** - Denies `uv add|remove|lock` via a quote-aware command
  tokenizer (handles wrappers, env-var prefixes, `bash -c` recursion, and uv's
  value-taking global flags — not a regex).
- **Stop hooks** - Re-report tool failures as `additionalContext`: `stop-ruff` (`--fix
  --unfixable F401`) scoped to the branch diff, `stop-pyright` project-wide on purpose,
  `stop-clean` (`poe clean`); venv binaries preferred over `uv run`; output capped at
  4000 chars; `stop_hook_active` loop guard.

## Develop

```sh
uv sync                 # builds the crate into .venv through maturin
cargo test
uv run devkit-hook --version
```

The repository is devkit-managed: `poe setup-project` keeps the shared configuration
current. Release with `poe release`; the wheel goes to SFTPyPI and every project takes the
new version on its next `poe setup-project`.
