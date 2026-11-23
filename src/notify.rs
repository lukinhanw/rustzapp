use serde::Deserialize;
use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum WhatsAppIpcCommand {
    WebNotification {
        notif_id: u64,
        title: String,
        #[serde(default)]
        body: String,
        #[serde(default)]
        silent: bool,
    },
}

/// Script injetado no WhatsApp Web para habilitar `Notification` e encaminhar
/// alertas nativos para o host Rust via `window.ipc.postMessage`.
pub const NOTIFICATION_SHIM_JS: &str = r#"
(function () {
  if (window.location.origin !== "https://web.whatsapp.com") return;
  if (window.__rustzappNotifInstalled) return;
  window.__rustzappNotifInstalled = true;

  let nextId = 1;
  const activeMap = new Map();
  const MAX_ACTIVE = 30;

  function postNotif(payload) {
    try {
      if (window.ipc && typeof window.ipc.postMessage === "function") {
        window.ipc.postMessage(JSON.stringify(payload));
      }
    } catch (_) {}
  }

  window.__rustzappClickNotif = function (id) {
    const inst = activeMap.get(id);
    if (!inst) return;
    try {
      const ev = new Event("click");
      if (typeof inst.onclick === "function") {
        inst.onclick(ev);
      }
      inst.dispatchEvent(ev);
    } catch (_) {}
  };

  class RustZappNotification extends EventTarget {
    static get permission() {
      return "granted";
    }

    static get maxActions() {
      return 0;
    }

    static requestPermission(cb) {
      if (typeof cb === "function") {
        try { cb("granted"); } catch (_) {}
      }
      return Promise.resolve("granted");
    }

    constructor(title, options) {
      super();
      const opts = options || {};
      this.title = String(title ?? "");
      this.body = opts.body ? String(opts.body) : "";
      this.icon = opts.icon ? String(opts.icon) : "";
      this.tag = opts.tag ? String(opts.tag) : "";
      this.data = opts.data ?? null;
      this.silent = Boolean(opts.silent);
      this.onclick = null;
      this.onclose = null;
      this.onshow = null;
      this.onerror = null;

      this._id = nextId++;
      activeMap.set(this._id, this);
      if (activeMap.size > MAX_ACTIVE) {
        const oldestKey = activeMap.keys().next().value;
        activeMap.delete(oldestKey);
      }

      postNotif({
        cmd: "web_notification",
        notif_id: this._id,
        title: this.title,
        body: this.body,
        silent: this.silent,
      });

      setTimeout(() => {
        const ev = new Event("show");
        if (typeof this.onshow === "function") {
          try { this.onshow(ev); } catch (_) {}
        }
        this.dispatchEvent(ev);
      }, 0);
    }

    close() {
      activeMap.delete(this._id);
      const ev = new Event("close");
      if (typeof this.onclose === "function") {
        try { this.onclose(ev); } catch (_) {}
      }
      this.dispatchEvent(ev);
    }
  }

  try {
    Object.defineProperty(window, "Notification", {
      value: RustZappNotification,
      configurable: true,
      writable: true,
    });
  } catch (_) {
    window.Notification = RustZappNotification;
  }

  if (navigator.permissions && typeof navigator.permissions.query === "function") {
    const origQuery = navigator.permissions.query.bind(navigator.permissions);
    navigator.permissions.query = function (desc) {
      if (desc && desc.name === "notifications") {
        return Promise.resolve({
          name: "notifications",
          state: "granted",
          onchange: null,
          addEventListener: function () {},
          removeEventListener: function () {},
          dispatchEvent: function () { return true; },
        });
      }
      return origQuery(desc);
    };
  }
})();
"#;

#[cfg(target_os = "windows")]
const TOAST_AUMID: &str = "RustZapp.Notifications";

#[cfg(target_os = "windows")]
const EMBEDDED_ICON: &[u8] = include_bytes!("../assets/icon.ico");

/// Garante o arquivo de ícone local e o registro do `AppUserModelId` para notificações Toast no Windows.
#[cfg(target_os = "windows")]
pub fn ensure_toast_registered(data_dir: &Path) -> PathBuf {
    let _ = std::fs::create_dir_all(data_dir);
    let icon_path = data_dir.join("rustzapp.ico");
    let needs_write = match std::fs::metadata(&icon_path) {
        Ok(meta) => meta.len() != EMBEDDED_ICON.len() as u64,
        Err(_) => true,
    };
    if needs_write {
        let _ = std::fs::write(&icon_path, EMBEDDED_ICON);
    }

    register_aumid_in_registry(TOAST_AUMID, "RustZapp", &icon_path);
    icon_path
}

