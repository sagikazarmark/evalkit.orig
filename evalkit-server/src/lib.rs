use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Form, Json, Router};
use chrono::Utc;
use evalkit::{CompareConfig, Comparison, RunResult, Sample, ScorerStats, compare};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromoteAnnotationsRequest {
    pub output_path: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PromotionResult {
    pub exported_count: usize,
    pub output_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AlertRule {
    pub id: i64,
    pub name: String,
    pub scorer_name: String,
    pub min_value: f64,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateAlertRule {
    pub name: String,
    pub scorer_name: String,
    pub min_value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AlertStatus {
    pub rule: AlertRule,
    pub run_id: String,
    pub observed_value: Option<f64>,
    pub triggered: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DriftMeasurement {
    Numeric {
        observed_mean: f64,
        baseline_mean: f64,
        delta: f64,
    },
    Binary {
        observed_pass_rate: f64,
        baseline_pass_rate: f64,
        delta: f64,
    },
    Metric {
        observed_mean: f64,
        baseline_mean: f64,
        delta: f64,
    },
    Label {
        observed_mode: String,
        baseline_mode: String,
        distance: f64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DriftStatus {
    pub run_id: String,
    pub scorer_name: String,
    pub baseline_run_ids: Vec<String>,
    pub measurement: DriftMeasurement,
    pub threshold: f64,
    pub triggered: bool,
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
                    ServerError::Store(format!(
                        "failed to deserialize stored run `{run_id}`: {err}"
                    ))
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

    pub fn promote_annotations(
        &self,
        run_id: &str,
        request: &PromoteAnnotationsRequest,
    ) -> Result<PromotionResult, ServerError> {
        let run = self
            .get_run(run_id)?
            .ok_or_else(|| ServerError::NotFound(format!("run `{run_id}` not found")))?;
        let annotations = self.list_annotations(run_id)?;
        let selected = annotations
            .into_iter()
            .filter(|annotation| {
                request
                    .label
                    .as_ref()
                    .is_none_or(|label| label == &annotation.label)
            })
            .collect::<Vec<_>>();

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&request.output_path)
            .map_err(store_error)?;
        let mut exported_ids = Vec::new();
        let sample_by_id = run
            .samples
            .iter()
            .map(|sample| (sample.id.clone(), sample))
            .collect::<std::collections::HashMap<_, _>>();

        for annotation in &selected {
            let Some(sample) = sample_by_id.get(&annotation.sample_id) else {
                continue;
            };

            let mut promoted = (*sample).clone();
            promoted.metadata.insert(
                String::from("annotation"),
                serde_json::json!({
                    "run_id": annotation.run_id,
                    "label": annotation.label,
                    "note": annotation.note,
                    "annotation_id": annotation.id,
                }),
            );
            serde_json::to_writer(&mut file, &promoted).map_err(|err| {
                ServerError::Store(format!("failed to serialize promoted sample: {err}"))
            })?;
            file.write_all(b"\n").map_err(store_error)?;
            exported_ids.push(annotation.id);
        }

        if !exported_ids.is_empty() {
            let connection = self.connection()?;
            let promoted_at = Utc::now().to_rfc3339();
            for annotation_id in &exported_ids {
                connection
                    .execute(
                        "UPDATE annotations SET promoted_at = ?1 WHERE id = ?2",
                        params![promoted_at, annotation_id],
                    )
                    .map_err(store_error)?;
            }
        }

        Ok(PromotionResult {
            exported_count: exported_ids.len(),
            output_path: request.output_path.clone(),
        })
    }

    pub fn create_alert_rule(&self, rule: &CreateAlertRule) -> Result<AlertRule, ServerError> {
        let connection = self.connection()?;
        let created_at = Utc::now();
        connection
            .execute(
                "INSERT INTO alert_rules (name, scorer_name, min_value, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![rule.name, rule.scorer_name, rule.min_value, created_at.to_rfc3339()],
            )
            .map_err(store_error)?;

        Ok(AlertRule {
            id: connection.last_insert_rowid(),
            name: rule.name.clone(),
            scorer_name: rule.scorer_name.clone(),
            min_value: rule.min_value,
            created_at,
        })
    }

    pub fn list_alert_rules(&self) -> Result<Vec<AlertRule>, ServerError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, scorer_name, min_value, created_at FROM alert_rules ORDER BY created_at ASC",
            )
            .map_err(store_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(AlertRule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    scorer_name: row.get(2)?,
                    min_value: row.get(3)?,
                    created_at: parse_timestamp(row.get::<_, String>(4)?)?,
                })
            })
            .map_err(store_error)?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(row.map_err(store_error)?);
        }
        Ok(rules)
    }

    pub fn evaluate_alerts(&self, run_id: &str) -> Result<Vec<AlertStatus>, ServerError> {
        let run = self
            .get_run(run_id)?
            .ok_or_else(|| ServerError::NotFound(format!("run `{run_id}` not found")))?;
        let rules = self.list_alert_rules()?;

        Ok(rules
            .into_iter()
            .map(|rule| {
                let observed_value = average_score(&run.result, &rule.scorer_name);
                AlertStatus {
                    rule: rule.clone(),
                    run_id: run_id.to_string(),
                    triggered: observed_value.is_some_and(|value| value < rule.min_value),
                    observed_value,
                }
            })
            .collect())
    }

    pub fn detect_drift(
        &self,
        run_id: &str,
        baseline_window: usize,
    ) -> Result<Vec<DriftStatus>, ServerError> {
        let run = self
            .get_run(run_id)?
            .ok_or_else(|| ServerError::NotFound(format!("run `{run_id}` not found")))?;
        let runs = self.list_runs()?;
        let target_index = runs
            .iter()
            .position(|summary| summary.run_id == run_id)
            .ok_or_else(|| ServerError::NotFound(format!("run `{run_id}` not found")))?;
        let baseline_summaries = runs
            .iter()
            .skip(target_index + 1)
            .take(baseline_window)
            .cloned()
            .collect::<Vec<_>>();

        if baseline_summaries.is_empty() {
            return Ok(Vec::new());
        }

        let target_stats = run.result.stats();
        let mut baseline_runs = Vec::new();
        for summary in baseline_summaries {
            let Some(run) = self.get_run(&summary.run_id)? else {
                continue;
            };
            baseline_runs.push((summary.run_id, run.result.stats()));
        }

        let mut scorer_names = target_stats
            .scorer_stats
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        scorer_names.sort();

        let mut drifts = Vec::new();
        for scorer_name in scorer_names {
            let Some(target_stat) = target_stats.scorer_stats.get(&scorer_name) else {
                continue;
            };

            let mut baseline_run_ids = Vec::new();
            let mut baseline_stats = Vec::new();
            for (baseline_run_id, baseline_stats_for_run) in &baseline_runs {
                let Some(baseline_stat) = baseline_stats_for_run.scorer_stats.get(&scorer_name)
                else {
                    continue;
                };
                baseline_run_ids.push(baseline_run_id.clone());
                baseline_stats.push(baseline_stat);
            }

            let Some(drift) = scorer_drift_status(
                run_id,
                &scorer_name,
                target_stat,
                &baseline_run_ids,
                &baseline_stats,
            ) else {
                continue;
            };
            drifts.push(drift);
        }

        Ok(drifts)
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
                );
                CREATE TABLE IF NOT EXISTS alert_rules (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    scorer_name TEXT NOT NULL,
                    min_value REAL NOT NULL,
                    created_at TEXT NOT NULL
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
        .route("/dashboard", get(dashboard_page))
        .route("/runs/:run_id", get(run_detail_page))
        .route("/runs/:left/diff/:right", get(diff_page))
        .route(
            "/runs/:run_id/annotations",
            axum::routing::post(create_annotation_form),
        )
        .route(
            "/runs/:run_id/promote",
            axum::routing::post(promote_annotations_form),
        )
        .route("/alert-rules", axum::routing::post(create_alert_rule_form))
        .route("/healthz", get(healthz))
        .route("/api/runs", get(list_runs).post(create_run))
        .route("/api/runs/:run_id", get(get_run))
        .route(
            "/api/runs/:run_id/annotations",
            get(list_annotations).post(create_annotation),
        )
        .route(
            "/api/runs/:run_id/promote",
            axum::routing::post(promote_annotations),
        )
        .route("/api/runs/:run_id/alerts", get(get_alerts))
        .route("/api/runs/:run_id/drift", get(get_drift))
        .route(
            "/api/alert-rules",
            get(list_alert_rules).post(create_alert_rule),
        )
        .route("/api/runs/:left/diff/:right", get(diff_runs))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn home_page(State(state): State<AppState>) -> Result<Html<String>, ServerError> {
    Ok(Html(render_home_page(&state.store.list_runs()?)))
}

async fn dashboard_page(State(state): State<AppState>) -> Result<Html<String>, ServerError> {
    let runs = state.store.list_runs()?;
    let rules = state.store.list_alert_rules()?;
    let mut alerts = Vec::new();
    let mut drifts = Vec::new();
    for run in runs.iter().take(5) {
        alerts.extend(state.store.evaluate_alerts(&run.run_id)?);
        drifts.extend(state.store.detect_drift(&run.run_id, 5)?);
    }

    Ok(Html(render_dashboard_page(&runs, &rules, &alerts, &drifts)))
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

async fn promote_annotations(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(request): Json<PromoteAnnotationsRequest>,
) -> Result<Json<PromotionResult>, ServerError> {
    Ok(Json(state.store.promote_annotations(&run_id, &request)?))
}

async fn create_annotation_form(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Form(annotation): Form<CreateAnnotation>,
) -> Result<Redirect, ServerError> {
    state.store.create_annotation(&run_id, &annotation)?;
    Ok(Redirect::to(&format!("/runs/{run_id}")))
}

async fn promote_annotations_form(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Form(request): Form<PromoteAnnotationsRequest>,
) -> Result<Redirect, ServerError> {
    state.store.promote_annotations(&run_id, &request)?;
    Ok(Redirect::to(&format!("/runs/{run_id}")))
}

async fn create_alert_rule_form(
    State(state): State<AppState>,
    Form(rule): Form<CreateAlertRule>,
) -> Result<Redirect, ServerError> {
    state.store.create_alert_rule(&rule)?;
    Ok(Redirect::to("/dashboard"))
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

async fn create_alert_rule(
    State(state): State<AppState>,
    Json(rule): Json<CreateAlertRule>,
) -> Result<(StatusCode, Json<AlertRule>), ServerError> {
    Ok((
        StatusCode::CREATED,
        Json(state.store.create_alert_rule(&rule)?),
    ))
}

async fn list_alert_rules(
    State(state): State<AppState>,
) -> Result<Json<Vec<AlertRule>>, ServerError> {
    Ok(Json(state.store.list_alert_rules()?))
}

async fn get_alerts(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Result<Json<Vec<AlertStatus>>, ServerError> {
    Ok(Json(state.store.evaluate_alerts(&run_id)?))
}

async fn get_drift(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Result<Json<Vec<DriftStatus>>, ServerError> {
    Ok(Json(state.store.detect_drift(&run_id, 5)?))
}

fn parse_timestamp(value: String) -> Result<chrono::DateTime<Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })
}

fn store_error(error: impl Display) -> ServerError {
    ServerError::Store(error.to_string())
}

fn render_home_page(runs: &[RunSummary]) -> String {
    let mut html = page_shell(
        "Runs",
        String::from("<h1>Runs</h1><p>Browse stored eval runs and compare recent outputs.</p>"),
    );
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
        let status = if sample_has_failure(sample) {
            "sample-failed"
        } else {
            "sample-ok"
        };
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
        html.push_str(&format!(
            "</ul><form method=\"post\" action=\"/runs/{run_id}/annotations\"><input type=\"hidden\" name=\"sample_id\" value=\"{sample_id}\"><label>Label <input name=\"label\" required></label><br><label>Note <textarea name=\"note\"></textarea></label><br><button type=\"submit\">Save annotation</button></form>",
            run_id = run.result.metadata.run_id,
            sample_id = escape_html(&sample.sample_id),
        ));
        html.push_str("</ul></article>");
    }

    html.push_str("</section><section><h2>Annotations</h2>");
    if annotations.is_empty() {
        html.push_str("<p>No annotations yet.</p>");
    } else {
        html.push_str("<ul>");
        for annotation in annotations {
            html.push_str(&format!(
                "<li><strong>{}</strong> on {}: {}{}</li>",
                escape_html(&annotation.label),
                escape_html(&annotation.sample_id),
                escape_html(&annotation.note),
                annotation
                    .promoted_at
                    .as_ref()
                    .map(|timestamp| format!(" <em>(promoted {})</em>", timestamp))
                    .unwrap_or_default(),
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str(&format!(
        "<form method=\"post\" action=\"/runs/{}/promote\"><label>Output path <input name=\"output_path\" value=\"annotations.jsonl\" required></label><br><label>Filter label <input name=\"label\"></label><br><button type=\"submit\">Promote annotations</button></form>",
        run.result.metadata.run_id
    ));
    html.push_str("</section></body></html>");
    html
}

fn render_dashboard_page(
    runs: &[RunSummary],
    rules: &[AlertRule],
    alerts: &[AlertStatus],
    drifts: &[DriftStatus],
) -> String {
    let mut html = page_shell(
        "Dashboard",
        String::from(
            "<h1>Prod-Eval Dashboard</h1><p>Recent runs, alert rules, threshold breaches, and drift checks.</p>",
        ),
    );
    html.push_str("<section><h2>Alert rules</h2><form method=\"post\" action=\"/alert-rules\"><label>Name <input name=\"name\" required></label><br><label>Scorer <input name=\"scorer_name\" required></label><br><label>Minimum value <input name=\"min_value\" type=\"number\" step=\"0.01\" required></label><br><button type=\"submit\">Create rule</button></form>");
    if rules.is_empty() {
        html.push_str("<p>No alert rules configured.</p>");
    } else {
        html.push_str("<ul>");
        for rule in rules {
            html.push_str(&format!(
                "<li><strong>{}</strong> on {} below {:.3}</li>",
                escape_html(&rule.name),
                escape_html(&rule.scorer_name),
                rule.min_value,
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("</section><section><h2>Recent runs</h2><ul>");
    for run in runs.iter().take(5) {
        html.push_str(&format!(
            "<li><a href=\"/runs/{}\">{}</a> · {} samples · <a href=\"/api/runs/{}/alerts\">alert json</a> · <a href=\"/api/runs/{}/drift\">drift json</a></li>",
            run.run_id,
            run.run_id,
            run.sample_count,
            run.run_id,
            run.run_id,
        ));
    }
    html.push_str("</ul></section><section><h2>Triggered alerts</h2>");
    let triggered = alerts
        .iter()
        .filter(|alert| alert.triggered)
        .collect::<Vec<_>>();
    if triggered.is_empty() {
        html.push_str("<p>No active alerts.</p>");
    } else {
        html.push_str("<ul>");
        for alert in triggered {
            html.push_str(&format!(
                "<li><strong>{}</strong> on run {}: observed {:.3}, threshold {:.3}</li>",
                escape_html(&alert.rule.name),
                escape_html(&alert.run_id),
                alert.observed_value.unwrap_or_default(),
                alert.rule.min_value,
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("</section><section><h2>Triggered drift</h2>");
    let triggered_drifts = drifts
        .iter()
        .filter(|drift| drift.triggered)
        .collect::<Vec<_>>();
    if triggered_drifts.is_empty() {
        html.push_str("<p>No drift detected across recent runs.</p>");
    } else {
        html.push_str("<ul>");
        for drift in triggered_drifts {
            html.push_str(&format!(
                "<li><strong>{}</strong> on run {} against {}: {}</li>",
                escape_html(&drift.scorer_name),
                escape_html(&drift.run_id),
                escape_html(&drift.baseline_run_ids.join(", ")),
                escape_html(&format_drift_measurement(drift)),
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

fn format_drift_measurement(drift: &DriftStatus) -> String {
    match &drift.measurement {
        DriftMeasurement::Numeric {
            observed_mean,
            baseline_mean,
            delta,
        }
        | DriftMeasurement::Metric {
            observed_mean,
            baseline_mean,
            delta,
        } => format!(
            "observed {:.3}, baseline {:.3}, delta {:.3}, threshold {:.3}",
            observed_mean, baseline_mean, delta, drift.threshold,
        ),
        DriftMeasurement::Binary {
            observed_pass_rate,
            baseline_pass_rate,
            delta,
        } => format!(
            "observed {:.1}%, baseline {:.1}%, delta {:.1} pts, threshold {:.1} pts",
            observed_pass_rate * 100.0,
            baseline_pass_rate * 100.0,
            delta * 100.0,
            drift.threshold * 100.0,
        ),
        DriftMeasurement::Label {
            observed_mode,
            baseline_mode,
            distance,
        } => format!(
            "mode {} vs {}, distance {:.3}, threshold {:.3}",
            observed_mode, baseline_mode, distance, drift.threshold,
        ),
    }
}

fn average_score(run: &RunResult, scorer_name: &str) -> Option<f64> {
    let mut values = Vec::new();

    for sample in &run.samples {
        for trial in &sample.trials {
            let Some(score) = trial.scores.get(scorer_name) else {
                continue;
            };
            if let Some(value) = score_value(score) {
                values.push(value);
            }
        }
    }

    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn score_value(score: &Result<evalkit::Score, evalkit::ScorerError>) -> Option<f64> {
    match score {
        Ok(evalkit::Score::Binary(value)) => Some(if *value { 1.0 } else { 0.0 }),
        Ok(evalkit::Score::Numeric(value)) => Some(*value),
        Ok(evalkit::Score::Structured { score, .. }) => Some(*score),
        Ok(evalkit::Score::Metric { value, .. }) => Some(*value),
        Ok(evalkit::Score::Label(_)) | Ok(_) | Err(_) => None,
    }
}

fn scorer_drift_status(
    run_id: &str,
    scorer_name: &str,
    target: &ScorerStats,
    baseline_run_ids: &[String],
    baselines: &[&ScorerStats],
) -> Option<DriftStatus> {
    if baseline_run_ids.is_empty() || baselines.is_empty() {
        return None;
    }

    match target {
        ScorerStats::Numeric { mean, .. } => {
            let baseline_values = baselines
                .iter()
                .filter_map(|baseline| match baseline {
                    ScorerStats::Numeric { mean, .. } => Some(*mean),
                    _ => None,
                })
                .collect::<Vec<_>>();
            numeric_drift_status(
                run_id,
                scorer_name,
                baseline_run_ids,
                *mean,
                &baseline_values,
                |observed_mean, baseline_mean, delta| DriftMeasurement::Numeric {
                    observed_mean,
                    baseline_mean,
                    delta,
                },
            )
        }
        ScorerStats::Binary { pass_rate, .. } => {
            let baseline_values = baselines
                .iter()
                .filter_map(|baseline| match baseline {
                    ScorerStats::Binary { pass_rate, .. } => Some(*pass_rate),
                    _ => None,
                })
                .collect::<Vec<_>>();
            numeric_drift_status(
                run_id,
                scorer_name,
                baseline_run_ids,
                *pass_rate,
                &baseline_values,
                |observed_pass_rate, baseline_pass_rate, delta| DriftMeasurement::Binary {
                    observed_pass_rate,
                    baseline_pass_rate,
                    delta,
                },
            )
        }
        ScorerStats::Metric { mean, .. } => {
            let baseline_values = baselines
                .iter()
                .filter_map(|baseline| match baseline {
                    ScorerStats::Metric { mean, .. } => Some(*mean),
                    _ => None,
                })
                .collect::<Vec<_>>();
            numeric_drift_status(
                run_id,
                scorer_name,
                baseline_run_ids,
                *mean,
                &baseline_values,
                |observed_mean, baseline_mean, delta| DriftMeasurement::Metric {
                    observed_mean,
                    baseline_mean,
                    delta,
                },
            )
        }
        ScorerStats::Label { distribution, mode } => label_drift_status(
            run_id,
            scorer_name,
            baseline_run_ids,
            distribution,
            mode,
            baselines,
        ),
    }
}

fn numeric_drift_status(
    run_id: &str,
    scorer_name: &str,
    baseline_run_ids: &[String],
    observed_value: f64,
    baseline_values: &[f64],
    measurement: impl Fn(f64, f64, f64) -> DriftMeasurement,
) -> Option<DriftStatus> {
    if baseline_values.is_empty() {
        return None;
    }

    let baseline_mean = mean(baseline_values);
    let threshold =
        (sample_stddev(baseline_values) * 2.0).max((baseline_mean.abs() * 0.2).max(0.1));
    let delta = observed_value - baseline_mean;

    Some(DriftStatus {
        run_id: run_id.to_string(),
        scorer_name: scorer_name.to_string(),
        baseline_run_ids: baseline_run_ids.to_vec(),
        measurement: measurement(observed_value, baseline_mean, delta),
        threshold,
        triggered: delta.abs() >= threshold,
    })
}

fn label_drift_status(
    run_id: &str,
    scorer_name: &str,
    baseline_run_ids: &[String],
    observed_distribution: &HashMap<String, usize>,
    observed_mode: &str,
    baselines: &[&ScorerStats],
) -> Option<DriftStatus> {
    let baseline_distributions = baselines
        .iter()
        .filter_map(|baseline| match baseline {
            ScorerStats::Label { distribution, .. } => {
                Some(distribution_probability_map(distribution))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if baseline_distributions.is_empty() {
        return None;
    }

    let observed_distribution = distribution_probability_map(observed_distribution);
    let baseline_distribution = average_distribution(&baseline_distributions);
    let threshold = 0.3;
    let distance = total_variation_distance(&observed_distribution, &baseline_distribution);

    Some(DriftStatus {
        run_id: run_id.to_string(),
        scorer_name: scorer_name.to_string(),
        baseline_run_ids: baseline_run_ids.to_vec(),
        measurement: DriftMeasurement::Label {
            observed_mode: observed_mode.to_string(),
            baseline_mode: dominant_label(&baseline_distribution).unwrap_or_default(),
            distance,
        },
        threshold,
        triggered: distance >= threshold,
    })
}

fn distribution_probability_map(distribution: &HashMap<String, usize>) -> HashMap<String, f64> {
    let total = distribution.values().sum::<usize>() as f64;
    if total <= f64::EPSILON {
        return HashMap::new();
    }

    distribution
        .iter()
        .map(|(label, count)| (label.clone(), *count as f64 / total))
        .collect()
}

fn average_distribution(distributions: &[HashMap<String, f64>]) -> HashMap<String, f64> {
    let mut averaged = HashMap::<String, f64>::new();
    for distribution in distributions {
        for (label, probability) in distribution {
            *averaged.entry(label.clone()).or_insert(0.0) += *probability;
        }
    }

    for probability in averaged.values_mut() {
        *probability /= distributions.len() as f64;
    }

    averaged
}

fn total_variation_distance(left: &HashMap<String, f64>, right: &HashMap<String, f64>) -> f64 {
    let mut labels = BTreeSet::new();
    labels.extend(left.keys().cloned());
    labels.extend(right.keys().cloned());

    labels
        .into_iter()
        .map(|label| {
            let left_value = left.get(&label).copied().unwrap_or_default();
            let right_value = right.get(&label).copied().unwrap_or_default();
            (left_value - right_value).abs()
        })
        .sum::<f64>()
        / 2.0
}

fn dominant_label(distribution: &HashMap<String, f64>) -> Option<String> {
    distribution
        .iter()
        .max_by(|(left_label, left_value), (right_label, right_value)| {
            left_value
                .total_cmp(right_value)
                .then_with(|| right_label.cmp(left_label))
        })
        .map(|(label, _)| label.clone())
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_stddev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let mean = mean(values);
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() as f64 - 1.0);
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::{
        CreateAlertRule, CreateAnnotation, DriftMeasurement, PromoteAnnotationsRequest, RunStore,
        StoredRun,
    };
    use chrono::{Duration, Utc};
    use evalkit::{
        Direction, RunMetadata, RunResult, Sample, SampleResult, Score, ScoreDefinition,
        TrialResult,
    };
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
            samples: vec![
                Sample::builder(json!({ "prompt": "hello" }))
                    .id(sample_id)
                    .reference(json!("echo::hello"))
                    .build()
                    .unwrap(),
            ],
        }
    }

    fn stored_binary_run_fixture(run_id: &str, sample_id: &str, score: bool) -> StoredRun {
        let mut run = stored_run_fixture(run_id, sample_id);
        run.result.samples[0].trials[0]
            .scores
            .insert(String::from("exact_match"), Ok(Score::Binary(score)));
        run
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
        assert_eq!(
            store
                .get_run("run-a")
                .unwrap()
                .unwrap()
                .result
                .metadata
                .run_id,
            "run-a"
        );

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

        let output_path = temp.path().join("promoted.jsonl");
        let promotion = store
            .promote_annotations(
                "run-a",
                &PromoteAnnotationsRequest {
                    output_path: output_path.display().to_string(),
                    label: Some(String::from("needs_review")),
                },
            )
            .unwrap();
        assert_eq!(promotion.exported_count, 1);
        let promoted = std::fs::read_to_string(&output_path).unwrap();
        assert!(promoted.contains("sample-a"));
        assert!(promoted.contains("needs_review"));
        assert!(
            store.list_annotations("run-a").unwrap()[0]
                .promoted_at
                .is_some()
        );

        let rule = store
            .create_alert_rule(&CreateAlertRule {
                name: String::from("Exact match floor"),
                scorer_name: String::from("exact_match"),
                min_value: 0.5,
            })
            .unwrap();
        assert_eq!(rule.scorer_name, "exact_match");

        let alerts = store.evaluate_alerts("run-b").unwrap();
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].triggered);
    }

    #[test]
    fn run_store_detects_binary_drift_from_recent_runs() {
        let temp = tempdir().unwrap();
        let store = RunStore::open(temp.path().join("runs.sqlite")).unwrap();
        let base = Utc::now();

        let mut first = stored_binary_run_fixture("run-a", "sample-a", true);
        first.result.metadata.started_at = base;
        first.result.metadata.completed_at = base + Duration::seconds(1);

        let mut second = stored_binary_run_fixture("run-b", "sample-a", true);
        second.result.metadata.started_at = base + Duration::minutes(1);
        second.result.metadata.completed_at = base + Duration::minutes(1) + Duration::seconds(1);

        let mut third = stored_binary_run_fixture("run-c", "sample-a", false);
        third.result.metadata.started_at = base + Duration::minutes(2);
        third.result.metadata.completed_at = base + Duration::minutes(2) + Duration::seconds(1);

        store.store_run(&first).unwrap();
        store.store_run(&second).unwrap();
        store.store_run(&third).unwrap();

        let drift = store.detect_drift("run-c", 2).unwrap();
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].run_id, "run-c");
        assert_eq!(drift[0].scorer_name, "exact_match");
        assert_eq!(
            drift[0].baseline_run_ids,
            vec![String::from("run-b"), String::from("run-a")]
        );
        assert_eq!(
            drift[0].measurement,
            DriftMeasurement::Binary {
                observed_pass_rate: 0.0,
                baseline_pass_rate: 1.0,
                delta: -1.0,
            }
        );
        assert!(drift[0].threshold > 0.0);
        assert!(drift[0].triggered);
    }
}
