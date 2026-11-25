//! Entrega links e arquivos ao Windows (navegador padrão, programa padrão).

use std::path::{Path, PathBuf};

/// Abre um link no navegador padrão. Quem chama já validou o esquema
/// (`config::external_link`).
pub fn open_in_default_browser(url: &str) {
    shell_open(url);
}

/// Abre um arquivo baixado no programa padrão (PDF no leitor, foto no
/// visualizador...). Executáveis e scripts recebidos numa conversa NÃO são
/// executados: abre-se só a pasta onde foram salvos.
pub fn open_downloaded_file(path: &Path) {
    if is_risky_to_open(path) {
        if let Some(dir) = path.parent() {
            shell_open(&dir.to_string_lossy());
        }
    } else {
        shell_open(&path.to_string_lossy());
    }
}

/// Pasta escolhida no último "Salvar como" (vale enquanto o app estiver aberto).
static LAST_SAVE_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Nome sugerido para salvar: o WebView2 já desambiguou o caminho padrão com
/// " (N)" quando o arquivo existe em Downloads ("foto (3).jpg"). Para o usuário,
/// sugere o nome original; a janela avisa se for sobrescrever.
pub fn suggested_file_name(default_path: &Path) -> String {
    let name = default_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "download".to_string());
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name.as_str(), ""),
    };
    if let Some(open) = stem.rfind(" (") {
        let inner = &stem[open + 2..];
        if let Some(num) = inner.strip_suffix(')') {
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) && open > 0 {
                return format!("{}{}", &stem[..open], ext);
            }
        }
    }
    name
}

/// Mostra a janela "Salvar como" do Windows. `None` = o usuário cancelou.
#[cfg(target_os = "windows")]
pub fn ask_save_path(owner_hwnd: isize, default_path: &Path) -> Option<PathBuf> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStringExt;

    #[repr(C)]
    struct OpenFileNameW {
        l_struct_size: u32,
        hwnd_owner: isize,
        h_instance: *mut c_void,
        lpstr_filter: *const u16,
        lpstr_custom_filter: *mut u16,
        n_max_cust_filter: u32,
        n_filter_index: u32,
        lpstr_file: *mut u16,
        n_max_file: u32,
        lpstr_file_title: *mut u16,
        n_max_file_title: u32,
        lpstr_initial_dir: *const u16,
        lpstr_title: *const u16,
        flags: u32,
        n_file_offset: u16,
        n_file_extension: u16,
        lpstr_def_ext: *const u16,
        l_cust_data: isize,
        lpfn_hook: *mut c_void,
        lp_template_name: *const u16,
        pv_reserved: *mut c_void,
        dw_reserved: u32,
        flags_ex: u32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetSaveFileNameW(ofn: *mut OpenFileNameW) -> i32;
    }

    const OFN_OVERWRITEPROMPT: u32 = 0x0000_0002;
    const OFN_NOCHANGEDIR: u32 = 0x0000_0008;
    const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
    const OFN_EXPLORER: u32 = 0x0008_0000;
    const MAX_PATH_BUF: usize = 32_768;

    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();

    let name = suggested_file_name(default_path);
    let ext = Path::new(&name)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();

    let initial_dir = LAST_SAVE_DIR
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .filter(|d| d.is_dir())
        .or_else(|| default_path.parent().map(Path::to_path_buf))
        .unwrap_or_default();

    // Filtro: "Arquivo PDF (*.pdf)" + "Todos os arquivos (*.*)", separado por NULs.
    let mut filter = String::new();
    if !ext.is_empty() {
        filter.push_str(&format!("Arquivo {} (*.{})\0*.{}\0", ext.to_uppercase(), ext, ext));
    }
    filter.push_str("Todos os arquivos (*.*)\0*.*\0\0");
    let filter_w: Vec<u16> = filter.encode_utf16().collect();

    let mut file_buf = vec![0u16; MAX_PATH_BUF];
    for (i, c) in name.encode_utf16().take(MAX_PATH_BUF - 1).enumerate() {
        file_buf[i] = c;
    }
    let initial_dir_w = wide(&initial_dir.to_string_lossy());
    let title_w = wide("Salvar arquivo do WhatsApp");
    let def_ext_w = wide(&ext);

    let mut ofn = OpenFileNameW {
        l_struct_size: std::mem::size_of::<OpenFileNameW>() as u32,
        hwnd_owner: owner_hwnd,
        h_instance: std::ptr::null_mut(),
        lpstr_filter: filter_w.as_ptr(),
        lpstr_custom_filter: std::ptr::null_mut(),
        n_max_cust_filter: 0,
        n_filter_index: 1,
        lpstr_file: file_buf.as_mut_ptr(),
        n_max_file: MAX_PATH_BUF as u32,
        lpstr_file_title: std::ptr::null_mut(),
        n_max_file_title: 0,
        lpstr_initial_dir: initial_dir_w.as_ptr(),
        lpstr_title: title_w.as_ptr(),
        flags: OFN_OVERWRITEPROMPT | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST | OFN_EXPLORER,
        n_file_offset: 0,
        n_file_extension: 0,
        lpstr_def_ext: if ext.is_empty() { std::ptr::null() } else { def_ext_w.as_ptr() },
        l_cust_data: 0,
        lpfn_hook: std::ptr::null_mut(),
        lp_template_name: std::ptr::null(),
        pv_reserved: std::ptr::null_mut(),
        dw_reserved: 0,
        flags_ex: 0,
    };

    if unsafe { GetSaveFileNameW(&mut ofn) } == 0 {
        return None; // cancelado (ou erro do diálogo)
    }
    let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
    let chosen = PathBuf::from(std::ffi::OsString::from_wide(&file_buf[..len]));
    if let (Some(dir), Ok(mut last)) = (chosen.parent(), LAST_SAVE_DIR.lock()) {
        *last = Some(dir.to_path_buf());
    }
    Some(chosen)
}

