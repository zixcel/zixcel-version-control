use crate::{
    ObservationError, SOURCE_REVISION_OBSERVATION_SCHEMA_V1, SourceRevisionObservation,
    WorkingState,
};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

const MAX_GIT_OUTPUT: usize = 1_048_576;

pub struct GitObserver;

impl GitObserver {
    /// Observes only the selected root. Parent-directory discovery, remotes,
    /// credentials, source contents and Git hooks are outside this boundary.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the selected root is invalid, Git cannot be
    /// started, its output exceeds the bound, or object identities are invalid.
    pub fn observe(
        root: impl AsRef<Path>,
        observed_at: &str,
    ) -> Result<Option<SourceRevisionObservation>, ObservationError> {
        let root = root.as_ref();
        let metadata =
            fs::metadata(root).map_err(|error| ObservationError::Unavailable(error.to_string()))?;
        if !metadata.is_dir() {
            return Err(ObservationError::Unavailable(
                "selected root is not a directory".into(),
            ));
        }
        if !root.join(".git").exists() {
            return Ok(None);
        }
        if observed_at.is_empty() || observed_at.len() > 128 {
            return Err(ObservationError::Invalid(
                "observed_at must be a bounded timestamp".into(),
            ));
        }

        let canonical = fs::canonicalize(root)
            .map_err(|error| ObservationError::Unavailable(error.to_string()))?;
        let repository_ref = format!(
            "repository/{}",
            hex_digest(canonical.to_string_lossy().as_bytes())
        );
        let revision = optional_git(root, &["rev-parse", "--verify", "HEAD"])?;
        let content_root = optional_git(root, &["rev-parse", "--verify", "HEAD^{tree}"])?;
        let branch = optional_git(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
        let parents = if revision.is_some() {
            git(root, &["show", "-s", "--format=%P", "HEAD"])?
                .split_ascii_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for value in revision
            .iter()
            .chain(content_root.iter())
            .chain(parents.iter())
        {
            validate_object_id(value)?;
        }

        let status = git_bytes(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=normal"],
        )?;
        let entries = status
            .split(|byte| *byte == 0)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let working_state = if entries.iter().any(|entry| {
            entry.len() >= 2
                && (entry[0] == b'U'
                    || entry[1] == b'U'
                    || matches!((entry[0], entry[1]), (b'A', b'A') | (b'D', b'D')))
        }) {
            WorkingState::Conflicted
        } else if entries.iter().any(|entry| entry.starts_with(b"??")) {
            WorkingState::Untracked
        } else if entries.is_empty() {
            WorkingState::Clean
        } else {
            WorkingState::Modified
        };
        let changed_entry_count =
            u32::try_from(entries.len()).map_err(|_| ObservationError::OutputLimit)?;

        let identity = format!(
            "git\n{repository_ref}\n{}\n{}\n{}\n{working_state:?}\n{changed_entry_count}\n{}",
            revision.as_deref().unwrap_or("unborn"),
            content_root.as_deref().unwrap_or("unborn"),
            parents.join(" "),
            branch.as_deref().unwrap_or("detached")
        );
        let evidence_digest_sha256 = hex_digest(identity.as_bytes());
        Ok(Some(SourceRevisionObservation {
            schema: SOURCE_REVISION_OBSERVATION_SCHEMA_V1.into(),
            observation_ref: format!("version-control/observation/{evidence_digest_sha256}"),
            provider: "git".into(),
            repository_ref,
            immutable_revision_ref: revision,
            parent_revision_refs: parents,
            content_root_ref: content_root,
            branch_hint: branch,
            working_state,
            changed_entry_count,
            observed_at: observed_at.into(),
            evidence_digest_sha256,
            adapter_ref: "zixcel-version-control/git/0.10.0".into(),
        }))
    }
}

fn optional_git(root: &Path, arguments: &[&str]) -> Result<Option<String>, ObservationError> {
    match run_git(root, arguments)? {
        (true, output) => {
            let value = String::from_utf8(output)
                .map_err(|_| ObservationError::Invalid("Git output is not UTF-8".into()))?;
            let value = value.trim();
            (!value.is_empty())
                .then(|| value.to_owned())
                .map_or(Ok(None), |value| Ok(Some(value)))
        }
        (false, _) => Ok(None),
    }
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, ObservationError> {
    let (success, output) = run_git(root, arguments)?;
    if !success {
        return Err(ObservationError::Invalid(format!(
            "Git command `{}` failed",
            arguments.first().copied().unwrap_or("unknown")
        )));
    }
    String::from_utf8(output)
        .map(|value| value.trim().to_owned())
        .map_err(|_| ObservationError::Invalid("Git output is not UTF-8".into()))
}

fn git_bytes(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, ObservationError> {
    let (success, output) = run_git(root, arguments)?;
    if success {
        Ok(output)
    } else {
        Err(ObservationError::Invalid(format!(
            "Git command `{}` failed",
            arguments.first().copied().unwrap_or("unknown")
        )))
    }
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<(bool, Vec<u8>), ObservationError> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ObservationError::AdapterUnavailable)?;
    let mut output = Vec::new();
    let mut stdout = child
        .stdout
        .take()
        .ok_or(ObservationError::AdapterUnavailable)?;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = stdout
            .read(&mut buffer)
            .map_err(|error| ObservationError::Invalid(error.to_string()))?;
        if read == 0 {
            break;
        }
        if output.len().saturating_add(read) > MAX_GIT_OUTPUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ObservationError::OutputLimit);
        }
        output.extend_from_slice(&buffer[..read]);
    }
    let status = child
        .wait()
        .map_err(|error| ObservationError::Invalid(error.to_string()))?;
    Ok((status.success(), output))
}

