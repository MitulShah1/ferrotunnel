// app.js

const state = {
    requests: [],
    tunnels: [],
    selectedRequestId: null,
    metrics: { requests: 0, errorRate: 0 },
    maxRequests: 100,
    trafficData: [],
    maxTrafficPoints: 60, // 60 data points displayed
    requestTimestamps: [] // Track recent request timestamps for rate calculation
};

// DOM Elements
const els = {
    requestTableBody: document.getElementById('requests-table-body'),
    detailsPanel: document.getElementById('details-panel'),
    statTunnels: document.getElementById('stat-tunnels'),
    statRequests: document.getElementById('stat-requests'),
    statErrorRate: document.getElementById('stat-error-rate'),
    btnClosePanel: document.getElementById('btn-close-panel'),
    btnReplay: document.getElementById('btn-replay'),
    chartCanvas: document.getElementById('trafficChart'),
};

// Formatters
const fmtTime = (iso) => new Date(iso).toLocaleTimeString();
const fmtDuration = (ms) => `${ms}ms`;
const fmtSize = (bytes) => bytes < 1024 ? `${bytes}B` : `${(bytes / 1024).toFixed(1)}KB`;

// Init Chart
let trafficChart;

function initChart() {
    trafficChart = new Chart(els.chartCanvas, {
        type: 'line',
        data: {
            labels: [],
            datasets: [{
                label: 'Requests/min',
                data: [],
                borderColor: '#6366f1',
                tension: 0.4,
                fill: true,
                backgroundColor: 'rgba(99, 102, 241, 0.1)',
                pointRadius: 2,
                pointHoverRadius: 4
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: { duration: 300 },
            scales: {
                y: { 
                    beginAtZero: true, 
                    grid: { color: '#334155' },
                    ticks: { color: '#94a3b8', stepSize: 1 }
                },
                x: { 
                    grid: { display: false }, 
                    ticks: { color: '#94a3b8', maxTicksLimit: 10 } 
                }
            },
            plugins: { legend: { display: false } }
        }
    });

    // Initialize chart with empty data
    initChartLabels();

    // Update chart every 2 seconds for real-time feel
    setInterval(updateTrafficChart, 2000);
}

function initChartLabels() {
    const now = Date.now();
    const labels = [];

    // Create 60 labels for the past 2 minutes (2-second intervals)
    for (let i = state.maxTrafficPoints - 1; i >= 0; i--) {
        const time = new Date(now - i * 2000);
        labels.push(time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }));
    }

    trafficChart.data.labels = labels;
    state.trafficData = new Array(state.maxTrafficPoints).fill(0);
    trafficChart.data.datasets[0].data = [...state.trafficData];
    trafficChart.update('none');
}

function updateTrafficChart() {
    const now = Date.now();
    const timeLabel = new Date(now).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });

    // Clean up old timestamps (older than 2 seconds for current bucket)
    const cutoff = now - 2000;
    state.requestTimestamps = state.requestTimestamps.filter(ts => ts > cutoff);

    // Count requests in the last 2 seconds
    const recentCount = state.requestTimestamps.length;

    // Shift data left and add new data point
    state.trafficData.shift();
    state.trafficData.push(recentCount);

    // Shift labels left and add new label
    trafficChart.data.labels.shift();
    trafficChart.data.labels.push(timeLabel);

    trafficChart.data.datasets[0].data = [...state.trafficData];
    trafficChart.update('none');

    // Clear timestamps after recording
    state.requestTimestamps = [];
}

function recordRequest() {
    state.requestTimestamps.push(Date.now());
}

// Render Logic
function renderStats() {
    els.statTunnels.textContent = state.tunnels.length;
    els.statRequests.textContent = state.metrics.requests;
    els.statErrorRate.textContent = `${state.metrics.errorRate.toFixed(1)}%`;
}

function renderRequestRow(req) {
    const tr = document.createElement('tr');
    tr.dataset.id = req.id;
    tr.innerHTML = `
        <td><span class="badge-method method-${req.method}">${req.method}</span></td>
        <td><div style="max-width:300px;overflow:hidden;text-overflow:ellipsis">${req.path}</div></td>
        <td><span class="status-code status-${Math.floor(req.status / 100)}xx">${req.status}</span></td>
        <td>${fmtSize(req.response_size || 0)}</td>
        <td>${fmtDuration(req.duration_ms || 0)}</td>
        <td style="color:var(--text-secondary)">${fmtTime(req.timestamp)}</td>
    `;
    tr.addEventListener('click', () => openDetails(req));
    return tr;
}

function renderRequestList() {
    els.requestTableBody.innerHTML = '';
    state.requests.forEach(req => {
        els.requestTableBody.appendChild(renderRequestRow(req));
    });
}

