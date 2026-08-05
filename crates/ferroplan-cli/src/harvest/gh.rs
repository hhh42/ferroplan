use std::collections::BTreeMap;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::{
    digest_bytes, validate_window, ArtifactEvidence, ExecutionEvidence, ExecutionResult,
    ObservationPack, ObservationWindow, ObservedWorkItem, TransportFailure, OBSERVATION_SCHEMA,
};

const PAGE_SIZE: usize = 100;

pub fn collect_with_gh(
    repositories: &[String],
    window: ObservationWindow,
    max_pages: usize,
) -> Result<ObservationPack> {
    validate_window(&window)?;
    let mut failures = Vec::new();
    let mut work_items = Vec::new();

    if !gh_available() {
        failures.push(TransportFailure {
            repository: "*".to_owned(),
            operation: "gh --version".to_owned(),
            state: "BLOCKED".to_owned(),
            detail: "GitHub CLI is unavailable; no repository work was observed".to_owned(),
        });
    } else {
        for repository in repositories {
            match collect_repository(repository, &window, max_pages.max(1)) {
                Ok((mut observed, mut repository_failures)) => {
                    work_items.append(&mut observed);
                    failures.append(&mut repository_failures);
                }
                Err(error) => failures.push(TransportFailure {
                    repository: repository.clone(),
                    operation: "collect repository observation".to_owned(),
                    state: "BLOCKED".to_owned(),
                    detail: error.to_string(),
                }),
            }
        }
    }

    work_items.sort_by(|left, right| {
        left.repository
            .cmp(&right.repository)
            .then(left.committed_at_utc.cmp(&right.committed_at_utc))
            .then(left.sha.cmp(&right.sha))
    });
    failures.sort_by(|left, right| {
        left.repository
            .cmp(&right.repository)
            .then(left.operation.cmp(&right.operation))
            .then(left.detail.cmp(&right.detail))
    });
    let run_seed = format!(
        "{}\n{}\n{}",
        window.start_utc,
        window.end_exclusive_utc,
        repositories.join("\n")
    );

    Ok(ObservationPack {
        schema: OBSERVATION_SCHEMA.to_owned(),
        run_id: format!("gh-{}", &digest_bytes(run_seed.as_bytes())[..16]),
        window,
        repositories: repositories.to_vec(),
        work_items,
        transport_failures: failures,
    })
}

fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn collect_repository(
    repository: &str,
    window: &ObservationWindow,
    max_pages: usize,
) -> Result<(Vec<ObservedWorkItem>, Vec<TransportFailure>)> {
    validate_repository(repository)?;
    let (runs, failures) = collect_runs(repository, window, max_pages)?;
    let mut items = Vec::new();

    for page in 1..=max_pages {
        let endpoint = format!(
            "repos/{repository}/commits?since={}&until={}&per_page={PAGE_SIZE}&page={page}",
            window.start_utc, window.end_exclusive_utc
        );
        let summaries: Vec<CommitSummary> = gh_api(&endpoint)?;
        if summaries.is_empty() {
            break;
        }
        let count = summaries.len();
        for summary in summaries {
            let detail: CommitDetail =
                gh_api(&format!("repos/{repository}/commits/{}", summary.sha))?;
            let (executions, artifacts) = runs.get(&detail.sha).cloned().unwrap_or_default();
            let committed_at = detail
                .commit
                .committer
                .as_ref()
                .or(detail.commit.author.as_ref())
                .map(|identity| identity.date.clone())
                .unwrap_or_default();
            items.push(ObservedWorkItem {
                repository: repository.to_owned(),
                sha: detail.sha,
                parent_sha: detail.parents.first().map(|parent| parent.sha.clone()),
                message: detail.commit.message,
                committed_at_utc: committed_at,
                source_url: detail.html_url,
                changed_paths: detail.files.into_iter().map(|file| file.filename).collect(),
                executions,
                artifacts,
                probabilistic_outcomes: Vec::new(),
            });
        }
        if count < PAGE_SIZE {
            break;
        }
    }

    Ok((items, failures))
}

type RunEvidence = BTreeMap<String, (Vec<ExecutionEvidence>, Vec<ArtifactEvidence>)>;

