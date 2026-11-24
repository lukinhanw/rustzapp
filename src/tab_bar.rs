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

/// "Tamanho do texto" de Acessibilidade do Windows (`TextScaleFactor`, 100-225%)
/// como fator (1.0 = 100%). Fora do Windows, ou sem o valor, 1.0.
pub fn windows_text_scale() -> f64 {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        const HKEY_CURRENT_USER: isize = 0x8000_0001_u32 as i32 as isize;
        const RRF_RT_REG_DWORD: u32 = 0x0000_0010;

        #[link(name = "advapi32")]
        extern "system" {
            fn RegGetValueW(
                hkey: isize,
                sub_key: *const u16,
                value: *const u16,
                flags: u32,
                kind: *mut u32,
                data: *mut c_void,
                data_len: *mut u32,
            ) -> i32;
        }

        let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let key = wide(r"Software\Microsoft\Accessibility");
        let name = wide("TextScaleFactor");
        let mut value: u32 = 0;
        let mut len = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut value as *mut u32 as *mut c_void,
                &mut len,
            )
        };
        if status == 0 && (100..=225).contains(&value) {
            return f64::from(value) / 100.0;
        }
    }
    1.0
}

pub fn set_maximized_script(maximized: bool) -> String {
    format!("window.setMaximized({maximized});")
}

pub fn lock_state_script(has_password: bool, locked: bool) -> String {
    format!("window.setLockState({{\"hasPassword\":{has_password},\"locked\":{locked}}});")
}

pub fn lock_error_script(message: &str) -> String {
    let json = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".to_string());
    format!("window.lockError({json});")
}

pub const PASSWORD_SAVED_SCRIPT: &str = "window.passwordSaved();";

