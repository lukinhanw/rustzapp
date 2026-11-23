use serde::Serialize;

pub const TAB_BAR_HEIGHT: f64 = 40.0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabViewState {
    pub id: u64,
    pub name: String,
    pub unread: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderPayload<'a> {
    active_id: u64,
    tabs: &'a [TabViewState],
}

pub fn render_tabs_script(active_id: u64, tabs: &[TabViewState]) -> String {
    let payload = RenderPayload { active_id, tabs };
    let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    format!("window.renderTabs({json});")
}

pub fn set_maximized_script(maximized: bool) -> String {
    format!("window.setMaximized({maximized});")
}

pub const TAB_BAR_HTML: &str = r##"<!DOCTYPE html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<style>
  * {
    box-sizing: border-box;
    user-select: none;
    -webkit-user-select: none;
  }
  html, body {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: #111b21;
    color: #e9edef;
    font-family: "Segoe UI", system-ui, -apple-system, sans-serif;
    font-size: 13px;
  }
  .bar {
    display: flex;
    align-items: center;
    height: 40px;
    padding: 0 0 0 10px;
    border-bottom: 1px solid #222d34;
    background: #111b21;
  }
  .app-logo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    margin-right: 8px;
    flex-shrink: 0;
  }
  .tabs {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    flex: 0 1 auto;
    min-width: 0;
    height: 100%;
    padding-top: 6px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 33px;
    max-width: 200px;
    min-width: 90px;
    padding: 0 8px 0 11px;
    border-radius: 8px 8px 0 0;
    background: #182229;
    color: #8696a0;
    cursor: pointer;
    flex-shrink: 0;
    border: 1px solid transparent;
    border-bottom: none;
    transition: background 0.12s ease, color 0.12s ease;
  }
  .tab:hover {
    background: #1f2c34;
    color: #d1d7db;
  }
  .tab.active {
    background: #202c33;
    color: #e9edef;
    border-color: #2a3942;
    box-shadow: inset 0 2px 0 #00a884;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #54656f;
    flex-shrink: 0;
  }
  .tab.active .dot {
    background: #00a884;
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    font-weight: 500;
  }
  .rename-input {
    width: 112px;
    height: 22px;
    border: 1px solid #00a884;
    border-radius: 4px;
    background: #111b21;
    color: #e9edef;
    font-family: inherit;
    font-size: 12px;
    padding: 0 5px;
    outline: none;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: #00a884;
    color: #111b21;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #8696a0;
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    flex-shrink: 0;
    padding: 0;
  }
  .close:hover {
    background: #374248;
    color: #e9edef;
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    margin-left: 6px;
    border: none;
    border-radius: 6px;
    background: #202c33;
    color: #d1d7db;
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    flex-shrink: 0;
  }
  .icon-btn:hover {
    background: #2a3942;
    color: #00a884;
  }
  .drag-spacer {
    flex: 1;
    min-width: 36px;
    height: 100%;
  }
  .reload-btn {
    margin-right: 6px;
  }
  .win-controls {
    display: flex;
    align-items: stretch;
    height: 100%;
    flex-shrink: 0;
  }
  .win-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 46px;
    height: 100%;
    border: none;
    background: transparent;
    color: #d1d7db;
    cursor: pointer;
    padding: 0;
    transition: background 0.1s ease, color 0.1s ease;
  }
  .win-btn:hover {
    background: #2a3942;
    color: #ffffff;
  }
  .win-btn.close-win:hover {
    background: #e81123;
    color: #ffffff;
  }
  .win-btn svg {
    width: 10px;
    height: 10px;
    stroke: currentColor;
    fill: none;
    stroke-width: 1.25;
  }
  .icon-restore {
    display: none;
  }
  body.maximized .icon-max {
    display: none;
  }
  body.maximized .icon-restore {
    display: block;
  }
</style>
</head>
<body>
  <div id="bar" class="bar">
    <div class="app-logo" title="WhatsApp Browser">
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
        <path fill="#00a884" d="M12 2a9.8 9.8 0 0 0-8.46 14.74L2 22l5.42-1.48A9.8 9.8 0 1 0 12 2z"/>
        <path fill="#e9edef" d="M16.56 14.4c-.24-.12-1.42-.7-1.64-.78-.22-.08-.38-.12-.54.12-.16.24-.62.78-.76.94-.14.16-.28.18-.52.06a6.58 6.58 0 0 1-1.94-1.2 7.26 7.26 0 0 1-1.34-1.66c-.14-.24-.01-.37.1-.49.11-.11.24-.28.36-.42.12-.14.16-.24.24-.4.08-.16.04-.3-.02-.42-.06-.12-.54-1.3-.74-1.78-.2-.47-.4-.4-.54-.41h-.46c-.16 0-.42.06-.64.3-.22.24-.84.82-.84 2s.86 2.32.98 2.48c.12.16 1.7 2.6 4.12 3.64.58.25 1.03.4 1.38.51.58.18 1.1.16 1.52.1.46-.07 1.42-.58 1.62-1.14.2-.56.2-1.04.14-1.14-.06-.1-.22-.16-.46-.28z"/>
      </svg>
    </div>
    <div id="tabs" class="tabs"></div>
    <button id="new-tab" class="icon-btn" title="Nova aba WhatsApp (cookies independentes)">+</button>
    <div class="drag-spacer" title="Arraste para mover a janela"></div>
    <button id="reload-tab" class="icon-btn reload-btn" title="Recarregar aba ativa">&#8635;</button>
    <div class="win-controls">
      <button id="btn-min" class="win-btn" title="Minimizar" aria-label="Minimizar">
        <svg viewBox="0 0 10 10"><line x1="0" y1="5" x2="10" y2="5"/></svg>
      </button>
      <button id="btn-max" class="win-btn" title="Maximizar / Restaurar" aria-label="Maximizar">
        <svg class="icon-max" viewBox="0 0 10 10"><rect x="0.6" y="0.6" width="8.8" height="8.8"/></svg>
        <svg class="icon-restore" viewBox="0 0 10 10">
          <path d="M2.5 2.5V0.6h6.9v6.9H7.5"/>
          <rect x="0.6" y="2.5" width="6.9" height="6.9"/>
        </svg>
      </button>
      <button id="btn-close" class="win-btn close-win" title="Fechar" aria-label="Fechar">
        <svg viewBox="0 0 10 10">
          <line x1="1" y1="1" x2="9" y2="9"/>
          <line x1="9" y1="1" x2="1" y2="9"/>
        </svg>
      </button>
    </div>
  </div>