fn collect_runs(
    repository: &str,
    window: &ObservationWindow,
    max_pages: usize,
) -> Result<(RunEvidence, Vec<TransportFailure>)> {
    let mut by_sha: RunEvidence = BTreeMap::new();
    let mut failures = Vec::new();

    for page in 1..=max_pages {
        let response: RunsResponse = gh_api(&format!(
            "repos/{repository}/actions/runs?per_page={PAGE_SIZE}&page={page}"
        ))?;
        if response.workflow_runs.is_empty() {
            break;
        }
        let count = response.workflow_runs.len();
        for run in response.workflow_runs {
            if run.created_at < window.start_utc || run.created_at >= window.end_exclusive_utc {
                continue;
            }
            let result = match (run.status.as_str(), run.conclusion.as_deref()) {
                ("completed", Some("success")) => ExecutionResult::Pass,
                ("completed", Some("cancelled" | "skipped")) => ExecutionResult::Cancelled,
                ("completed", Some(_)) => ExecutionResult::Fail,
                ("queued" | "in_progress", _) => ExecutionResult::Pending,
                _ => ExecutionResult::Unknown,
            };
            let exit_code = match result {
                ExecutionResult::Pass => Some(0),
                ExecutionResult::Fail | ExecutionResult::Cancelled => Some(1),
                ExecutionResult::Pending | ExecutionResult::Unknown => None,
            };
            let execution = ExecutionEvidence {
                surface: "github-actions".to_owned(),
                command: format!("workflow:{}#{}", run.name, run.id),
                source_sha: run.head_sha.clone(),
                result,
                exit_code,
                observed_at_utc: run.updated_at.clone(),
                evidence_url: run.html_url.clone(),
            };
            let artifacts = match collect_artifacts(repository, &run) {
                Ok(artifacts) => artifacts,
                Err(error) => {
                    failures.push(TransportFailure {
                        repository: repository.to_owned(),
                        operation: format!("collect workflow artifacts for run {}", run.id),
                        state: "BLOCKED".to_owned(),
                        detail: error.to_string(),
                    });
                    Vec::new()
                }
            };
            let entry = by_sha.entry(run.head_sha).or_default();
            entry.0.push(execution);
            entry.1.extend(artifacts);
        }
        if count < PAGE_SIZE {
            break;
        }
    }

    for (executions, artifacts) in by_sha.values_mut() {
        executions.sort_by(|left, right| {
            left.command
                .cmp(&right.command)
                .then(left.evidence_url.cmp(&right.evidence_url))
        });
        artifacts.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.evidence_url.cmp(&right.evidence_url))
        });
    }
    Ok((by_sha, failures))
}

fn collect_artifacts(repository: &str, run: &WorkflowRun) -> Result<Vec<ArtifactEvidence>> {
    let response: ArtifactsResponse = gh_api(&format!(
        "repos/{repository}/actions/runs/{}/artifacts?per_page={PAGE_SIZE}",
        run.id
    ))?;
    Ok(response
        .artifacts
        .into_iter()
        .filter(|artifact| !artifact.expired)
        .map(|artifact| ArtifactEvidence {
            name: artifact.name,
            source_sha: run.head_sha.clone(),
            evidence_url: artifact.archive_download_url,
            digest: artifact.digest,
            size_bytes: artifact.size_in_bytes,
        })
        .collect())
}

fn gh_api<T: DeserializeOwned>(endpoint: &str) -> Result<T> {
    let output = Command::new("gh")
        .args(["api", endpoint])
        .output()
        .with_context(|| format!("launching gh api {endpoint}"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "gh api {endpoint} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .with_context(|| format!("decoding gh api response for {endpoint}"))
}

fn validate_repository(repository: &str) -> Result<()> {
    let Some((owner, name)) = repository.split_once('/') else {
        return Err(anyhow!("repository must use owner/name form: {repository}"));
    };
    if owner.is_empty()
        || name.is_empty()
        || !owner.chars().all(valid_repo_char)
        || !name.chars().all(valid_repo_char)
    {
        return Err(anyhow!("invalid repository identity: {repository}"));
    }
    Ok(())
}

fn valid_repo_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.')
}

#[derive(Deserialize)]
struct CommitSummary {
    sha: String,
}

#[derive(Deserialize)]
struct CommitDetail {
    sha: String,
    html_url: String,
    commit: CommitPayload,
    #[serde(default)]
    parents: Vec<Parent>,
    #[serde(default)]
    files: Vec<ChangedFile>,
}

#[derive(Deserialize)]
struct CommitPayload {
    message: String,
    #[serde(default)]
    author: Option<GitIdentity>,
    #[serde(default)]
    committer: Option<GitIdentity>,
}

#[derive(Deserialize)]
struct GitIdentity {
    date: String,
}

#[derive(Deserialize)]
struct Parent {
    sha: String,
}

#[derive(Deserialize)]
struct ChangedFile {
    filename: String,
}

#[derive(Deserialize)]
struct RunsResponse {
    #[serde(default)]
    workflow_runs: Vec<WorkflowRun>,
}

#[derive(Deserialize)]
struct WorkflowRun {
    id: u64,
    name: String,
    head_sha: String,
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    html_url: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct ArtifactsResponse {
    #[serde(default)]
    artifacts: Vec<WorkflowArtifact>,
}

#[derive(Deserialize)]
struct WorkflowArtifact {
    name: String,
    archive_download_url: String,
    expired: bool,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    size_in_bytes: Option<u64>,
}
