# 🦀 Ferris Fetcher (MVP)

Ferris Fetcher is a high-performance, multi-stream download manager built to demonstrate the capabilities of Rust for backend throughput and React for a modern, responsive frontend.

## 🚀 Key Features

**Multi-Stream Downloading:** Splits files into 8 simultaneous streams for maximum bandwidth utilization.

**State Persistence:** Auto-saves download history to `history.json` (survives restarts).

**Concurrency Control:** Pause, Resume (Restart), and Cancel downloads.

**Real-time Progress:** Live streaming updates of download speeds and percentage.

**OS Integration:** “Show in Folder” integration for Windows Explorer.

## 🛠️ Tech Stack

### Backend (Rust)
- **Runtime:** Tokio (Async I/O)  
- **Web Framework:** Axum  
- **HTTP Client:** Reqwest  
- **State Management:** DashMap (Thread-safe concurrency) & Serde (JSON persistence)

### Frontend (React)
- **Build Tool:** Vite  
- **Styling:** Tailwind CSS v4  
- **Icons:** Lucide React  

## 📦 Prerequisites

Before running the application, ensure you have the following installed:

- **Rust & Cargo:** Install Rust  
- **Node.js & npm:** Install Node.js (LTS version recommended)

## 🏃‍♂️ Getting Started

### 1. Start the Backend (The Engine)
The backend handles the file writing, threading, and persistence.

```bash
cd backend
cargo run
```

You should see:
```bash
Ferris Fetcher listening on localhost:3000
```

### 2. Start the Frontend (The Interface)

Open a new terminal, navigate to the frontend folder, and launch the UI.

```bash
cd frontend
npm install
npm run dev
```

Open the URL shown (usually `http://localhost:5173`) in your browser.

## 📖 How to Use

- **Add a Download:** Paste a direct file URL (e.g., an image or ISO) into the input box and click *Download*.  
- **Pause/Resume:** Use the controls in the list to manage active downloads.  
  - In this MVP, “Resume” restarts the stream to ensure data integrity.  
- **Locate Files:** Click the Folder Icon on completed downloads to open the file location in Windows Explorer.  
- **Default Location:** `{User}/Downloads/FF/`

## 📂 Project Structure

- `backend/src/engine.rs`: Core logic for multi-threading and streaming file writes.  
- `backend/src/main.rs`: API routes and state management.  
- `frontend/src/App.jsx`: Main UI logic and state polling.  
- `backend/history.json`: Created automatically to persist download lists.

## 🚧 Known Limitations (MVP)

- **Platform:** Windows 10/11 optimized (Mac/Linux support planned for V2).  
- **Resume Behavior:** Currently restarts the download rather than filling specific byte-gaps (simpler implementation for MVP).