// Detail Panel
async function openDetails(summaryReq) {
    state.selectedRequestId = summaryReq.id;
    els.detailsPanel.classList.add('open');

    // Show loading state while fetching full details
    document.getElementById('detail-method').textContent = summaryReq.method;
    document.getElementById('detail-method').className = `method-badge method-${summaryReq.method}`;
    document.getElementById('detail-path').textContent = summaryReq.path;

    // Clear previous data
    document.getElementById('detail-req-headers').innerHTML = '<div style="padding:1rem">Loading...</div>';
    document.getElementById('detail-res-headers').innerHTML = '<div style="padding:1rem">Loading...</div>';
    document.getElementById('detail-req-body').textContent = 'Loading...';
    document.getElementById('detail-res-body').textContent = 'Loading...';

    try {
        const res = await fetch(`/api/v1/requests/${summaryReq.id}`);
        if (!res.ok) throw new Error('Failed to fetch details');
        const req = await res.json();

        // Render Headers
        const renderHeaders = (headers, containerId) => {
            const container = document.getElementById(containerId);
            if (!headers || Object.keys(headers).length === 0) {
                container.innerHTML = '<div style="color:var(--text-muted);padding:1rem">No headers captured</div>';
                return;
            }
            container.innerHTML = Object.entries(headers).map(([k, v]) => `
                <div><dt>${k}</dt><dd>${v}</dd></div>
            `).join('');
        };

        renderHeaders(req.request_headers, 'detail-req-headers');
        renderHeaders(req.response_headers, 'detail-res-headers');

        // Render Body
        const renderBody = (body, containerId) => {
            const container = document.getElementById(containerId);
            if (!body) { container.innerHTML = '// No body content'; return; }
            try {
                // If body is already a string but potentially JSON
                if (typeof body !== 'string') body = JSON.stringify(body, null, 2);
                const json = JSON.parse(body);
                container.textContent = JSON.stringify(json, null, 2);
            } catch {
                container.textContent = body;
            }
        };

        renderBody(req.request_body, 'detail-req-body');
        renderBody(req.response_body, 'detail-res-body');

    } catch (e) {
        console.error("Error fetching details", e);
        document.getElementById('detail-req-headers').innerHTML = '<div style="color:red;padding:1rem">Error loading details</div>';
    }

    // Reset tabs
    document.querySelectorAll('.tab-link').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    document.querySelector('.tab-link[data-tab="req-headers"]').classList.add('active');
    document.getElementById('req-headers').classList.add('active');
}

// Data Fetching
async function fetchInitialData() {
    try {
        const [tunnelsRes, reqsRes, healthRes] = await Promise.all([
            fetch('/api/v1/tunnels'),
            fetch('/api/v1/requests'),
            fetch('/api/v1/health')
        ]);

        state.tunnels = await tunnelsRes.json();
        state.requests = await reqsRes.json();
        const health = await healthRes.json();

        if (health && health.version) {
            const verEl = document.getElementById('setting-version');
            if (verEl) verEl.textContent = 'v' + health.version;
        }

        state.metrics.requests = state.requests.length;

        renderStats();
        renderRequestList();
        renderTunnels();
        renderAllRequests();
    } catch (e) {
        console.error("Failed to fetch initial data", e);
    }
}

// Render Tunnels (New)
function renderTunnels() {
    const container = document.getElementById('tunnels-list');
    if (!state.tunnels.length) {
        container.innerHTML = '<p style="color:var(--text-secondary)">No active tunnels found.</p>';
        return;
    }

    const getStatusStyle = (status) => {
        if (status === 'connected') return { bg: 'rgba(34,197,94,0.1)', color: 'var(--status-success)' };
        if (status === 'connecting') return { bg: 'rgba(234,179,8,0.1)', color: '#eab308' };
        return { bg: 'rgba(239,68,68,0.1)', color: 'var(--status-error)' };
    };

    container.innerHTML = state.tunnels.map(t => {
        const statusStyle = getStatusStyle(t.status);
        const hasPublicUrl = t.public_url && t.public_url !== 'N/A';
        return `
        <div class="tunnel-item" style="display:flex; justify-content:space-between; align-items:center; padding:1.25rem; background:rgba(255,255,255,0.03); border:1px solid var(--border-color); border-radius:0.75rem; margin-bottom:1rem;">
            <div style="display:flex; align-items:center; gap:1rem;">
                <div style="width:40px; height:40px; background:rgba(37,99,235,0.1); border-radius:0.5rem; display:flex; align-items:center; justify-content:center; color:var(--primary-color);">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>
                </div>
                <div>
                    <div style="font-weight:600; font-size:1.05rem; margin-bottom:0.25rem; display:flex; align-items:center; gap:0.5rem;">
                        ${t.subdomain || 'Local Tunnel'}
                        ${hasPublicUrl ? `<a href="${t.public_url}" target="_blank" style="color:var(--text-secondary); text-decoration:none; display:flex; align-items:center" title="Open URL">
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path><polyline points="15 3 21 3 21 9"></polyline><line x1="10" y1="14" x2="21" y2="3"></line></svg>
                        </a>` : ''}
                    </div>
                    <div style="color:var(--text-secondary); font-family:var(--font-mono); font-size:0.85rem;">
                        ${hasPublicUrl ? `${t.public_url} <span style="margin:0 0.5rem; opacity:0.5">→</span>` : ''} ${t.local_addr}
                    </div>
                </div>
            </div>
            <div style="text-align:right">
                <span class="status-indicator" style="display:inline-flex; align-items:center; gap:0.4rem; background:${statusStyle.bg}; color:${statusStyle.color}; padding:0.4rem 0.75rem; border-radius:2rem; font-size:0.85rem; font-weight:500;">
                    <span style="width:6px; height:6px; border-radius:50%; background:currentColor"></span>
                    ${t.status.toUpperCase()}
                </span>
                <div style="margin-top:0.5rem; font-size:0.8rem; color:var(--text-muted)">
                    Created ${fmtTime(t.created_at)}
                </div>
            </div>
        </div>
    `}).join('');
}

