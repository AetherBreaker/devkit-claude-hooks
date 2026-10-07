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
