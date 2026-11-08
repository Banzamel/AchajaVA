//! The Plan tab: what is scheduled, the details and last report of an item, run now, remove.

use std::sync::Arc;

use achaja_core::error::{AppError, Result};
use achaja_core::plan::{self, jobs, store};
use serde::Serialize;
use tauri::State;

use crate::host::AppState;

#[derive(Serialize)]
pub struct PlanRow {
    id: u64,
    kind: String,
    label: String,
    next: f64,
    repeat: bool,
    report_mtime: f64,
}

#[tauri::command]
pub fn plan_items(st: State<'_, Arc<AppState>>) -> Vec<PlanRow> {
    store::items(&st.paths)
        .into_iter()
        .map(|i| PlanRow {
            id: i.id,
            label: i.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| i.text.clone()),
            kind: i.kind.clone(),
            next: i.next,
            repeat: i.repeat.is_some(),
            report_mtime: jobs::last_report_mtime(&st.paths, i.id),
        })
        .collect()
}

/// The description of an item and, for a job, its last report (None when it has not run).
#[tauri::command]
pub fn plan_detail(st: State<'_, Arc<AppState>>, id: u64) -> Option<(String, Option<String>)> {
    let item = store::find(&st.paths, &id.to_string())?;
    let report = (item.kind == "job").then(|| jobs::last_report(&st.paths, item.id)).flatten();
    Some((plan::describe(&item), report))
}

#[tauri::command]
pub fn plan_remove(st: State<'_, Arc<AppState>>, id: u64) -> usize {
    store::remove(&st.paths, &id.to_string())
}

#[tauri::command]
pub fn plan_run(st: State<'_, Arc<AppState>>, id: u64) -> Result<String> {
    let item = store::find(&st.paths, &id.to_string()).filter(|i| i.kind == "job").ok_or_else(|| AppError::msg("Nie ma takiego zadania w tle."))?;
    Ok(format!("{}|{}", item.name.clone().unwrap_or_default(), jobs::start(&st.paths, &item)))
}
