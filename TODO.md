# TODO

## stop-pyright

- [ ] Skip nested projects. `stop-pyright` runs pyright from the project root, so a git
      submodule or any subdirectory with its own `pyproject.toml` is checked with the
      parent's venv and settings: its imports don't resolve and its own `[tool.pyright]`
      (a Linux `pythonPlatform`, say) is ignored, so every stop reports false errors (seen
      with pos-tunnel's `relay/` submodule; pos-tunnel now excludes it by hand). Leave such
      directories out of the run, or check each with its own root and venv.
