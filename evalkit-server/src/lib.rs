use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use evalkit::{CompareConfig, Comparison, RunResult, Sample, compare};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct AppState {
    store: Arc<RunStore>,
}

impl AppState {
    pub fn new(store: RunStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunStore {
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredRun {
    pub result: RunResult,
    #[serde(default)]
    pub samples: Vec<Sample<Value, Value>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RunSummary {
    pub run_id: String,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: chrono::DateTime<Utc>,
    pub acquisition_mode: String,
    pub sample_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateAnnotation {
    pub sample_id: String,
    pub label: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnnotationRecord {
    pub id: i64,
    pub run_id: String,
    pub sample_id: String,
    pub label: String,
    pub note: String,
    pub created_at: chrono::DateTime<Utc>,
    pub promoted_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug)]
pub enum ServerError {
    NotFound(String),
    InvalidRequest(String),
    Store(String),
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(message) | Self::InvalidRequest(message) | Self::Store(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for ServerError {}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::Store(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string()).into_response()
    }
}

impl RunStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ServerError> {
        let store = Self { path: path.into() };
        store.init_schema()?;
        Ok(store)
    }

    pub fn list_runs(&self) -> Result<Vec<RunSummary>, ServerError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT run_id, started_at, completed_at, acquisition_mode, sample_count FROM runs ORDER BY started_at DESC",
            )
            .map_err(store_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(RunSummary {
                    run_id: row.get(0)?,
                    started_at: parse_timestamp(row.get::<_, String>(1)?)?,
                    completed_at: parse_timestamp(row.get::<_, String>(2)?)?,
                    acquisition_mode: row.get(3)?,
                    sample_count: row.get::<_, i64>(4)? as usize,
                })
            })
            .map_err(store_error)?;

        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row.map_err(store_error)?);
        }
        Ok(summaries)
    }

    pub fn store_run(&self, run: &StoredRun) -> Result<(), ServerError> {
        let connection = self.connection()?;
        let run_json = serde_json::to_string(run).map_err(|err| {
            ServerError::InvalidRequest(format!("failed to serialize stored run: {err}"))
        })?;
        let metadata_json = serde_json::to_string(&run.result.metadata).map_err(|err| {
            ServerError::InvalidRequest(format!("failed to serialize run metadata: {err}"))
        })?;

        connection
            .execute(
                "INSERT OR REPLACE INTO runs (run_id, started_at, completed_at, acquisition_mode, sample_count, metadata_json, run_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    run.result.metadata.run_id,
                    run.result.metadata.started_at.to_rfc3339(),
                    run.result.metadata.completed_at.to_rfc3339(),
                    run.result.metadata.acquisition_mode,
                    run.result.samples.len() as i64,
                    metadata_json,
                    run_json,
                    Utc::now().to_rfc3339(),
                ],
            )
            .map_err(store_error)?;

        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<StoredRun>, ServerError> {
        let connection = self.connection()?;
        let run_json = connection
            .query_row(
                "SELECT run_json FROM runs WHERE run_id = ?1",
                [run_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(store_error)?;

        run_json
            .map(|json| {
                serde_json::from_str(&json).map_err(|err| {
                    ServerError::Store(format!("failed to deserialize stored run `{run_id}`: {err}"))
                })
            })
            .transpose()
    }

    pub fn diff_runs(&self, left: &str, right: &str) -> Result<Comparison, ServerError> {
        let left_run = self
            .get_run(left)?
            .ok_or_else(|| ServerError::NotFound(format!("run `{left}` not found")))?;
        let right_run = self
            .get_run(right)?
            .ok_or_else(|| ServerError::NotFound(format!("run `{right}` not found")))?;

        Ok(compare(
            &left_run.result,
            &right_run.result,
            CompareConfig::default(),
        ))
    }

    pub fn create_annotation(
        &self,
        run_id: &str,
        annotation: &CreateAnnotation,
    ) -> Result<AnnotationRecord, ServerError> {
        if self.get_run(run_id)?.is_none() {
            return Err(ServerError::NotFound(format!("run `{run_id}` not found")));
        }

        let connection = self.connection()?;
        let created_at = Utc::now();
        connection
            .execute(
                "INSERT INTO annotations (run_id, sample_id, label, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![run_id, annotation.sample_id, annotation.label, annotation.note, created_at.to_rfc3339()],
            )
            .map_err(store_error)?;

        let id = connection.last_insert_rowid();
        Ok(AnnotationRecord {
            id,
            run_id: run_id.to_string(),
            sample_id: annotation.sample_id.clone(),
            label: annotation.label.clone(),
            note: annotation.note.clone(),
            created_at,
            promoted_at: None,
        })
    }

    pub fn list_annotations(&self, run_id: &str) -> Result<Vec<AnnotationRecord>, ServerError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, run_id, sample_id, label, note, created_at, promoted_at FROM annotations WHERE run_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(store_error)?;
        let rows = statement
            .query_map([run_id], |row| {
                Ok(AnnotationRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    sample_id: row.get(2)?,
                    label: row.get(3)?,
                    note: row.get(4)?,
                    created_at: parse_timestamp(row.get::<_, String>(5)?)?,
                    promoted_at: row
                        .get::<_, Option<String>>(6)?
                        .map(parse_timestamp)
                        .transpose()?,
                })
            })
            .map_err(store_error)?;

        let mut annotations = Vec::new();
        for row in rows {
            annotations.push(row.map_err(store_error)?);
        }

        Ok(annotations)
    }

    fn init_schema(&self) -> Result<(), ServerError> {
        let connection = self.connection()?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS runs (
                    run_id TEXT PRIMARY KEY,
                    started_at TEXT NOT NULL,
                    completed_at TEXT NOT NULL,
                    acquisition_mode TEXT NOT NULL,
                    sample_count INTEGER NOT NULL,
                    metadata_json TEXT NOT NULL,
                    run_json TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS annotations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    run_id TEXT NOT NULL,
                    sample_id TEXT NOT NULL,
                    label TEXT NOT NULL,
                    note TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    promoted_at TEXT,
                    FOREIGN KEY(run_id) REFERENCES runs(run_id)
                );",
            )
            .map_err(store_error)
    }

    fn connection(&self) -> Result<Connection, ServerError> {
        Connection::open(&self.path).map_err(store_error)
    }
}

