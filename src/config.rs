use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

pub const WHATSAPP_URL: &str = "https://web.whatsapp.com/";
pub const APP_DIR_NAME: &str = "WhatsAppBrowser";
const MAX_TAB_NAME_LEN: usize = 32;

/// Retorna o diretório base de dados da aplicação em `%LOCALAPPDATA%\WhatsAppBrowser`.
pub fn app_data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join(APP_DIR_NAME)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTab {
    pub id: u64,
    pub name: String,
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub next_id: u64,
    pub active_id: u64,
    pub tabs: Vec<SavedTab>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            next_id: 2,
            active_id: 1,
            tabs: vec![SavedTab {
                id: 1,
                name: "WhatsApp 1".to_string(),
                profile_id: "wa_1".to_string(),
            }],
        }
    }
}

impl AppConfig {
    /// Carrega a configuração salva ou inicializa com uma aba padrão.
    pub fn load(path: &Path) -> Self {
        let Ok(content) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(mut cfg) = serde_json::from_str::<Self>(&content) else {
            return Self::default();
        };
        if cfg.tabs.is_empty() {
            return Self::default();
        }
        let max_id = cfg.tabs.iter().map(|t| t.id).max().unwrap_or(0);
        if cfg.next_id <= max_id {
            cfg.next_id = max_id + 1;
        }
        if !cfg.tabs.iter().any(|t| t.id == cfg.active_id) {
            cfg.active_id = cfg.tabs[0].id;
        }
        cfg
    }

    /// Salva a configuração em disco de forma segura.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)?;
        if fs::rename(&tmp_path, path).is_err() {
            fs::write(path, &json)?;
            let _ = fs::remove_file(&tmp_path);
        }
        Ok(())
    }

    /// Cria uma nova aba com `profile_id` inédito (sem reutilizar cookies antigos).
    pub fn add_tab(&mut self) -> SavedTab {
        let id = self.next_id;
        self.next_id += 1;
        let tab = SavedTab {
            id,
            name: format!("WhatsApp {id}"),
            profile_id: format!("wa_{id}"),
        };
        self.active_id = id;
        self.tabs.push(tab.clone());
        tab
    }

    /// Remove uma aba pelo ID. Se era a última aba, cria uma nova aba limpa e a retorna.
    pub fn remove_tab(&mut self, id: u64) -> Option<SavedTab> {
        let Some(pos) = self.tabs.iter().position(|t| t.id == id) else {
            return None;
        };
        self.tabs.remove(pos);

        if self.tabs.is_empty() {
            return Some(self.add_tab());
        }

        if self.active_id == id {
            let next_pos = pos.min(self.tabs.len() - 1);
            self.active_id = self.tabs[next_pos].id;
        }
        None
    }

    /// Renomeia uma aba existente, removendo espaços extras e limitando a 32 caracteres.
    pub fn rename_tab(&mut self, id: u64, new_name: &str) -> bool {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return false;
        }
        let clamped: String = trimmed.chars().take(MAX_TAB_NAME_LEN).collect();
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.name = clamped;
            true
        } else {
            false
        }
    }
}

/// Permite estritamente o WhatsApp Web (e recursos internos `about:blank`, `data:`, `blob:https://web.whatsapp.com/`).
pub fn is_allowed_url(raw: &str) -> bool {
    if raw == "about:blank" {
        return true;
    }
    let Ok(parsed) = Url::parse(raw) else {
        return false;
    };
    match parsed.scheme() {
        "https" => parsed.host_str() == Some("web.whatsapp.com"),
        "blob" => parsed.path().starts_with("https://web.whatsapp.com/"),
        "data" => true,
        _ => false,
    }
}

/// Link clicado numa conversa que deve abrir no navegador padrão do Windows.
/// Só `http`, `https` e `mailto` (fora do próprio WhatsApp Web): uma mensagem não
/// pode disparar `file:`, `javascript:` ou protocolos de outros programas.
/// Retorna a URL normalizada pelo parser.
pub fn external_link(raw: &str) -> Option<String> {
    if is_allowed_url(raw) {
        return None;
    }
    let parsed = Url::parse(raw).ok()?;
    match parsed.scheme() {
        "http" | "https" if parsed.host_str().is_some_and(|h| !h.is_empty()) => Some(parsed.into()),
        "mailto" => Some(parsed.into()),
        _ => None,
    }
}

