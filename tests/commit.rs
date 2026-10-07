//! `stop-ruff` committing its own fixes, against a real git repo: what gets committed
//! depends on git's status format, pathspec rules and detached-HEAD handling, which only the
//! real thing answers honestly.

use std::path::Path;
use std::process::Command;

use devkit_claude_hooks::process::{CapturedOutput, Env, Runner, SystemRunner};
use devkit_claude_hooks::{Hook, run};
use serde_json::Value;

/// Real git; anything else plays ruff: appends a line to each of `fixes` (repo-root-relative)
/// and exits `code` with `stdout`.
struct FakeRuff<'a> {
  root: &'a Path,
  fixes: &'a [&'a str],
  code: i32,
  stdout: &'a str,
}

impl Runner for FakeRuff<'_> {
  fn run_capture_env(&self, program: &str, args: &[String], env: &Env, cwd: &Path) -> anyhow::Result<CapturedOutput> {
    if program == "git" {
      return SystemRunner.run_capture_env(program, args, env, cwd);
    }
    for f in self.fixes {
      let p = self.root.join(f);
      let text = std::fs::read_to_string(&p).unwrap();
      std::fs::write(&p, format!("{text}fixed = 1\n")).unwrap();
    }
    Ok(CapturedOutput {
      code: Some(self.code),
      stdout: self.stdout.to_string(),
      stderr: String::new(),
    })
  }
}

fn git(root: &Path, args: &[&str]) -> String {
  let out = Command::new("git").args(args).current_dir(root).output().unwrap();
  assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
  String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A repo on `main` with `files` committed, configured so a commit works anywhere.
fn repo(files: &[&str]) -> tempfile::TempDir {
  let dir = tempfile::tempdir().unwrap();
  let root = dir.path();
  git(root, &["init", "-q", "-b", "main"]);
  for (k, v) in [
    ("user.name", "t"),
    ("user.email", "t@t"),
    ("commit.gpgsign", "false"),
    ("core.autocrlf", "false"),
  ] {
    git(root, &["config", k, v]);
  }
  for f in files {
    let p = root.join(f);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "x = 1\n").unwrap();
  }
  git(root, &["add", "-A"]);
  git(root, &["commit", "-q", "-m", "init"]);
  dir
}

fn changed_in_head(root: &Path) -> Vec<String> {
  git(root, &["-c", "core.quotePath=false", "show", "--name-only", "--format=", "HEAD"])
    .lines()
    .map(String::from)
    .collect()
}

#[test]
fn only_files_that_were_clean_before_ruff_are_committed() {
  let dir = repo(&["clean.py", "edited.py", "staged.txt"]);
  let root = dir.path();
  // Work already in the tree: an unstaged edit ruff then also fixes, a staged change ruff
  // never touches, and a new untracked file.
  std::fs::write(root.join("edited.py"), "x = 2\n").unwrap();
  std::fs::write(root.join("staged.txt"), "staged\n").unwrap();
  git(root, &["add", "staged.txt"]);
  std::fs::write(root.join("new.py"), "y = 1\n").unwrap();
  let ruff = FakeRuff {
    root,
    fixes: &["clean.py", "edited.py", "new.py"],
    code: 0,
    stdout: "",
  };

  assert_eq!(run(Hook::StopRuff, "{}", root, &ruff), None);

  assert_eq!(git(root, &["log", "-1", "--format=%s"]).trim(), "style: apply ruff's safe fixes");
  assert_eq!(changed_in_head(root), ["clean.py"]);
  let status = git(root, &["status", "--porcelain"]);
  assert!(status.contains(" M edited.py"), "{status}");
  assert!(status.contains("M  staged.txt"), "still staged, not committed: {status}");
  assert!(status.contains("?? new.py"), "{status}");
}

#[test]
fn a_failing_run_still_commits_and_says_so() {
  let dir = repo(&["a.py"]);
  let root = dir.path();
  let ruff = FakeRuff {
    root,
    fixes: &["a.py"],
    code: 1,
    stdout: "a.py:1:1: E501 too long\n",
  };

  let out = run(Hook::StopRuff, "{}", root, &ruff).expect("must report");
  let v: Value = serde_json::from_str(&out).unwrap();
  let ctx = v["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
  assert!(
    ctx.ends_with("E501 too long\n(its safe fixes to 1 file(s) were committed)"),
    "{ctx}"
  );
  assert_eq!(changed_in_head(root), ["a.py"]);
}

#[test]
fn nothing_is_committed_on_a_detached_head() {
  let dir = repo(&["a.py"]);
  let root = dir.path();
  git(root, &["checkout", "-q", "--detach"]);
  let head = git(root, &["rev-parse", "HEAD"]);
  let ruff = FakeRuff {
    root,
    fixes: &["a.py"],
    code: 0,
    stdout: "",
  };

  run(Hook::StopRuff, "{}", root, &ruff);

  assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
  assert!(git(root, &["status", "--porcelain"]).contains(" M a.py"));
}

#[test]
fn paths_resolve_from_the_repo_root_and_are_not_globbed() {
  // Claude opened at `app/`: status names `app/[x].py` from the root, so the pathspec must be
  // root-anchored, and `[x]` taken literally rather than as a character class matching `x.py`.
  let dir = repo(&["app/[x].py", "app/x.py"]);
  let root = dir.path();
  let ruff = FakeRuff {
    root,
    fixes: &["app/[x].py"],
    code: 0,
    stdout: "",
  };

  run(Hook::StopRuff, "{}", &root.join("app"), &ruff);

  assert_eq!(changed_in_head(root), ["app/[x].py"]);
}