/// Extensões que o Windows executaria em vez de "abrir" como documento.
pub fn is_risky_to_open(path: &Path) -> bool {
    const RISKY: &[&str] = &[
        "exe", "com", "scr", "pif", "bat", "cmd", "ps1", "psm1", "vbs", "vbe", "js", "jse", "wsf",
        "wsh", "hta", "msi", "msp", "msix", "msixbundle", "appx", "appxbundle", "lnk", "url", "reg",
        "cpl", "jar", "inf", "scf", "application", "gadget", "chm",
    ];
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => RISKY.iter().any(|r| r.eq_ignore_ascii_case(ext)),
        None => true, // sem extensão: não dá para saber o que o Windows faria
    }
}

/// `ShellExecuteW` com o verbo "open": o Windows escolhe o programa padrão.
#[cfg(target_os = "windows")]
fn shell_open(target: &str) {
    use std::ffi::c_void;

    const SW_SHOWNORMAL: i32 = 1;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show_cmd: i32,
        ) -> isize;
    }

    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let verb = wide("open");
    let file = wide(target);
    // Valores <= 32 indicam erro; não há o que fazer além de ignorar.
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn shell_open(target: &str) {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(target).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_name_drops_webview_counter() {
        let d = |p: &str| suggested_file_name(Path::new(p));
        assert_eq!(d("C:/D/relatorio (7).pdf"), "relatorio.pdf");
        assert_eq!(d("C:/D/foto (12).jpeg"), "foto.jpeg");
        assert_eq!(d("C:/D/sem numero.pdf"), "sem numero.pdf");
        assert_eq!(d("C:/D/versao (final).pdf"), "versao (final).pdf");
        assert_eq!(d("C:/D/(2).pdf"), "(2).pdf");
        assert_eq!(d("C:/D/arquivo (3)"), "arquivo");
    }

    #[test]
    fn documents_open_but_programs_do_not() {
        for ok in ["a.pdf", "Foto.JPG", "nota.docx", "audio.ogg", "planilha.xlsx", "x.zip"] {
            assert!(!is_risky_to_open(Path::new(ok)), "{ok}");
        }
        for bad in ["setup.exe", "run.BAT", "x.ps1", "a.msi", "atalho.lnk", "f.hta", "semextensao"] {
            assert!(is_risky_to_open(Path::new(bad)), "{bad}");
        }
    }
}
