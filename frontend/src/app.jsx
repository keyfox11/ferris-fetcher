import { useState, useEffect } from 'react'
import { Trash2, FolderOpen, Pause, Play } from 'lucide-react'

function App() {
    const [downloads, setDownloads] = useState([])
    const [url, setUrl] = useState('')
    const [loading, setLoading] = useState(false)

    const formatBytes = (bytes, decimals = 2) => {
        if (!+bytes) return '0 Bytes'
        const k = 1024
        const dm = decimals < 0 ? 0 : decimals
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB']
        const i = Math.floor(Math.log(bytes) / Math.log(k))
        return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`
    }

    const fetchDownloads = async () => {
        try {
            const res = await fetch('http://localhost:3000/api/downloads')
            const data = await res.json()
            setDownloads(data)
        } catch (err) { console.error(err) }
    }

    useEffect(() => {
        fetchDownloads()
        const interval = setInterval(fetchDownloads, 500)
        return () => clearInterval(interval)
    }, [])

    const handleAdd = async (e) => {
        e.preventDefault()
        if (!url) return
        setLoading(true)
        await fetch('http://localhost:3000/api/downloads', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ url })
        })
        setUrl('')
        fetchDownloads()
        setLoading(false)
    }

    const handleOpen = async (id) => await fetch(`http://localhost:3000/api/downloads/${id}/open`, { method: 'POST' })
    const handlePause = async (id) => { await fetch(`http://localhost:3000/api/downloads/${id}/pause`, { method: 'POST' }); fetchDownloads() }
    const handleResume = async (id) => { await fetch(`http://localhost:3000/api/downloads/${id}/resume`, { method: 'POST' }); fetchDownloads() }

    const handleDelete = async (id) => {
        if (!window.confirm("Remove this download?")) return;
        await fetch(`http://localhost:3000/api/downloads/${id}`, { method: 'DELETE' })
        fetchDownloads()
    }
    const handleClearCompleted = async () => {
        if (!window.confirm("Remove all completed?")) return;
        await fetch(`http://localhost:3000/api/downloads/completed`, { method: 'DELETE' })
        fetchDownloads()
    }
    const handleClearAll = async () => {
        if (!window.confirm("WARNING: Remove ALL downloads?")) return;
        await fetch(`http://localhost:3000/api/downloads`, { method: 'DELETE' })
        fetchDownloads()
    }

    return (
        <div className="min-h-screen bg-slate-50 p-8 font-sans">
            <div className="max-w-4xl mx-auto">
                <header className="flex justify-between items-center mb-8">
                    <h1 className="text-4xl font-extrabold text-orange-600 tracking-tight flex items-center gap-3">
                        🦀 Ferris Fetcher
                    </h1>
                    <div className="flex gap-2">
                        <button onClick={handleClearCompleted} className="text-sm bg-white border border-gray-300 px-3 py-2 rounded hover:bg-gray-50 text-gray-600 transition">
                            Clear Completed
                        </button>
                        <button onClick={handleClearAll} className="text-sm bg-red-50 border border-red-200 text-red-600 px-3 py-2 rounded hover:bg-red-100 transition">
                            Clear All
                        </button>
                    </div>
                </header>

                <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100 mb-8">
                    <form onSubmit={handleAdd} className="flex gap-4">
                        <input
                            type="text"
                            placeholder="Paste file URL here..."
                            className="flex-1 p-3 bg-gray-50 border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500 transition"
                            value={url}
                            onChange={(e) => setUrl(e.target.value)}
                        />
                        <button
                            type="submit"
                            disabled={loading}
                            className="bg-orange-600 text-white px-8 py-3 rounded-lg font-bold hover:bg-orange-700 disabled:opacity-50 shadow-sm transition"
                        >
                            {loading ? 'Starting...' : 'Download'}
                        </button>
                    </form>
                </div>

                <div className="space-y-4">
                    {downloads.map(task => {
                        const progress = task.total_size > 0
                            ? Math.round((task.downloaded_bytes / task.total_size) * 100)
                            : 0;

                        return (
                            <div key={task.id} className="bg-white p-5 rounded-xl shadow-sm border border-gray-100 transition hover:shadow-md">
                                <div className="flex items-start justify-between mb-3">
                                    {/* Changed container to handle vertical stacking */}
                                    <div className="overflow-hidden pr-4 flex-1">
                                        <p className="font-bold text-gray-800 truncate text-lg" title={task.url}>
                                            {task.filename === "Pending..." ? task.url : task.filename}
                                        </p>

                                        <div className="mt-1 flex flex-col gap-1">
                                            {/* Line 1: Status */}
                                            <span className={`text-xs uppercase tracking-wider font-bold ${task.status === 'Completed' ? 'text-green-600' :
                                                    task.status === 'Downloading' ? 'text-blue-600' :
                                                        task.status === 'Paused' ? 'text-yellow-600' : 'text-gray-400'
                                                }`}>
                                                {typeof task.status === 'object' ? 'Error' : task.status}
                                            </span>

                                            {/* Line 2: Size Counter */}
                                            {task.total_size > 0 && (
                                                <span className="text-xs text-gray-400 font-mono">
                                                    {formatBytes(task.downloaded_bytes)} / {formatBytes(task.total_size)}
                                                </span>
                                            )}
                                        </div>
                                    </div>

                                    <div className="flex items-center gap-2">
                                        {task.status === 'Downloading' && (
                                            <button onClick={() => handlePause(task.id)} className="p-2 text-gray-500 hover:text-orange-600 bg-gray-50 rounded-full hover:bg-orange-50" title="Pause">
                                                <Pause size={18} />
                                            </button>
                                        )}
                                        {task.status === 'Paused' && (
                                            <button onClick={() => handleResume(task.id)} className="p-2 text-gray-500 hover:text-green-600 bg-gray-50 rounded-full hover:bg-green-50" title="Resume">
                                                <Play size={18} />
                                            </button>
                                        )}
                                        {task.status === 'Completed' && (
                                            <button onClick={() => handleOpen(task.id)} className="p-2 text-gray-500 hover:text-blue-600 bg-gray-50 rounded-full hover:bg-blue-50" title="Show in Folder">
                                                <FolderOpen size={18} />
                                            </button>
                                        )}
                                        <button onClick={() => handleDelete(task.id)} className="p-2 text-gray-400 hover:text-red-600 bg-gray-50 rounded-full hover:bg-red-50" title="Remove">
                                            <Trash2 size={18} />
                                        </button>
                                    </div>
                                </div>

                                <div className="w-full bg-gray-100 rounded-full h-2.5 overflow-hidden mt-3">
                                    <div
                                        className={`h-2.5 rounded-full transition-all duration-300 ${task.status === 'Completed' ? 'bg-green-500' :
                                                task.status === 'Paused' ? 'bg-yellow-400' : 'bg-orange-500'
                                            }`}
                                        style={{ width: `${progress}%` }}
                                    ></div>
                                </div>
                            </div>
                        )
                    })}

                    {downloads.length === 0 && (
                        <div className="text-center py-12 bg-gray-50 rounded-xl border-2 border-dashed border-gray-200">
                            <p className="text-gray-400">No downloads yet. Feed the crab!</p>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}

export default App