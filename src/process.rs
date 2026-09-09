//! Running a program and capturing what it printed: the one seam this crate needs, behind
//! a trait so the tests answer from scripts instead of spawning ruff, pyright or poe. A
//! private copy of the seam `aeth-devkit-core` has: a library repo whose only export is a
//! test seam is not worth a dependency edge (devkit split spec, section 3).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result};

/// What a finished process left behind. `code` is `None` when a signal ended it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapturedOutput {
  pub code: Option<i32>,
  pub stdout: String,
  pub stderr: String,
}

impl CapturedOutput {
  pub fn success(&self) -> bool {
    self.code == Some(0)
  }
}

pub trait Runner {
  /// Run `program args` in `cwd` and capture stdout and stderr.
  fn run_capture(&self, program: &str, args: &[String], cwd: &Path) -> Result<CapturedOutput>;
}

/// Spawns for real.
pub struct SystemRunner;

impl Runner for SystemRunner {
  fn run_capture(&self, program: &str, args: &[String], cwd: &Path) -> Result<CapturedOutput> {
    let out = Command::new(program)
      .args(args)
      .current_dir(cwd)
      .output()
      .with_context(|| format!("running {program}"))?;
    Ok(CapturedOutput {
      code: out.status.code(),
      stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
      stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
  }
}

/// One recorded call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
  pub program: String,
  pub args: Vec<String>,
  pub cwd: PathBuf,
}

struct Script {
  program: String,
  arg_prefix: Vec<String>,
  code: i32,
  stdout: String,
  stderr: String,
}

/// Records every call and answers from scripts; for tests. The most recently registered
/// script whose program matches and whose `arg_prefix` starts the call's arguments wins, so
/// a broad default can be overridden for one call; an unmatched call exits `exit_code` with
/// no output.
pub struct RecordingRunner {
  pub calls: RefCell<Vec<Invocation>>,
  pub exit_code: i32,
  scripts: RefCell<Vec<Script>>,
}

impl RecordingRunner {
  pub fn new(exit_code: i32) -> Self {
    Self {
      calls: RefCell::new(Vec::new()),
      exit_code,
      scripts: RefCell::new(Vec::new()),
    }
  }

  pub fn script(&self, program: &str, arg_prefix: &[&str], code: i32, stdout: &str) -> &Self {
    self.push(program, arg_prefix, code, stdout, "")
  }

  /// A scripted stderr, for callers that read it to decide what a non-zero exit means.
  pub fn script_err(&self, program: &str, arg_prefix: &[&str], code: i32, stderr: &str) -> &Self {
    self.push(program, arg_prefix, code, "", stderr)
  }

  fn push(&self, program: &str, arg_prefix: &[&str], code: i32, stdout: &str, stderr: &str) -> &Self {
    self.scripts.borrow_mut().push(Script {
      program: program.to_string(),
      arg_prefix: arg_prefix.iter().map(|s| s.to_string()).collect(),
      code,
      stdout: stdout.to_string(),
      stderr: stderr.to_string(),
    });
    self
  }

  /// The argument lists of every recorded call to `program`, in order.
  pub fn calls_for(&self, program: &str) -> Vec<Vec<String>> {
    self
      .calls
      .borrow()
      .iter()
      .filter(|c| c.program == program)
      .map(|c| c.args.clone())
      .collect()
  }
}

impl Runner for RecordingRunner {
  fn run_capture(&self, program: &str, args: &[String], cwd: &Path) -> Result<CapturedOutput> {
    self.calls.borrow_mut().push(Invocation {
      program: program.to_string(),
      args: args.to_vec(),
      cwd: cwd.to_path_buf(),
    });
    let scripts = self.scripts.borrow();
    Ok(
      match scripts
        .iter()
        .rev()
        .find(|s| s.program == program && args.starts_with(&s.arg_prefix))
      {
        Some(s) => CapturedOutput {
          code: Some(s.code),
          stdout: s.stdout.clone(),
          stderr: s.stderr.clone(),
        },
        None => CapturedOutput {
          code: Some(self.exit_code),
          ..Default::default()
        },
      },
    )
  }
}
