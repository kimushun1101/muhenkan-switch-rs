const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ── State ──
let config = null;       // Current config from backend
let guiSettings = {};    // GUI-only settings
let keyBindings = {};    // kbd file key bindings { apps: {editor: "A"}, ... }

// ── Tab switching ──
document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
    document.querySelectorAll(".panel").forEach((p) => p.classList.remove("active"));
    tab.classList.add("active");
    document.getElementById(`panel-${tab.dataset.tab}`).classList.add("active");
  });
});

// ── Load config on startup ──
async function loadConfig() {
  try {
    config = await invoke("get_config");
    renderConfig();
  } catch (e) {
    console.error("設定の読み込みに失敗:", e);
  }
}

// ── Render config to UI ──
function renderConfig() {
  if (!config) return;

  // Timestamp
  renderTimestamp();

  // Search engines
  renderSearchList();

  // Folders
  renderFoldersList();

  // Apps
  renderAppsList();
}

// ── Timestamp ──
function renderTimestamp() {
  const presetSelect = document.getElementById("ts-format-preset");
  const customInput = document.getElementById("ts-format-custom");
  const format = config.timestamp.format;

  // Check if format matches a preset
  const presetOption = Array.from(presetSelect.options).find((o) => o.value === format);
  if (presetOption) {
    presetSelect.value = format;
    customInput.classList.add("hidden");
  } else {
    presetSelect.value = "custom";
    customInput.value = format;
    customInput.classList.remove("hidden");
  }

  // Position
  document.querySelector(`input[name="ts-position"][value="${config.timestamp.position}"]`).checked = true;

  updateTimestampPreview();
}

function getTimestampFormat() {
  const preset = document.getElementById("ts-format-preset").value;
  if (preset === "custom") {
    return document.getElementById("ts-format-custom").value;
  }
  return preset;
}

async function updateTimestampPreview() {
  const format = getTimestampFormat();
  try {
    const preview = await invoke("validate_timestamp_format", { format });
    document.getElementById("ts-preview").textContent = preview;
    document.getElementById("ts-preview").style.color = "";
  } catch (e) {
    document.getElementById("ts-preview").textContent = e;
    document.getElementById("ts-preview").style.color = "var(--red)";
  }
}

document.getElementById("ts-format-preset").addEventListener("change", (e) => {
  const customInput = document.getElementById("ts-format-custom");
  if (e.target.value === "custom") {
    customInput.classList.remove("hidden");
    customInput.focus();
  } else {
    customInput.classList.add("hidden");
  }
  updateTimestampPreview();
});

document.getElementById("ts-format-custom").addEventListener("input", () => {
  updateTimestampPreview();
});

// ── Drag reordering (mouse event based) ──
let dragState = null;

function enableDragReorder(row) {
  const handle = document.createElement("span");
  handle.className = "drag-handle";
  handle.textContent = "≡";
  handle.title = "ドラッグで並べ替え";
  row.prepend(handle);

  handle.addEventListener("mousedown", (e) => {
    e.preventDefault();
    const container = row.parentElement;
    row.classList.add("dragging");
    dragState = { row, container, startY: e.clientY };

    function onMouseMove(e) {
      if (!dragState) return;
      const siblings = [...container.querySelectorAll(".list-row:not(.dragging)")];
      for (const sibling of siblings) {
        const rect = sibling.getBoundingClientRect();
        const mid = rect.top + rect.height / 2;
        if (e.clientY < mid) {
          container.insertBefore(dragState.row, sibling);
          return;
        }
      }
      // Past all siblings — move to end
      container.appendChild(dragState.row);
    }

    function onMouseUp() {
      if (dragState) {
        dragState.row.classList.remove("dragging");
        dragState = null;
      }
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    }

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  });
}

// ── Search engines ──
function renderSearchList() {
  const container = document.getElementById("search-list");
  container.innerHTML = "";
  const bindings = keyBindings.search || {};
  for (const [key, url] of Object.entries(config.search || {})) {
    const boundKey = bindings[key] || "";
    addSearchRow(container, key, url, boundKey);
  }
}

function addSearchRow(container, key = "", url = "", boundKey = "") {
  const row = document.createElement("div");
  row.className = "list-row";
  const keyLabel = boundKey ? `<span class="bound-key" title="無変換+${escapeHtml(boundKey)}">${escapeHtml(boundKey)}</span>` : "";
  row.innerHTML = `
    ${keyLabel}
    <input type="text" class="key-input" placeholder="キー" value="${escapeHtml(key)}">
    <input type="text" placeholder="URL テンプレート ({query})" value="${escapeHtml(url)}">
    <button class="btn-remove" title="削除">&times;</button>
  `;
  row.querySelector(".btn-remove").addEventListener("click", () => row.remove());
  enableDragReorder(row);
  container.appendChild(row);
}

document.getElementById("btn-add-search").addEventListener("click", () => {
  addSearchRow(document.getElementById("search-list"));
});