pub fn router(store: RunStore) -> Router {
    let state = AppState::new(store);

    Router::new()
        .route("/", get(home_page))
        .route("/runs/:run_id", get(run_detail_page))
        .route("/runs/:left/diff/:right", get(diff_page))
        .route("/healthz", get(healthz))
        .route("/api/runs", get(list_runs).post(create_run))
        .route("/api/runs/:run_id", get(get_run))
        .route("/api/runs/:run_id/annotations", get(list_annotations).post(create_annotation))
        .route("/api/runs/:left/diff/:right", get(diff_runs))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn home_page(State(state): State<AppState>) -> Result<Html<String>, ServerError> {
    Ok(Html(render_home_page(&state.store.list_runs()?)))
}

async fn list_runs(State(state): State<AppState>) -> Result<Json<Vec<RunSummary>>, ServerError> {
    Ok(Json(state.store.list_runs()?))
}

async fn create_run(
    State(state): State<AppState>,
    Json(run): Json<StoredRun>,
) -> Result<StatusCode, ServerError> {
    state.store.store_run(&run)?;
    Ok(StatusCode::CREATED)
}

async fn get_run(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Result<Json<StoredRun>, ServerError> {
    let run = state
        .store
        .get_run(&run_id)?
        .ok_or_else(|| ServerError::NotFound(format!("run `{run_id}` not found")))?;
    Ok(Json(run))
}

async fn diff_runs(
    State(state): State<AppState>,
    AxumPath((left, right)): AxumPath<(String, String)>,
) -> Result<Json<Comparison>, ServerError> {
    Ok(Json(state.store.diff_runs(&left, &right)?))
}

async fn create_annotation(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(annotation): Json<CreateAnnotation>,
) -> Result<(StatusCode, Json<AnnotationRecord>), ServerError> {
    let created = state.store.create_annotation(&run_id, &annotation)?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn list_annotations(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Result<Json<Vec<AnnotationRecord>>, ServerError> {
    Ok(Json(state.store.list_annotations(&run_id)?))
}

async fn run_detail_page(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Result<Html<String>, ServerError> {
    let run = state
        .store
        .get_run(&run_id)?
        .ok_or_else(|| ServerError::NotFound(format!("run `{run_id}` not found")))?;
    let annotations = state.store.list_annotations(&run_id)?;

    Ok(Html(render_run_detail_page(&run, &annotations)))
}

async fn diff_page(
    State(state): State<AppState>,
    AxumPath((left, right)): AxumPath<(String, String)>,
) -> Result<Html<String>, ServerError> {
    let comparison = state.store.diff_runs(&left, &right)?;
    Ok(Html(render_diff_page(&comparison)))
}

fn parse_timestamp(value: String) -> Result<chrono::DateTime<Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })
}

fn store_error(error: impl Display) -> ServerError {
    ServerError::Store(error.to_string())
}

fn render_home_page(runs: &[RunSummary]) -> String {
    let mut html = page_shell("Runs", String::from("<h1>Runs</h1><p>Browse stored eval runs and compare recent outputs.</p>"));
    html.push_str("<div class=\"runs\">");

    for (index, run) in runs.iter().enumerate() {
        html.push_str("<article class=\"card\">");
        html.push_str(&format!(
            "<h2><a href=\"/runs/{id}\">{id}</a></h2><p><strong>Samples:</strong> {samples}<br><strong>Mode:</strong> {mode}<br><strong>Started:</strong> {started}</p>",
            id = run.run_id,
            samples = run.sample_count,
            mode = escape_html(&run.acquisition_mode),
            started = run.started_at,
        ));
        if let Some(next) = runs.get(index + 1) {
            html.push_str(&format!(
                "<p><a href=\"/runs/{left}/diff/{right}\">Compare against {right}</a></p>",
                left = run.run_id,
                right = next.run_id,
            ));
        }
        html.push_str("</article>");
    }

    html.push_str("</div></body></html>");
    html
}

fn render_run_detail_page(run: &StoredRun, annotations: &[AnnotationRecord]) -> String {
    let mut html = page_shell(
        &format!("Run {}", run.result.metadata.run_id),
        format!(
            "<h1>Run {}</h1><p><strong>Acquisition:</strong> {}<br><strong>Samples:</strong> {}<br><strong>Started:</strong> {}</p>",
            run.result.metadata.run_id,
            escape_html(&run.result.metadata.acquisition_mode),
            run.result.samples.len(),
            run.result.metadata.started_at,
        ),
    );
    html.push_str("<section><h2>Samples</h2>");

    for sample in &run.result.samples {
        let status = if sample_has_failure(sample) { "sample-failed" } else { "sample-ok" };
        html.push_str(&format!(
            "<article class=\"card {status}\"><h3>{id}</h3><p><strong>Trials:</strong> {trials} · <strong>Scored:</strong> {scored} · <strong>Errors:</strong> {errors}</p>",
            id = escape_html(&sample.sample_id),
            trials = sample.trial_count,
            scored = sample.scored_count,
            errors = sample.error_count,
        ));
        html.push_str("<ul>");
        for trial in &sample.trials {
            for (name, score) in &trial.scores {
                html.push_str(&format!(
                    "<li><strong>{}</strong>: {}</li>",
                    escape_html(name),
                    escape_html(&format_score(score))
                ));
            }
        }
        html.push_str("</ul></article>");
    }

    html.push_str("</section><section><h2>Annotations</h2>");
    if annotations.is_empty() {
        html.push_str("<p>No annotations yet.</p>");
    } else {
        html.push_str("<ul>");
        for annotation in annotations {
            html.push_str(&format!(
                "<li><strong>{}</strong> on {}: {}</li>",
                escape_html(&annotation.label),
                escape_html(&annotation.sample_id),
                escape_html(&annotation.note),
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("</section></body></html>");
    html
}

fn render_diff_page(comparison: &Comparison) -> String {
    let mut html = page_shell(
        "Run Diff",
        format!(
            "<h1>Diff</h1><p><strong>Baseline:</strong> {}<br><strong>Candidate:</strong> {}</p>",
            escape_html(&comparison.baseline_id),
            escape_html(&comparison.candidate_id),
        ),
    );
    html.push_str("<section><h2>Shared scorers</h2>");

    for (name, scorer) in &comparison.shared_scorers {
        html.push_str(&format!(
            "<article class=\"card\"><h3>{}</h3><p><strong>Aggregate delta:</strong> {:.4}</p><ul>",
            escape_html(name),
            scorer.aggregate_delta,
        ));
        for sample in scorer.sample_comparisons.values() {
            html.push_str(&format!(
                "<li>{}: {:?} ({:.4})</li>",
                escape_html(&sample.sample_id),
                sample.direction,
                sample.delta,
            ));
        }
        html.push_str("</ul></article>");
    }

    html.push_str("</section></body></html>");
    html
}

fn page_shell(title: &str, body: String) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><style>body{{font-family:Inter,system-ui,sans-serif;background:#0f172a;color:#e2e8f0;margin:0;padding:24px}}a{{color:#7dd3fc}}.runs{{display:grid;gap:16px}}.card{{background:#111827;border:1px solid #334155;border-radius:16px;padding:16px;margin:12px 0}}.sample-failed{{border-color:#f97316}}.sample-ok{{border-color:#22c55e}}ul{{padding-left:20px}}</style></head><body>{}",
        escape_html(title),
        body,
    )
}

fn sample_has_failure(sample: &evalkit::SampleResult) -> bool {
    sample.trials.iter().any(|trial| {
        trial.scores.values().any(|score| match score {
            Ok(evalkit::Score::Binary(value)) => !value,
            Ok(_) => false,
            Err(_) => true,
        })
    })
}

fn format_score(score: &Result<evalkit::Score, evalkit::ScorerError>) -> String {
    match score {
        Ok(value) => format!("{value:?}"),
        Err(error) => format!("error: {error}"),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::{CreateAnnotation, RunStore, StoredRun};
    use chrono::{Duration, Utc};
    use evalkit::{Direction, RunMetadata, RunResult, Sample, SampleResult, Score, ScoreDefinition, TrialResult};
    use serde_json::json;
    use tempfile::tempdir;

    fn stored_run_fixture(run_id: &str, sample_id: &str) -> StoredRun {
        StoredRun {
            result: RunResult {
                metadata: RunMetadata {
                    run_id: run_id.to_string(),
                    seed: Some(7),
                    dataset_fingerprint: String::from("dataset"),
                    scorer_fingerprint: String::from("scorers"),
                    code_commit: Some(String::from("abc123")),
                    code_fingerprint: Some(String::from("tree:abc123")),
                    judge_model_pins: vec![String::from("mock/model")],
                    started_at: Utc::now(),
                    completed_at: Utc::now() + Duration::seconds(1),
                    duration: std::time::Duration::from_secs(1),
                    trial_count: 1,
                    score_definitions: vec![ScoreDefinition {
                        name: String::from("exact_match"),
                        direction: Some(Direction::Maximize),
                    }],
                    acquisition_mode: String::from("inline"),
                },
                samples: vec![SampleResult {
                    sample_id: sample_id.to_string(),
                    trial_count: 1,
                    scored_count: 1,
                    error_count: 0,
                    trials: vec![TrialResult {
                        trial_index: 0,
                        duration: std::time::Duration::from_millis(10),
                        scores: std::collections::HashMap::from([(
                            String::from("exact_match"),
                            Ok(Score::Binary(true)),
                        )]),
                    }],
                    token_usage: Default::default(),
                    cost_usd: None,
                }],
            },
            samples: vec![Sample::builder(json!({ "prompt": "hello" }))
                .id(sample_id)
                .reference(json!("echo::hello"))
                .build()
                .unwrap()],
        }
    }

    #[test]
    fn run_store_round_trips_runs_diffs_and_annotations() {
        let temp = tempdir().unwrap();
        let store = RunStore::open(temp.path().join("runs.sqlite")).unwrap();
        let first = stored_run_fixture("run-a", "sample-a");
        let mut second = stored_run_fixture("run-b", "sample-a");
        second.result.samples[0].trials[0]
            .scores
            .insert(String::from("exact_match"), Ok(Score::Binary(false)));

        store.store_run(&first).unwrap();
        store.store_run(&second).unwrap();

        let runs = store.list_runs().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(store.get_run("run-a").unwrap().unwrap().result.metadata.run_id, "run-a");

        let diff = store.diff_runs("run-a", "run-b").unwrap();
        assert!(diff.shared_scorers.contains_key("exact_match"));

        let annotation = store
            .create_annotation(
                "run-a",
                &CreateAnnotation {
                    sample_id: String::from("sample-a"),
                    label: String::from("needs_review"),
                    note: String::from("wrong answer"),
                },
            )
            .unwrap();
        assert_eq!(annotation.label, "needs_review");
        assert_eq!(store.list_annotations("run-a").unwrap().len(), 1);
    }
}
