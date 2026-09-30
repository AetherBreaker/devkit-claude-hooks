# TODO

- [ ] **Auto-commit `stop-ruff`'s safe fixes** — `stop-ruff` (`src/stop.rs`) runs
      `ruff check --fix` and leaves whatever it changes uncommitted and unreported: a clean
      fix exits 0, so the hook says nothing and the diff just sits in the tree until someone
      notices `git status`. Auto-commit those changes instead. Constraints found while
      scoping this:
  - The commit must happen inside the same invocation that ran `--fix`, before the
    pass/fail branch — `stop_hook_active` skips the whole hook on a continued turn, so a
    turn where ruff still had unfixable complaints would never get a later chance to
    commit the fixes it already made.
  - On `main`/`master`, `scope()` runs project-wide, so the commit must stage only the
    paths ruff actually touched (diff `git status` around the `--fix` call, or otherwise
    track ruff's fixed-file list) — never a blanket `git add -A`/`git commit -a`, since
    the tree can hold unrelated uncommitted work (a design doc mid-edit, etc.) at Stop
    time that must not get swept in.
  - `stop-pyright` never fixes anything (report-only) and `stop-clean` only deletes
    generated files, so neither is in scope for this — only `stop-ruff` applies.

- [ ] **`stop-pyright` runs pyright without the venv's interpreter** — `resolve()` in
      `src/stop.rs` spawns `.venv/Scripts/pyright.exe` directly to skip `uv run`'s sync
      check, but pyright then picks the interpreter from PATH to find site-packages. On a
      machine where bare `python` is the Microsoft Store alias, the run prints "Python was
      not found" and every import of an installed package is reported unresolved (seen on
      aeth_devkit: 7 false errors per turn, 0 under `uv run pyright`; and on
      ScheduledInvoiceProcessor: ~20 false errors per turn, including knock-on
      `reportUnnecessaryTypeIgnoreComment` hits where the unresolved types turn Unknown).
      The noise repeats after every turn, so it trains everyone to ignore the hook.
      Requirement: pyright must analyse against the project's own venv interpreter whatever
      PATH the hook inherits, with results matching `uv run pyright`. The approach is open:
      the same venv setup `uv run` would give the child process, handing pyright the
      interpreter explicitly, dropping the fast path for pyright, or something else — pick
      whatever holds up on Windows and Linux venv layouts. Ruff is unaffected (no
      interpreter needed); `poe clean` goes through the same path and should be checked.
