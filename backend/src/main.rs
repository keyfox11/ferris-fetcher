use axum::{
    extract::Path,
    routing::{delete, get, post}, // Added delete
    Extension,
    Json,
    Router,
};
use dashmap::DashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid; // NEW: Thread-safe map for control handles

mod engine;
use engine::{DownloadStatus, DownloadTask};

type AppState = Arc<Mutex<Vec<DownloadTask>>>;
// NEW: A registry to hold the "Stop Button" for each active download
type TaskRegistry = Arc<DashMap<String, tokio::task::AbortHandle>>;

const HISTORY_FILE: &str = "history.json";

#[tokio::main]
async fn main() {
    let initial_data = load_history();
    let state: AppState = Arc::new(Mutex::new(initial_data));
    let registry: TaskRegistry = Arc::new(DashMap::new());

    // Auto-save task
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            save_history(&state_clone).await;
        }
    });

    let app = Router::new()
        // Standard Routes
        .route(
            "/api/downloads",
            get(list_downloads).post(add_download).delete(delete_all),
        )
        .route("/api/downloads/completed", delete(delete_completed))
        .route("/api/downloads/:id", delete(delete_single))
        .route("/api/downloads/:id/open", post(open_file_location))
        // Control Routes
        .route("/api/downloads/:id/pause", post(pause_download))
        .route("/api/downloads/:id/resume", post(resume_download))
        .layer(Extension(state))
        .layer(Extension(registry)) // Inject the registry
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    println!("Ferris Fetcher listening on localhost:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- CONTROLLERS ---

// 1. Pause Logic
async fn pause_download(
    Path(id): Path<String>,
    Extension(state): Extension<AppState>,
    Extension(registry): Extension<TaskRegistry>,
) -> Json<String> {
    // Stop the thread
    if let Some((_, handle)) = registry.remove(&id) {
        handle.abort();
    }
    // Update status text
    let mut tasks = state.lock().await;
    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
        task.status = DownloadStatus::Paused;
    }
    Json("Paused".to_string())
}

// 2. Resume Logic
async fn resume_download(
    Path(id): Path<String>,
    Extension(state): Extension<AppState>,
    Extension(registry): Extension<TaskRegistry>,
) -> Json<String> {
    let mut tasks = state.lock().await;
    // Find the task data
    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
        // Only resume if currently paused or failed
        task.status = DownloadStatus::Pending; // Reset status

        // Spawn a new download thread
        let url = task.url.clone();
        let id_clone = id.clone();
        let state_clone = state.clone(); // We need a fresh clone of the Arc for the thread

        // Important: We drop the lock on 'tasks' here implicitly before spawning,
        // otherwise the download thread would deadlock waiting for us to finish.
        drop(tasks);

        let handle = tokio::spawn(async move {
            let _ = engine::start_multistream_download(url, id_clone, state_clone).await;
        });

        // Save the new handle so we can pause it again
        registry.insert(id.clone(), handle.abort_handle());
    }
    Json("Resumed".to_string())
}

// 3. Delete Single
async fn delete_single(
    Path(id): Path<String>,
    Extension(state): Extension<AppState>,
    Extension(registry): Extension<TaskRegistry>,
) -> Json<String> {
    // Abort if running
    if let Some((_, handle)) = registry.remove(&id) {
        handle.abort();
    }
    // Remove from list
    let mut tasks = state.lock().await;
    tasks.retain(|t| t.id != id);
    Json("Deleted".to_string())
}

// 4. Delete Completed
async fn delete_completed(Extension(state): Extension<AppState>) -> Json<String> {
    let mut tasks = state.lock().await;
    tasks.retain(|t| t.status != DownloadStatus::Completed);
    Json("Cleaned".to_string())
}

// 5. Delete All
async fn delete_all(
    Extension(state): Extension<AppState>,
    Extension(registry): Extension<TaskRegistry>,
) -> Json<String> {
    // Abort everything
    registry.clear(); // This drops all handles, effectively aborting them?
                      // Actually, clearing DashMap doesn't call abort automatically.
                      // We must iterate.
    for entry in registry.iter() {
        entry.value().abort();
    }
    registry.clear();

    let mut tasks = state.lock().await;
    tasks.clear();
    Json("Nuked".to_string())
}

// --- EXISTING HELPERS ---

fn load_history() -> Vec<DownloadTask> {
    if let Ok(data) = std::fs::read_to_string(HISTORY_FILE) {
        serde_json::from_str(&data).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    }
}

async fn save_history(state: &AppState) {
    let tasks = state.lock().await;
    if let Ok(json) = serde_json::to_string_pretty(&*tasks) {
        let _ = std::fs::write(HISTORY_FILE, json);
    }
}

async fn list_downloads(Extension(state): Extension<AppState>) -> Json<Vec<DownloadTask>> {
    let tasks = state.lock().await;
    Json(tasks.clone())
}

#[derive(serde::Deserialize)]
struct CreateDownload {
    url: String,
}

async fn add_download(
    Extension(state): Extension<AppState>,
    Extension(registry): Extension<TaskRegistry>,
    Json(payload): Json<CreateDownload>,
) -> Json<DownloadTask> {
    let id = Uuid::new_v4().to_string();
    let new_task = DownloadTask {
        id: id.clone(),
        url: payload.url.clone(),
        filename: "Pending...".to_string(),
        total_size: None,
        downloaded_bytes: 0,
        status: DownloadStatus::Pending,
        save_path: String::new(),
    };

    {
        let mut tasks = state.lock().await;
        tasks.push(new_task.clone());
    }
    save_history(&state).await;

    let state_clone = state.clone();
    let id_clone = id.clone();

    // Spawn and capture handle
    let handle = tokio::spawn(async move {
        let _ = engine::start_multistream_download(payload.url, id_clone, state_clone).await;
    });

    // Store handle
    registry.insert(id, handle.abort_handle());

    Json(new_task)
}

async fn open_file_location(
    Path(id): Path<String>,
    Extension(state): Extension<AppState>,
) -> Json<String> {
    let tasks = state.lock().await;
    if let Some(task) = tasks.iter().find(|t| t.id == id) {
        if cfg!(target_os = "windows") {
            Command::new("explorer")
                .arg("/select,")
                .arg(&task.save_path)
                .spawn()
                .expect("Failed to open explorer");
        }
    }
    Json("Opened".to_string())
}