// ── Folders ──
function renderFoldersList() {
  const container = document.getElementById("folders-list");
  container.innerHTML = "";
  const bindings = keyBindings.folders || {};
  for (const [key, path] of Object.entries(config.folders || {})) {
    const boundKey = bindings[key] || "";
    addFolderRow(container, key, path, boundKey);
  }
}

function addFolderRow(container, key = "", path = "", boundKey = "") {
  const row = document.createElement("div");
  row.className = "list-row";
  const keyLabel = boundKey ? `<span class="bound-key" title="無変換+${escapeHtml(boundKey)}">${escapeHtml(boundKey)}</span>` : "";
  row.innerHTML = `
    ${keyLabel}
    <input type="text" class="key-input" placeholder="キー" value="${escapeHtml(key)}">
    <input type="text" class="path-input" placeholder="パス (~/Documents)" value="${escapeHtml(path)}">
    <button class="btn-browse" title="参照">参照</button>
    <button class="btn-remove" title="削除">&times;</button>
  `;
  row.querySelector(".btn-remove").addEventListener("click", () => row.remove());
  row.querySelector(".btn-browse").addEventListener("click", async () => {
    try {
      const selected = await invoke("browse_folder");
      if (selected) {
        row.querySelector(".path-input").value = selected;
      }
    } catch (e) {
      console.error("フォルダ選択に失敗:", e);
    }
  });
  enableDragReorder(row);
  container.appendChild(row);
}

document.getElementById("btn-add-folder").addEventListener("click", () => {
  addFolderRow(document.getElementById("folders-list"));
});

// ── Apps ──
function renderAppsList() {
  const container = document.getElementById("apps-list");
  container.innerHTML = "";
  const bindings = keyBindings.apps || {};
  for (const [key, entry] of Object.entries(config.apps || {})) {
    const process = typeof entry === "string" ? entry : entry.process;
    const command = typeof entry === "string" ? "" : entry.command || "";
    const boundKey = bindings[key] || "";
    addAppRow(container, key, process, command, boundKey);
  }
}

function addAppRow(container, key = "", process = "", command = "", boundKey = "") {
  const row = document.createElement("div");
  row.className = "list-row";
  const keyLabel = boundKey ? `<span class="bound-key" title="無変換+${escapeHtml(boundKey)}">${escapeHtml(boundKey)}</span>` : "";
  row.innerHTML = `
    ${keyLabel}
    <input type="text" class="key-input" placeholder="キー" value="${escapeHtml(key)}">
    <input type="text" class="process-input" placeholder="プロセス名" value="${escapeHtml(process)}">
    <input type="text" class="command-input" placeholder="実行コマンド" value="${escapeHtml(command)}">
    <button class="btn-pick-process" title="プロセス選択">選択</button>
    <button class="btn-remove" title="削除">&times;</button>
  `;
  row.querySelector(".btn-remove").addEventListener("click", () => row.remove());
  row.querySelector(".btn-pick-process").addEventListener("click", async () => {
    const selected = await showProcessPicker();
    if (selected) {
      row.querySelector(".process-input").value = selected;
      row.querySelector(".command-input").value = selected.toLowerCase();
    }
  });
  enableDragReorder(row);
  container.appendChild(row);
}

document.getElementById("btn-add-app").addEventListener("click", () => {
  addAppRow(document.getElementById("apps-list"));
});

// ── Process picker modal ──
async function showProcessPicker() {
  return new Promise(async (resolve) => {
    let processes = [];
    try {
      processes = await invoke("get_running_processes");
    } catch (e) {
      console.error("プロセス一覧の取得に失敗:", e);
      resolve(null);
      return;
    }

    const overlay = document.createElement("div");
    overlay.className = "modal-overlay";
    overlay.innerHTML = `
      <div class="modal">
        <div class="modal-header">プロセスを選択</div>
        <div class="modal-body">
          <input type="text" class="modal-search" placeholder="フィルター...">
          <ul class="modal-list"></ul>
        </div>
        <div class="modal-footer">
          <button class="btn-cancel">キャンセル</button>
        </div>
      </div>
    `;

    const list = overlay.querySelector(".modal-list");
    const searchInput = overlay.querySelector(".modal-search");

    function renderProcessList(filter = "") {
      list.innerHTML = "";
      const filtered = processes.filter((p) =>
        p.name.toLowerCase().includes(filter.toLowerCase())
      );
      for (const p of filtered) {
        const li = document.createElement("li");
        li.textContent = p.name;
        li.addEventListener("click", () => {
          // Remove .exe extension
          let name = p.name;
          if (name.toLowerCase().endsWith(".exe")) {
            name = name.slice(0, -4);
          }
          close(name);
        });
        list.appendChild(li);
      }
    }

    searchInput.addEventListener("input", (e) => {
      renderProcessList(e.target.value);
    });

    function close(result) {
      overlay.remove();
      document.removeEventListener("keydown", onKeydown);
      resolve(result);
    }

    overlay.querySelector(".btn-cancel").addEventListener("click", () => close(null));

    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) close(null);
    });

    function onKeydown(e) {
      if (e.key === "Escape") close(null);
    }
    document.addEventListener("keydown", onKeydown);

    renderProcessList();
    document.body.appendChild(overlay);
    searchInput.focus();
  });
}