/// Extrai a contagem de mensagens não lidas do título do WhatsApp Web, ex: `"(3) WhatsApp"` -> `Some("3")`.
pub fn parse_unread_count(title: &str) -> Option<String> {
    let trimmed = title.trim_start();
    let rest = trimmed.strip_prefix('(')?;
    let end = rest.find(')')?;
    let count = rest[..end].trim();
    if count.is_empty() {
        return None;
    }
    let valid = count.chars().all(|c| c.is_ascii_digit() || c == '+')
        && count.chars().any(|c| c.is_ascii_digit());
    if valid {
        Some(count.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_allowlist_restricts_to_whatsapp_web() {
        assert!(is_allowed_url("https://web.whatsapp.com/"));
        assert!(is_allowed_url("https://web.whatsapp.com/send?phone=123"));
        assert!(is_allowed_url("about:blank"));
        assert!(is_allowed_url("data:text/plain;base64,SGVsbG8="));
        assert!(is_allowed_url(
            "blob:https://web.whatsapp.com/550e8400-e29b-41d4-a716-446655440000"
        ));

        assert!(!is_allowed_url("http://web.whatsapp.com/"));
        assert!(!is_allowed_url("https://www.whatsapp.com/"));
        assert!(!is_allowed_url("https://google.com/"));
        assert!(!is_allowed_url("https://web.whatsapp.com.evil.com/"));
        assert!(!is_allowed_url("blob:https://evil.com/123"));
        assert!(!is_allowed_url("javascript:alert(1)"));
        assert!(!is_allowed_url("file:///C:/Windows/System32"));
        assert!(!is_allowed_url("not a url"));
    }

    #[test]
    fn external_links_only_web_and_mail() {
        assert_eq!(external_link("https://google.com/a?b=1").as_deref(), Some("https://google.com/a?b=1"));
        assert_eq!(external_link("http://example.com").as_deref(), Some("http://example.com/"));
        assert_eq!(external_link("https://www.whatsapp.com/faq").as_deref(), Some("https://www.whatsapp.com/faq"));
        assert!(external_link("mailto:ana@example.com").is_some());

        assert!(external_link("https://web.whatsapp.com/").is_none());
        assert!(external_link("about:blank").is_none());
        assert!(external_link("javascript:alert(1)").is_none());
        assert!(external_link("file:///C:/Windows/System32/cmd.exe").is_none());
        assert!(external_link("ms-settings:privacy").is_none());
        assert!(external_link("blob:https://evil.com/123").is_none());
        assert!(external_link("not a url").is_none());
    }

    #[test]
    fn parse_unread_extracts_counts() {
        assert_eq!(parse_unread_count("(3) WhatsApp"), Some("3".to_string()));
        assert_eq!(
            parse_unread_count(" (99+) WhatsApp "),
            Some("99+".to_string())
        );
        assert_eq!(parse_unread_count("WhatsApp"), None);
        assert_eq!(parse_unread_count("() WhatsApp"), None);
        assert_eq!(parse_unread_count("(abc) WhatsApp"), None);
    }

    #[test]
    fn config_add_remove_and_rename_tabs() {
        let mut cfg = AppConfig::default();
        assert_eq!(cfg.tabs.len(), 1);
        assert_eq!(cfg.active_id, 1);

        let t2 = cfg.add_tab();
        assert_eq!(t2.id, 2);
        assert_eq!(t2.profile_id, "wa_2");
        assert_eq!(cfg.active_id, 2);

        assert!(cfg.rename_tab(2, "  Trabalho  "));
        assert_eq!(cfg.tabs[1].name, "Trabalho");
        assert!(!cfg.rename_tab(2, "   "));

        let long_name = "a".repeat(50);
        assert!(cfg.rename_tab(2, &long_name));
        assert_eq!(cfg.tabs[1].name.chars().count(), MAX_TAB_NAME_LEN);

        // Remover aba 2 volta a ativa para 1
        assert!(cfg.remove_tab(2).is_none());
        assert_eq!(cfg.tabs.len(), 1);
        assert_eq!(cfg.active_id, 1);

        // Remover a última aba cria automaticamente uma aba limpa com novo profile_id
        let replacement = cfg.remove_tab(1).expect("should spawn fresh tab");
        assert_eq!(replacement.id, 3);
        assert_eq!(replacement.profile_id, "wa_3");
        assert_eq!(cfg.active_id, 3);
        assert_eq!(cfg.tabs.len(), 1);
    }

    #[test]
    fn config_save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("wa_browser_test_{}", std::process::id()));
        let path = dir.join("tabs.json");
        let _ = fs::remove_dir_all(&dir);

        let mut cfg = AppConfig::default();
        cfg.add_tab();
        cfg.rename_tab(2, "Pessoal");
        cfg.save(&path).expect("save config");

        let loaded = AppConfig::load(&path);
        assert_eq!(loaded, cfg);

        let _ = fs::remove_dir_all(&dir);
    }
}