fn validate_object_id(value: &str) -> Result<(), ObservationError> {
    if matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(ObservationError::Invalid(
            "invalid immutable object identity".into(),
        ))
    }
}

fn hex_digest(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run(root: &Path, arguments: &[&str]) {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(arguments)
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    fn absence_is_optional_and_commit_then_change_is_observable() {
        let directory = tempfile::tempdir().unwrap();
        assert!(
            GitObserver::observe(directory.path(), "2026-09-29T00:00:00Z")
                .unwrap()
                .is_none()
        );

        run(directory.path(), &["init", "--quiet"]);
        run(directory.path(), &["config", "user.name", "Scenario User"]);
        run(
            directory.path(),
            &["config", "user.email", "scenario@example.invalid"],
        );
        fs::write(directory.path().join("work.sem"), "# Work\n").unwrap();
        run(directory.path(), &["add", "work.sem"]);
        run(directory.path(), &["commit", "--quiet", "-m", "add work"]);

        let clean = GitObserver::observe(directory.path(), "2026-09-29T00:00:00Z")
            .unwrap()
            .unwrap();
        assert_eq!(clean.working_state, WorkingState::Clean);
        assert!(clean.immutable_revision_ref.is_some());
        assert_eq!(clean.changed_entry_count, 0);

        fs::write(
            directory.path().join("work.sem"),
            "# Work\n  state: changed\n",
        )
        .unwrap();
        let changed = GitObserver::observe(directory.path(), "2026-09-29T00:01:00Z")
            .unwrap()
            .unwrap();
        assert_eq!(changed.working_state, WorkingState::Modified);
        assert_eq!(changed.changed_entry_count, 1);
        assert_ne!(changed.observation_ref, clean.observation_ref);
    }

    #[test]
    fn exact_retry_has_stable_evidence_identity() {
        let directory = tempfile::tempdir().unwrap();
        run(directory.path(), &["init", "--quiet"]);
        let first = GitObserver::observe(directory.path(), "2026-09-29T00:00:00Z")
            .unwrap()
            .unwrap();
        let retry = GitObserver::observe(directory.path(), "2026-09-29T00:02:00Z")
            .unwrap()
            .unwrap();
        assert_eq!(first.evidence_digest_sha256, retry.evidence_digest_sha256);
        assert_eq!(first.observation_ref, retry.observation_ref);
        assert_ne!(first.observed_at, retry.observed_at);
    }
}