// ── Collect config from UI ──
function collectConfig() {
  const collected = {
    search: {},
    folders: {},
    apps: {},
    timestamp: {
      format: getTimestampFormat(),
      position: document.querySelector('input[name="ts-position"]:checked').value,
    },
  };

  // Search
  for (const row of document.querySelectorAll("#search-list .list-row")) {
    const key = row.querySelector(".key-input").value.trim();
    const url = row.querySelectorAll("input[type='text']")[1].value.trim();
    if (key) collected.search[key] = url;
  }

  // Folders
  for (const row of document.querySelectorAll("#folders-list .list-row")) {
    const key = row.querySelector(".key-input").value.trim();
    const path = row.querySelector(".path-input").value.trim();
    if (key) collected.folders[key] = path;
  }

  // Apps
  for (const row of document.querySelectorAll("#apps-list .list-row")) {
    const key = row.querySelector(".key-input").value.trim();
    const process = row.querySelector(".process-input").value.trim();
    const command = row.querySelector(".command-input").value.trim();
    if (key) {
      if (command) {
        collected.apps[key] = { process, command };
      } else {
        collected.apps[key] = process;
      }
    }
  }

  return collected;
}

// ── Apply / Reset / Defaults ──
document.getElementById("btn-apply").addEventListener("click", async () => {
  const newConfig = collectConfig();
  try {
    await invoke("save_config", { config: newConfig });
    config = newConfig;
  } catch (e) {
    alert("保存に失敗しました:\n" + e);
  }
});

document.getElementById("btn-reset").addEventListener("click", async () => {
  await loadConfig();
});

document.getElementById("btn-defaults").addEventListener("click", async () => {
  try {
    config = await invoke("default_config");
    renderConfig();
  } catch (e) {
    console.error("初期値の取得に失敗:", e);
  }
});

// ── Kanata controls ──
document.getElementById("btn-kanata-start").addEventListener("click", async () => {
  try {
    await invoke("start_kanata");
    await refreshKanataStatus();
  } catch (e) {
    alert("kanata の開始に失敗しました:\n" + e);
  }
});

document.getElementById("btn-kanata-stop").addEventListener("click", async () => {
  try {
    await invoke("stop_kanata");
    await refreshKanataStatus();
  } catch (e) {
    alert("kanata の停止に失敗しました:\n" + e);
  }
});

document.getElementById("btn-kanata-restart").addEventListener("click", async () => {
  try {
    await invoke("restart_kanata");
    await refreshKanataStatus();
  } catch (e) {
    alert("kanata の再起動に失敗しました:\n" + e);
  }
});

async function refreshKanataStatus() {
  try {
    const status = await invoke("get_kanata_status");
    updateKanataUI(status.running);
  } catch (e) {
    updateKanataUI(false);
  }
}

function updateKanataUI(running) {
  const dots = document.querySelectorAll(".status-dot");
  const text = document.getElementById("kanata-status-text");
  const footerText = document.getElementById("footer-kanata-text");

  dots.forEach((dot) => {
    dot.classList.toggle("running", running);
  });
  text.textContent = running ? "実行中" : "停止中";
  footerText.textContent = running ? "kanata: 実行中" : "kanata: 停止中";

  // ボタンの有効/無効を切り替え
  document.getElementById("btn-kanata-start").disabled = running;
  document.getElementById("btn-kanata-stop").disabled = !running;
  document.getElementById("btn-kanata-restart").disabled = !running;
}

// Listen for status changes from backend
listen("kanata-status-changed", (event) => {
  updateKanataUI(event.payload);
});

// ── Autostart checkbox ──
const autostartCheckbox = document.getElementById("opt-autostart");
autostartCheckbox.addEventListener("change", async () => {
  try {
    await invoke("set_autostart_enabled", { enabled: autostartCheckbox.checked });
  } catch (e) {
    console.error("自動起動の切り替えに失敗:", e);
    autostartCheckbox.checked = !autostartCheckbox.checked;
  }
});

async function loadAutostart() {
  try {
    autostartCheckbox.checked = await invoke("get_autostart_enabled");
  } catch (e) {
    console.error("自動起動状態の取得に失敗:", e);
  }
}

// ── Utility ──
function escapeHtml(str) {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

// ── Initialize ──
async function loadKeyBindings() {
  try {
    keyBindings = await invoke("get_key_bindings");
  } catch (e) {
    console.error("キーバインドの読み込みに失敗:", e);
  }
}

async function init() {
  await loadKeyBindings();
  await loadConfig();
  await refreshKanataStatus();
  await loadAutostart();
}

init();