<script>
  const barEl = document.getElementById("bar");
  const tabsEl = document.getElementById("tabs");
  const newTabBtn = document.getElementById("new-tab");
  const reloadBtn = document.getElementById("reload-tab");
  const minBtn = document.getElementById("btn-min");
  const maxBtn = document.getElementById("btn-max");
  const closeWinBtn = document.getElementById("btn-close");

  let editingId = null;
  let pendingState = null;
  let isMaximized = false;

  function send(payload) {
    if (window.ipc && typeof window.ipc.postMessage === "function") {
      window.ipc.postMessage(JSON.stringify(payload));
    }
  }

  document.addEventListener("contextmenu", (e) => e.preventDefault());

  tabsEl.addEventListener("wheel", (e) => {
    if (e.deltaY !== 0) {
      tabsEl.scrollLeft += e.deltaY;
      e.preventDefault();
    }
  }, { passive: false });

  barEl.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    if (e.target.closest(".tab, button, input")) return;
    if (!isMaximized && e.clientY <= 4) {
      send({ cmd: "resize_north" });
      return;
    }
    if (e.detail === 2) {
      send({ cmd: "toggle_maximize" });
      return;
    }
    send({ cmd: "drag_window" });
  });

  newTabBtn.addEventListener("click", () => send({ cmd: "new_tab" }));
  reloadBtn.addEventListener("click", () => send({ cmd: "reload" }));
  minBtn.addEventListener("click", () => send({ cmd: "minimize" }));
  maxBtn.addEventListener("click", () => send({ cmd: "toggle_maximize" }));
  closeWinBtn.addEventListener("click", () => send({ cmd: "close_window" }));

  window.setMaximized = function(maximized) {
    isMaximized = Boolean(maximized);
    document.body.classList.toggle("maximized", isMaximized);
  };

  function startRename(tab, labelEl) {
    editingId = tab.id;
    const input = document.createElement("input");
    input.type = "text";
    input.className = "rename-input";
    input.value = tab.name;
    input.maxLength = 32;

    let finished = false;
    function finish(commit) {
      if (finished) return;
      finished = true;
      editingId = null;
      const value = input.value.trim();
      if (commit && value && value !== tab.name) {
        send({ cmd: "rename", id: tab.id, name: value });
      } else if (pendingState) {
        const state = pendingState;
        pendingState = null;
        window.renderTabs(state);
      }
    }

    input.addEventListener("keydown", (e) => {
      e.stopPropagation();
      if (e.key === "Enter") {
        finish(true);
      } else if (e.key === "Escape") {
        finish(false);
      }
    });
    input.addEventListener("blur", () => finish(true));
    input.addEventListener("click", (e) => e.stopPropagation());

    labelEl.replaceWith(input);
    input.focus();
    input.select();
  }

  window.renderTabs = function(state) {
    if (!state || !Array.isArray(state.tabs)) return;
    if (editingId !== null) {
      pendingState = state;
      return;
    }

    tabsEl.textContent = "";
    for (const tab of state.tabs) {
      const item = document.createElement("div");
      item.className = "tab" + (tab.id === state.activeId ? " active" : "");
      item.title = "Clique duplo para renomear";

      const dot = document.createElement("span");
      dot.className = "dot";
      item.appendChild(dot);

      const label = document.createElement("span");
      label.className = "label";
      label.textContent = tab.name;
      item.appendChild(label);

      if (tab.unread) {
        const badge = document.createElement("span");
        badge.className = "badge";
        badge.textContent = tab.unread;
        item.appendChild(badge);
      }

      const closeBtn = document.createElement("button");
      closeBtn.className = "close";
      closeBtn.textContent = "\u00d7";
      closeBtn.title = "Fechar aba";
      closeBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        send({ cmd: "close", id: tab.id });
      });
      item.appendChild(closeBtn);

      item.addEventListener("click", () => {
        if (tab.id !== state.activeId) {
          send({ cmd: "switch", id: tab.id });
        }
      });

      item.addEventListener("dblclick", (e) => {
        e.stopPropagation();
        startRename(tab, label);
      });

      tabsEl.appendChild(item);
    }
  };

  window.addEventListener("DOMContentLoaded", () => {
    send({ cmd: "ready" });
  });
</script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_serialize_cleanly() {
        let tabs = vec![TabViewState {
            id: 1,
            name: "Trabalho".to_string(),
            unread: Some("2".to_string()),
        }];
        let tabs_js = render_tabs_script(1, &tabs);
        assert!(tabs_js.starts_with("window.renderTabs("));
        assert!(tabs_js.contains("\"activeId\":1"));
        assert!(tabs_js.contains("\"name\":\"Trabalho\""));

        assert_eq!(set_maximized_script(true), "window.setMaximized(true);");
        assert_eq!(set_maximized_script(false), "window.setMaximized(false);");
    }
}