pub const TAB_BAR_HTML: &str = r##"<!DOCTYPE html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<style>
  /* Folga igual (--gap) em cima, embaixo, nas laterais e entre itens.
     --bar-h é a altura da faixa da barra na janela (TAB_BAR_HEIGHT, 40px
     lógicos). O app compensa o "Tamanho do texto" do Windows com zoom nesta
     WebView, então 1px CSS = 1px lógico. Não usar 100vh: com escala de texto o
     WebView2 arredonda o viewport para cima e a base da barra ficaria cortada. */
  :root {
    --bar-h: 40px;
    --gap: 6px;
    --item: calc(var(--bar-h) - 2 * var(--gap));
  }
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
  /* Espaçamento uniforme: 6px em cima, embaixo, nas laterais e entre itens
     (barra de 40px, itens de 28px). A linha divisória é desenhada por dentro
     (box-shadow) para não roubar altura da centralização. */
  .bar {
    display: flex;
    align-items: center;
    gap: var(--gap);
    height: var(--bar-h);
    padding: 0 0 0 var(--gap);
    box-shadow: inset 0 -1px 0 #222d34;
    background: #111b21;
  }
  .app-logo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--item);
    height: var(--item);
    flex-shrink: 0;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: var(--gap);
    flex: 0 1 auto;
    min-width: 0;
    height: 100%;
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
    height: var(--item);
    max-width: 200px;
    min-width: 90px;
    padding: 0 6px 0 10px;
    border-radius: 6px;
    background: #182229;
    color: #8696a0;
    cursor: pointer;
    flex-shrink: 0;
    border: 1px solid transparent;
    transition: background 0.12s ease, color 0.12s ease;
  }
  .tab:hover {
    background: #1f2c34;
    color: #d1d7db;
  }
  .tab.active {
    background: #202c33;
    color: #e9edef;
    border-color: #00a884;
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
    height: calc(var(--item) - 6px);
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
    width: var(--item);
    height: var(--item);
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
  /* Área clicável vai até a borda/canto da janela (bom com a janela
     maximizada); só o fundo do hover respeita as margens de 6px. */
  .win-controls {
    display: flex;
    align-items: stretch;
    height: 100%;
    margin-left: calc(var(--gap) / -2);
    flex-shrink: 0;
  }
  .win-btn {
    position: relative;
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
    transition: color 0.1s ease;
  }
  .win-btn::before {
    content: "";
    position: absolute;
    inset: var(--gap) calc(var(--gap) / 2);
    border-radius: 6px;
    background: transparent;
    transition: background 0.1s ease;
  }
  .win-btn.close-win {
    width: calc(46px + var(--gap) / 2);
  }
  .win-btn.close-win::before {
    inset: var(--gap) var(--gap) var(--gap) calc(var(--gap) / 2);
  }
  .win-btn:hover {
    color: #ffffff;
  }
  .win-btn:hover::before {
    background: #2a3942;
  }
  .win-btn.close-win:hover::before {
    background: #e81123;
  }
  .win-btn svg {
    position: relative;
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
  #lock-btn svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
  /* Bloqueado: a barra mostra só arrastar e os botões da janela (nada de nomes
     de abas ou contadores); o resto da janela é a tela de senha. */
  body.locked #tabs,
  body.locked #new-tab,
  body.locked #reload-tab,
  body.locked #lock-btn {
    visibility: hidden;
  }
  /* Tela de senha: só existe com a WebView da barra esticada na janela toda. */
  .overlay {
    display: none;
    position: fixed;
    top: var(--bar-h);
    left: 0;
    right: 0;
    bottom: 0;
    align-items: center;
    justify-content: center;
    background: #0b141a;
  }
  .overlay.show {
    display: flex;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 320px;
    padding: 28px;
    border: 1px solid #222d34;
    border-radius: 12px;
    background: #111b21;
    text-align: center;
  }
  .card svg {
    align-self: center;
    width: 40px;
    height: 40px;
    fill: none;
    stroke: #00a884;
    stroke-width: 1.6;
  }
  .card h2 {
    margin: 4px 0 0;
    font-size: 18px;
    font-weight: 600;
  }
  .hint {
    margin: 0 0 6px;
    color: #8696a0;
    font-size: 12px;
  }
  .card input {
    height: 36px;
    padding: 0 10px;
    border: 1px solid #2a3942;
    border-radius: 8px;
    background: #202c33;
    color: #e9edef;
    font-family: inherit;
    font-size: 13px;
    outline: none;
    user-select: text;
    -webkit-user-select: text;
  }
  .card input:focus {
    border-color: #00a884;
  }
  .error {
    min-height: 16px;
    color: #f15c6d;
    font-size: 12px;
  }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .actions button {
    height: 32px;
    padding: 0 14px;
    border: none;
    border-radius: 8px;
    font-family: inherit;
    font-size: 13px;
    white-space: nowrap;
    cursor: pointer;
  }
  .primary {
    background: #00a884;
    color: #111b21;
    font-weight: 600;
  }
  .primary:hover {
    background: #06cf9c;
  }
  .secondary {
    background: #202c33;
    color: #d1d7db;
  }
  .secondary:hover {
    background: #2a3942;
  }
  .danger {
    margin-right: auto;
    background: transparent;
    color: #f15c6d;
  }
  .hidden {
    display: none !important;
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
    <button id="lock-btn" class="icon-btn" title="Definir senha para exibir o WhatsApp">
      <svg viewBox="0 0 24 24"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>
    </button>
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
  <div id="overlay" class="overlay">
    <form id="pw-form" class="card" autocomplete="off">
      <svg viewBox="0 0 24 24"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>
      <h2 id="pw-title"></h2>
      <p id="pw-hint" class="hint"></p>
      <input id="pw-current" type="password" placeholder="Senha">
      <input id="pw-new" type="password" placeholder="Nova senha">
      <input id="pw-confirm" type="password" placeholder="Repita a nova senha">
      <div id="pw-error" class="error"></div>
      <div class="actions">
        <button type="button" id="pw-remove" class="danger">Remover senha</button>
        <button type="button" id="pw-cancel" class="secondary">Cancelar</button>
        <button type="submit" id="pw-ok" class="primary"></button>
      </div>
    </form>
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

  // ---- senha para exibir o WhatsApp ----
  // Modos da tela: "unlock" (bloqueado), "setup" (definir a primeira senha),
  // "manage" (alterar ou remover). O app estica esta WebView na janela toda e
  // esconde o WhatsApp enquanto a tela aparece.
  const lockBtn = document.getElementById("lock-btn");
  const overlayEl = document.getElementById("overlay");
  const pwForm = document.getElementById("pw-form");
  const pwTitle = document.getElementById("pw-title");
  const pwHint = document.getElementById("pw-hint");
  const pwCurrent = document.getElementById("pw-current");
  const pwNew = document.getElementById("pw-new");
  const pwConfirm = document.getElementById("pw-confirm");
  const pwError = document.getElementById("pw-error");
  const pwRemove = document.getElementById("pw-remove");
  const pwCancel = document.getElementById("pw-cancel");
  const pwOk = document.getElementById("pw-ok");
  const MODES = {
    unlock: { title: "WhatsApp bloqueado", hint: "Digite a senha para ver suas conversas.", ok: "Desbloquear", fields: [pwCurrent], current: "Senha" },
    setup: { title: "Definir senha", hint: "Ela será pedida ao abrir o app e depois de clicar no cadeado.", ok: "Salvar", fields: [pwNew, pwConfirm], cancel: true },
    manage: { title: "Alterar senha", hint: "Confirme a senha atual para trocar ou remover.", ok: "Salvar", fields: [pwCurrent, pwNew, pwConfirm], current: "Senha atual", cancel: true, remove: true },
  };
  let lockState = { hasPassword: false, locked: false };
  let pwMode = null;

  function showPw(mode) {
    const m = MODES[mode];
    pwMode = mode;
    pwTitle.textContent = m.title;
    pwHint.textContent = m.hint;
    pwOk.textContent = m.ok;
    pwCurrent.placeholder = m.current || "Senha";
    for (const input of [pwCurrent, pwNew, pwConfirm]) {
      input.value = "";
      input.classList.toggle("hidden", !m.fields.includes(input));
    }
    pwCancel.classList.toggle("hidden", !m.cancel);
    pwRemove.classList.toggle("hidden", !m.remove);
    pwError.textContent = "";
    document.body.classList.toggle("locked", mode === "unlock");
    overlayEl.classList.add("show");
    m.fields[0].focus();
  }

  function hidePw() {
    pwMode = null;
    for (const input of [pwCurrent, pwNew, pwConfirm]) input.value = "";
    overlayEl.classList.remove("show");
    document.body.classList.remove("locked");
  }

  function openPw(mode) {
    showPw(mode);
    send({ cmd: "overlay", on: true });
  }

  function cancelPw() {
    if (pwMode === "unlock") return;
    hidePw();
    send({ cmd: "overlay", on: false });
  }

  lockBtn.addEventListener("click", () => {
    if (lockState.hasPassword) send({ cmd: "lock" });
    else openPw("setup");
  });
  lockBtn.addEventListener("contextmenu", () => {
    if (lockState.hasPassword) openPw("manage");
  });
  pwCancel.addEventListener("click", cancelPw);
  pwForm.addEventListener("keydown", (e) => {
    if (e.key === "Escape") cancelPw();
  });
  pwRemove.addEventListener("click", () => {
    send({ cmd: "set_password", current: pwCurrent.value, new: "" });
  });
  pwForm.addEventListener("submit", (e) => {
    e.preventDefault();
    if (pwMode === "unlock") {
      send({ cmd: "unlock", password: pwCurrent.value });
      return;
    }
    if (!pwNew.value) {
      window.lockError("Digite a nova senha.");
      return;
    }
    if (pwNew.value !== pwConfirm.value) {
      window.lockError("As senhas não conferem.");
      return;
    }
    send({ cmd: "set_password", current: pwCurrent.value, new: pwNew.value });
  });

  window.setLockState = function(state) {
    lockState = state;
    lockBtn.title = state.hasPassword
      ? "Bloquear agora (clique direito: alterar ou remover a senha)"
      : "Definir senha para exibir o WhatsApp";
    if (state.locked) showPw("unlock");
    else if (pwMode === "unlock") hidePw();
  };

  window.lockError = function(message) {
    pwError.textContent = message;
    const first = MODES[pwMode] && MODES[pwMode].fields[0];
    if (first) {
      first.focus();
      first.select();
    }
  };

  window.passwordSaved = hidePw;

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

        assert_eq!(lock_state_script(true, false), r#"window.setLockState({"hasPassword":true,"locked":false});"#);
        // a mensagem vai como string JSON: aspas e quebras não escapam do script
        assert_eq!(lock_error_script("a\"b\n"), r#"window.lockError("a\"b\n");"#);
    }

    #[test]
    fn css_bar_height_matches_window_strip() {
        let expected = format!("--bar-h: {}px;", TAB_BAR_HEIGHT);
        assert!(TAB_BAR_HTML.contains(&expected), "atualize --bar-h no CSS para {expected}");
    }
}
