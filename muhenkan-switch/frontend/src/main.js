const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ── State ──
let config = null;       // Current config from backend
let guiSettings = {};    // GUI-only settings

// ── Available dispatch keys (must match kbd file) ──
const DISPATCH_KEYS = [
  "1", "2", "3", "4", "5",
  "q", "r", "t", "g",
  "a", "w", "e", "s", "d", "f",
];

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
  // Format
  const formatPreset = document.getElementById("ts-format-preset");
  const formatCustom = document.getElementById("ts-format-custom");
  const format = config.timestamp.format;

  const formatOption = Array.from(formatPreset.options).find((o) => o.value === format);
  if (formatOption) {
    formatPreset.value = format;
    formatCustom.classList.add("hidden");
  } else {
    formatPreset.value = "custom";
    formatCustom.value = format;
    formatCustom.classList.remove("hidden");
  }

  // Delimiter
  const delimPreset = document.getElementById("ts-delimiter-preset");
  const delimCustom = document.getElementById("ts-delimiter-custom");
  const delimiter = config.timestamp.delimiter ?? "_";

  const delimOption = Array.from(delimPreset.options).find((o) => o.value === delimiter);
  if (delimOption) {
    delimPreset.value = delimiter;
    delimCustom.classList.add("hidden");
  } else {
    delimPreset.value = "custom";
    delimCustom.value = delimiter;
    delimCustom.classList.remove("hidden");
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

function getTimestampDelimiter() {
  const preset = document.getElementById("ts-delimiter-preset").value;
  if (preset === "custom") {
    return document.getElementById("ts-delimiter-custom").value;
  }
  return preset;
}

async function updateTimestampPreview() {
  const format = getTimestampFormat();
  const delimiter = getTimestampDelimiter();
  const position = document.querySelector('input[name="ts-position"]:checked').value;
  try {
    const preview = await invoke("validate_timestamp_format", { format, delimiter, position });
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

document.getElementById("ts-delimiter-preset").addEventListener("change", (e) => {
  const customInput = document.getElementById("ts-delimiter-custom");
  if (e.target.value === "custom") {
    customInput.classList.remove("hidden");
    customInput.focus();
  } else {
    customInput.classList.add("hidden");
  }
  updateTimestampPreview();
});

document.getElementById("ts-delimiter-custom").addEventListener("input", () => {
  updateTimestampPreview();
});

document.querySelectorAll('input[name="ts-position"]').forEach((radio) => {
  radio.addEventListener("change", () => updateTimestampPreview());
});

// ── Dispatch key dropdown helper ──
function createDispatchKeySelect(selectedKey = "") {
  const select = document.createElement("select");
  select.className = "dispatch-key-select";
  select.title = "無変換+キー";

  const noneOpt = document.createElement("option");
  noneOpt.value = "";
  noneOpt.textContent = "—";
  select.appendChild(noneOpt);

  for (const k of DISPATCH_KEYS) {
    const opt = document.createElement("option");
    opt.value = k;
    opt.textContent = k.toUpperCase();
    select.appendChild(opt);
  }

  select.value = selectedKey || "";
  return select;
}

// ── Search engines ──
function renderSearchList() {
  const container = document.getElementById("search-list");
  container.innerHTML = "";
  for (const [name, entry] of Object.entries(config.search || {})) {
    addSearchRow(container, name, entry.url, entry.key || "");
  }
}

function addSearchRow(container, name = "", url = "", dispatchKey = "") {
  const row = document.createElement("div");
  row.className = "list-row";
  row.innerHTML = `
    <input type="text" class="key-input" placeholder="キー" value="${escapeHtml(name)}">
    <input type="text" placeholder="URL テンプレート ({query})" value="${escapeHtml(url)}">
    <button class="btn-remove" title="削除">&times;</button>
  `;
  // Insert dispatch key select before the first input
  const keySelect = createDispatchKeySelect(dispatchKey);
  row.insertBefore(keySelect, row.firstChild);
  row.querySelector(".btn-remove").addEventListener("click", () => row.remove());
  container.appendChild(row);
}

document.getElementById("btn-add-search").addEventListener("click", () => {
  addSearchRow(document.getElementById("search-list"));
});

// ── Folders ──
function renderFoldersList() {
  const container = document.getElementById("folders-list");
  container.innerHTML = "";
  for (const [name, entry] of Object.entries(config.folders || {})) {
    addFolderRow(container, name, entry.path, entry.key || "");
  }
}

function addFolderRow(container, name = "", path = "", dispatchKey = "") {
  const row = document.createElement("div");
  row.className = "list-row";
  row.innerHTML = `
    <input type="text" class="key-input" placeholder="キー" value="${escapeHtml(name)}">
    <input type="text" class="path-input" placeholder="パス (~/Documents)" value="${escapeHtml(path)}">
    <button class="btn-browse" title="参照">参照</button>
    <button class="btn-remove" title="削除">&times;</button>
  `;
  const keySelect = createDispatchKeySelect(dispatchKey);
  row.insertBefore(keySelect, row.firstChild);
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
  container.appendChild(row);
}

document.getElementById("btn-add-folder").addEventListener("click", () => {
  addFolderRow(document.getElementById("folders-list"));
});

// ── Apps ──
function renderAppsList() {
  const container = document.getElementById("apps-list");
  container.innerHTML = "";
  for (const [name, entry] of Object.entries(config.apps || {})) {
    addAppRow(container, name, entry.process, entry.command || "", entry.key || "");
  }
}

function addAppRow(container, name = "", process = "", command = "", dispatchKey = "") {
  const row = document.createElement("div");
  row.className = "list-row";
  row.innerHTML = `
    <input type="text" class="key-input" placeholder="キー" value="${escapeHtml(name)}">
    <input type="text" class="process-input" placeholder="プロセス名" value="${escapeHtml(process)}">
    <input type="text" class="command-input" placeholder="実行コマンド" value="${escapeHtml(command)}">
    <button class="btn-pick-process" title="プロセス選択">選択</button>
    <button class="btn-remove" title="削除">&times;</button>
  `;
  const keySelect = createDispatchKeySelect(dispatchKey);
  row.insertBefore(keySelect, row.firstChild);
  row.querySelector(".btn-remove").addEventListener("click", () => row.remove());
  row.querySelector(".btn-pick-process").addEventListener("click", async () => {
    const selected = await showProcessPicker();
    if (selected) {
      row.querySelector(".process-input").value = selected;
      row.querySelector(".command-input").value = selected.toLowerCase();
    }
  });
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

// ── Dispatch key duplicate validation ──
function validateDispatchKeys() {
  const usedKeys = {};
  for (const select of document.querySelectorAll(".dispatch-key-select")) {
    const key = select.value;
    if (!key) continue;
    if (usedKeys[key]) {
      return `ディスパッチキー "${key.toUpperCase()}" が重複しています`;
    }
    usedKeys[key] = true;
  }
  return null;
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
      delimiter: getTimestampDelimiter(),
    },
  };

  // Search
  for (const row of document.querySelectorAll("#search-list .list-row")) {
    const name = row.querySelector(".key-input").value.trim();
    const url = row.querySelectorAll("input[type='text']")[1].value.trim();
    const dispatchKey = row.querySelector(".dispatch-key-select").value;
    if (name) {
      const entry = { url };
      if (dispatchKey) entry.key = dispatchKey;
      collected.search[name] = entry;
    }
  }

  // Folders
  for (const row of document.querySelectorAll("#folders-list .list-row")) {
    const name = row.querySelector(".key-input").value.trim();
    const path = row.querySelector(".path-input").value.trim();
    const dispatchKey = row.querySelector(".dispatch-key-select").value;
    if (name) {
      const entry = { path };
      if (dispatchKey) entry.key = dispatchKey;
      collected.folders[name] = entry;
    }
  }

  // Apps
  for (const row of document.querySelectorAll("#apps-list .list-row")) {
    const name = row.querySelector(".key-input").value.trim();
    const process = row.querySelector(".process-input").value.trim();
    const command = row.querySelector(".command-input").value.trim();
    const dispatchKey = row.querySelector(".dispatch-key-select").value;
    if (name) {
      const entry = { process };
      if (dispatchKey) entry.key = dispatchKey;
      if (command) entry.command = command;
      collected.apps[name] = entry;
    }
  }

  return collected;
}

// ── Apply / Reset / Defaults ──
document.getElementById("btn-apply").addEventListener("click", async () => {
  try {
    // Client-side dispatch key validation
    const dupError = validateDispatchKeys();
    if (dupError) {
      alert(dupError);
      return;
    }

    const newConfig = collectConfig();
    console.log("[apply] saving config:", JSON.stringify(newConfig).slice(0, 200));
    await invoke("save_config", { config: newConfig });
    config = newConfig;

    // Brief save success indicator
    const btn = document.getElementById("btn-apply");
    const orig = btn.textContent;
    btn.textContent = "保存しました";
    setTimeout(() => { btn.textContent = orig; }, 1500);
  } catch (e) {
    console.error("[apply] error:", e);
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

// ── Kanata status ──
async function refreshKanataStatus() {
  try {
    const status = await invoke("get_kanata_status");
    updateKanataUI(status.running);
  } catch (e) {
    updateKanataUI(false);
  }
}

function updateKanataUI(running) {
  // Footer
  const footerDot = document.getElementById("footer-kanata-dot");
  const footerText = document.getElementById("footer-kanata-text");
  if (footerDot) footerDot.classList.toggle("running", running);
  if (footerText) footerText.textContent = running ? "キー割当（kanata）: 実行中" : "キー割当（kanata）: 停止中";
  // General tab
  const genDot = document.getElementById("general-kanata-dot");
  const genText = document.getElementById("general-kanata-text");
  if (genDot) genDot.classList.toggle("running", running);
  if (genText) genText.textContent = running ? "実行中" : "停止中";
}

listen("kanata-status-changed", (event) => {
  updateKanataUI(event.payload);
});

// ── General tab: help / install dir / quit ──
document.getElementById("btn-help").addEventListener("click", async () => {
  try {
    await invoke("open_help_window");
  } catch (e) {
    console.error("ヘルプウィンドウの表示に失敗:", e);
  }
});

document.getElementById("btn-github").addEventListener("click", async () => {
  const { open } = window.__TAURI__.shell;
  await open("https://github.com/kimushun1101/muhenkan-switch-rs");
});

document.getElementById("btn-open-dir").addEventListener("click", async () => {
  try {
    await invoke("open_install_dir");
  } catch (e) {
    alert("インストール先を開けませんでした:\n" + e);
  }
});

document.getElementById("btn-quit").addEventListener("click", async () => {
  await invoke("quit_app");
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
async function init() {
  await loadConfig();
  await refreshKanataStatus();
  await loadAutostart();

  // General tab info
  try {
    document.getElementById("app-version").textContent = "v" + await invoke("get_app_version");
  } catch (e) {
    console.error("バージョン情報の取得に失敗:", e);
  }
}

init();
