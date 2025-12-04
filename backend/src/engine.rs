use reqwest::header::RANGE;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering}; // NEW: For thread-safe counting
use std::sync::Arc;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};

pub fn get_download_dir() -> PathBuf {
    let base_dirs = directories::UserDirs::new().expect("Could not find user directories");
    let mut path = base_dirs
        .download_dir()
        .expect("No download dir")
        .to_path_buf();
    path.push("FF");
    path
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub total_size: Option<u64>,
    pub downloaded_bytes: u64,
    pub status: DownloadStatus,
    pub save_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Completed,
    Error(String),
}

pub async fn start_multistream_download(
    url: String,
    task_id: String,
    state_updater: Arc<Mutex<Vec<DownloadTask>>>,
) -> Result<(), String> {
    let ff_dir = get_download_dir();
    tokio::fs::create_dir_all(&ff_dir)
        .await
        .map_err(|e| e.to_string())?;

    let filename = url.split('/').last().unwrap_or("download.bin").to_string();
    let file_path = ff_dir.join(&filename);

    // 1. Get Details
    let client = reqwest::Client::new();
    let head = client.head(&url).send().await.map_err(|e| e.to_string())?;

    let content_length = head
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct| ct.to_str().ok())
        .and_then(|ct| ct.parse::<u64>().ok())
        .unwrap_or(0);

    let accepts_ranges = head
        .headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .map(|v| v == "bytes")
        .unwrap_or(false);

    // 2. Initialize File
    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| e.to_string())?;
    file.set_len(content_length)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Update State (Start)
    {
        let mut tasks = state_updater.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = DownloadStatus::Downloading;
            task.total_size = Some(content_length);
            task.save_path = file_path.to_string_lossy().to_string();
            // Reset bytes if restarting (simple resume)
            task.downloaded_bytes = 0;
        }
    }

    // 4. Setup Progress Tracking
    // We use an Atomic counter so all threads can update it cheaply
    let progress_counter = Arc::new(AtomicU64::new(0));

    // SPAWN A REPORTER: Updates the global state every 500ms
    let progress_clone = progress_counter.clone();
    let state_clone_reporter = state_updater.clone();
    let id_clone_reporter = task_id.clone();

    let reporter_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let current_bytes = progress_clone.load(Ordering::Relaxed);

            let mut tasks = state_clone_reporter.lock().await;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id_clone_reporter) {
                // Only update if we are still downloading
                if task.status == DownloadStatus::Downloading {
                    task.downloaded_bytes = current_bytes;
                } else {
                    break; // Stop reporting if paused/cancelled
                }
            }
        }
    });

    if accepts_ranges && content_length > 0 {
        // --- MULTI STREAM ---
        println!("Starting multi-stream download for {}", filename);
        let chunk_count = 8;
        let chunk_size = content_length / chunk_count;
        let mut handles = vec![];
        let sem = Arc::new(Semaphore::new(chunk_count as usize));

        for i in 0..chunk_count {
            let start = i * chunk_size;
            let end = if i == chunk_count - 1 {
                content_length - 1
            } else {
                (i + 1) * chunk_size - 1
            };

            let url_clone = url.clone();
            let path_clone = file_path.clone();
            let sem_clone = sem.clone();
            let progress_clone_worker = progress_counter.clone(); // Worker needs access to counter

            let handle = tokio::spawn(async move {
                let _permit = sem_clone.acquire().await.unwrap();
                let client = reqwest::Client::new();

                // Request the Range
                let mut response = client
                    .get(&url_clone)
                    .header(RANGE, format!("bytes={}-{}", start, end))
                    .send()
                    .await
                    .unwrap();

                let mut file = tokio::fs::OpenOptions::new()
                    .write(true)
                    .open(&path_clone)
                    .await
                    .unwrap();

                file.seek(tokio::io::SeekFrom::Start(start)).await.unwrap();

                // NEW: STREAMING LOGIC
                // Read chunks as they arrive
                while let Ok(Some(chunk)) = response.chunk().await {
                    file.write_all(&chunk).await.unwrap();
                    // Update the atomic counter immediately
                    progress_clone_worker.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                }
            });
            handles.push(handle);
        }

        for h in handles {
            let _ = h.await;
        }
    } else {
        // --- SINGLE STREAM ---
        println!("Falling back to single stream for {}", filename);
        let mut resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
        let progress_clone_worker = progress_counter.clone();

        while let Ok(Some(chunk)) = resp.chunk().await {
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            progress_clone_worker.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
    }

    // 5. Cleanup
    reporter_handle.abort(); // Stop the background reporter

    // Final Update to ensure 100%
    {
        let mut tasks = state_updater.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = DownloadStatus::Completed;
            task.downloaded_bytes = content_length;
        }
    }

    Ok(())
}