#[cfg(target_os = "windows")]
fn register_aumid_in_registry(aumid: &str, display_name: &str, icon_path: &Path) {
    use std::ffi::c_void;

    const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
    const KEY_WRITE: u32 = 0x0002_0006;
    const REG_SZ: u32 = 1;
    const REG_DWORD: u32 = 4;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegCreateKeyExW(
            hkey: isize,
            lp_sub_key: *const u16,
            reserved: u32,
            lp_class: *const u16,
            dw_options: u32,
            sam_desired: u32,
            lp_security_attributes: *const c_void,
            phk_result: *mut isize,
            lpdw_disposition: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            hkey: isize,
            lp_value_name: *const u16,
            reserved: u32,
            dw_type: u32,
            lp_data: *const u8,
            cb_data: u32,
        ) -> i32;
        fn RegCloseKey(hkey: isize) -> i32;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let subkey = to_wide(&format!(r"Software\Classes\AppUserModelId\{aumid}"));
    let mut hkey: isize = 0;

    unsafe {
        let rc = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            std::ptr::null_mut(),
        );
        if rc != 0 || hkey == 0 {
            return;
        }

        let name_key = to_wide("DisplayName");
        let name_val = to_wide(display_name);
        RegSetValueExW(
            hkey,
            name_key.as_ptr(),
            0,
            REG_SZ,
            name_val.as_ptr() as *const u8,
            (name_val.len() * 2) as u32,
        );

        let icon_key = to_wide("IconUri");
        let icon_val = to_wide(&icon_path.to_string_lossy());
        RegSetValueExW(
            hkey,
            icon_key.as_ptr(),
            0,
            REG_SZ,
            icon_val.as_ptr() as *const u8,
            (icon_val.len() * 2) as u32,
        );

        let show_key = to_wide("ShowInSettings");
        let show_val: u32 = 1;
        RegSetValueExW(
            hkey,
            show_key.as_ptr(),
            0,
            REG_DWORD,
            &show_val as *const u32 as *const u8,
            std::mem::size_of::<u32>() as u32,
        );

        RegCloseKey(hkey);
    }
}

/// Exibe uma notificação Toast nativa no Windows 10/11 e executa `on_click` ao clicar.
#[cfg(target_os = "windows")]
pub fn show_windows_toast(
    icon_path: &Path,
    tab_name: Option<&str>,
    title: &str,
    body: &str,
    silent: bool,
    on_click: impl Fn() + Send + Sync + 'static,
) {
    use std::sync::Arc;
    use tauri_winrt_notification::{Duration, IconCrop, Sound, Toast};

    let clean_title = if title.trim().is_empty() {
        "WhatsApp"
    } else {
        title.trim()
    };
    let clean_body = body.trim();
    let click_cb = Arc::new(on_click);

    let build_toast = |aumid: &str| {
        let cb = Arc::clone(&click_cb);
        let mut toast = Toast::new(aumid)
            .title(clean_title)
            .text1(clean_body)
            .icon(icon_path, IconCrop::Circular, "RustZapp")
            .duration(Duration::Short)
            .sound(if silent { None } else { Some(Sound::Default) })
            .on_activated(move |_| {
                cb();
                Ok(())
            });
        if let Some(tab) = tab_name {
            toast = toast.text2(tab);
        }
        toast
    };

    if build_toast(TOAST_AUMID).show().is_err() {
        let _ = build_toast(Toast::POWERSHELL_APP_ID).show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whatsapp_notification_ipc_and_rejects_bar_commands() {
        let json = r#"{"cmd":"web_notification","notif_id":7,"title":"Maria","body":"Oi!","silent":false}"#;
        let parsed: WhatsAppIpcCommand = serde_json::from_str(json).expect("valid notification");
        assert_eq!(
            parsed,
            WhatsAppIpcCommand::WebNotification {
                notif_id: 7,
                title: "Maria".to_string(),
                body: "Oi!".to_string(),
                silent: false,
            }
        );

        let disallowed = r#"{"cmd":"close_window"}"#;
        assert!(serde_json::from_str::<WhatsAppIpcCommand>(disallowed).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn writes_toast_icon_to_data_dir() {
        let dir = std::env::temp_dir().join(format!("rustzapp_notify_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let icon_path = ensure_toast_registered(&dir);
        assert!(icon_path.exists());
        let len = std::fs::metadata(&icon_path).unwrap().len();
        assert_eq!(len, EMBEDDED_ICON.len() as u64);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