// Render All Requests (New)
function renderAllRequests() {
    const container = document.querySelector('#view-requests .card > div');
    // Using simple reuse of the logic but customized for view
    const tableHtml = `
    <div class="table-responsive">
        <table class="data-table">
            <thead>
                <tr>
                    <th>Method</th>
                    <th>Path</th>
                    <th>Status</th>
                    <th>Size</th>
                    <th>Duration</th>
                    <th>Time</th>
                </tr>
            </thead>
            <tbody>
                ${state.requests.map(req => {
        return `
                    <tr onclick='openDetails(${JSON.stringify(req).replace(/'/g, "&apos;")})'>
                        <td><span class="badge-method method-${req.method}">${req.method}</span></td>
                        <td>${req.path}</td>
                        <td><span class="status-code status-${Math.floor(req.status / 100)}xx">${req.status}</span></td>
                        <td>${fmtSize(req.response_size || 0)}</td>
                        <td>${fmtDuration(req.duration_ms || 0)}</td>
                        <td style="color:var(--text-secondary)">${fmtTime(req.timestamp)}</td>
                    </tr>
                    `;
    }).join('')}
            </tbody>
        </table>
    </div>`;
    container.innerHTML = tableHtml;
}

// SSE Setup
function setupSSE() {
    const evtSource = new EventSource("/api/v1/events");

    evtSource.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            console.log("SSE Event:", data);

            if (data.type === 'new_request') {
                state.requests.unshift(data.payload);
                if (state.requests.length > state.maxRequests) state.requests.pop();

                // Update metrics
                state.metrics.requests++;
                recordRequest(); // Track for live traffic chart
                if (data.payload.status >= 400) {
                    state.metrics.errorRate = (state.metrics.errorRate * 9 + 100) / 10;
                } else {
                    state.metrics.errorRate = (state.metrics.errorRate * 9) / 10;
                }

                renderStats();
                // Update both lists
                els.requestTableBody.prepend(renderRequestRow(data.payload));
                renderAllRequests();

                if (els.requestTableBody.children.length > state.maxRequests) {
                    els.requestTableBody.lastElementChild.remove();
                }
            }
        } catch (e) { console.error("SSE Parse Error", e); }
    };
}

// Replay
async function replayRequest() {
    if (!state.selectedRequestId) return;
    const req = state.requests.find(r => r.id === state.selectedRequestId);
    if (!req) return;

    if (!confirm(`Replay ${req.method} ${req.path}?`)) return;

    try {
        els.btnReplay.disabled = true;
        els.btnReplay.textContent = 'Replaying...';

        const res = await fetch(`/api/v1/requests/${state.selectedRequestId}/replay`, {
            method: 'POST'
        });

        const data = await res.json();

        if (!res.ok) {
            throw new Error(data.error || 'Replay failed');
        }

        alert(`Replay successful! Status: ${data.response_status}`);
    } catch (e) {
        alert("Replay failed: " + e.message);
    } finally {
        els.btnReplay.disabled = false;
        els.btnReplay.textContent = 'Replay';
    }
}

// Event Listeners
document.addEventListener('DOMContentLoaded', () => {
    initChart();
    fetchInitialData();
    setupSSE();

    // Sidebar Navigation
    document.querySelectorAll('.nav-links a').forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();
            const viewName = link.dataset.view;

            // Update Sidebar
            document.querySelectorAll('.nav-links li').forEach(li => li.classList.remove('active'));
            link.parentElement.classList.add('active');

            // Update Main View
            document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
            const targetView = document.getElementById(`view-${viewName}`);
            if (targetView) targetView.classList.add('active');
        });
    });

    // Details Panel Tabs
    document.querySelectorAll('.tab-link').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.tab-link').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            btn.classList.add('active');
            document.getElementById(btn.dataset.tab).classList.add('active');
        });
    });

    els.btnClosePanel.addEventListener('click', () => {
        els.detailsPanel.classList.remove('open');
        state.selectedRequestId = null;
    });

    // Clear Button
    const clearBtn = document.getElementById('clear-requests');
    if (clearBtn) {
        clearBtn.addEventListener('click', () => {
            state.requests = [];
            state.metrics.requests = 0;
            state.metrics.errorRate = 0;
            renderStats();
            renderRequestList();
            renderAllRequests();
        });
    }

    els.btnReplay.addEventListener('click', replayRequest);
});
