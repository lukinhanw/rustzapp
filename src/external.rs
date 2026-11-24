//! Abre links externos no navegador padrão do sistema.

/// Entrega a URL ao Windows (`ShellExecuteW` com o verbo "open"), que usa o
/// navegador padrão configurado pelo usuário. Quem chama já validou o esquema
/// (`config::external_link`).
#[cfg(target_os = "windows")]
pub fn open_in_default_browser(url: &str) {
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
    let target = wide(url);
    // Valores <= 32 indicam erro; não há o que fazer além de ignorar.
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn open_in_default_browser(url: &str) {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(url).spawn();
}
