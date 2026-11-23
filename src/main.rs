#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod badge;
mod config;
mod tab_bar;

#[cfg(target_os = "windows")]
use badge::create_badge_icon;
use badge::{compute_total_unread, format_badge_label, render_badge_rgba, render_whatsapp_icon_rgba};
use config::{app_data_dir, is_allowed_url, parse_unread_count, AppConfig, SavedTab, WHATSAPP_URL};
use serde::Deserialize;
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::sync::{Arc, OnceLock};
use tab_bar::{
    render_tabs_script, set_maximized_script, TabViewState, TAB_BAR_HEIGHT, TAB_BAR_HTML,
};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Icon, ResizeDirection, Window, WindowBuilder},
};
use wry::{
    dpi::{LogicalPosition, LogicalSize},
    NewWindowResponse, Rect, WebContext, WebView, WebViewBuilder,
};

#[cfg(target_os = "windows")]
use tao::platform::windows::{
    EventLoopBuilderExtWindows, IconExtWindows, WindowBuilderExtWindows, WindowExtWindows,
};
#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;

#[cfg(target_os = "windows")]
mod win_taskbar {
    use crate::badge::BADGE_SIZE;
    use std::ffi::c_void;

    const IMAGE_ICON: u32 = 1;
    const LR_SHARED: u32 = 0x8000;
    const SM_CXICON: i32 = 11;
    const SM_CYICON: i32 = 12;
    const SM_CXSMICON: i32 = 49;
    const SM_CYSMICON: i32 = 50;
    const GCLP_HICON: i32 = -14;
    const GCLP_HICONSM: i32 = -34;
    const WM_SETICON: u32 = 0x0080;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;

    pub const WM_TRAY_CALLBACK: u32 = 0x8001;
    const NIM_ADD: u32 = 0x0000_0000;
    const NIM_MODIFY: u32 = 0x0000_0001;
    const NIM_DELETE: u32 = 0x0000_0002;
    const NIF_MESSAGE: u32 = 0x0000_0001;
    const NIF_ICON: u32 = 0x0000_0002;
    const NIF_TIP: u32 = 0x0000_0004;
    const BI_RGB: u32 = 0;
    const DIB_RGB_COLORS: u32 = 0;

    #[repr(C)]
    pub struct WinMsg {
        pub hwnd: isize,
        pub message: u32,
        pub w_param: usize,
        pub l_param: isize,
        pub time: u32,
        pub pt_x: i32,
        pub pt_y: i32,
    }

    #[repr(C)]
    struct BitmapInfoHeader {
        bi_size: u32,
        bi_width: i32,
        bi_height: i32,
        bi_planes: u16,
        bi_bit_count: u16,
        bi_compression: u32,
        bi_size_image: u32,
        bi_x_pels_per_meter: i32,
        bi_y_pels_per_meter: i32,
        bi_clr_used: u32,
        bi_clr_important: u32,
    }

    #[repr(C)]
    struct IconInfo {
        f_icon: i32,
        x_hotspot: u32,
        y_hotspot: u32,
        hbm_mask: *mut c_void,
        hbm_color: *mut c_void,
    }

    #[repr(C)]
    struct NotifyIconDataW {
        cb_size: u32,
        hwnd: isize,
        u_id: u32,
        u_flags: u32,
        u_callback_message: u32,
        h_icon: *mut c_void,
        sz_tip: [u16; 128],
        dw_state: u32,
        dw_state_mask: u32,
        sz_info: [u16; 256],
        u_timeout_or_version: u32,
        sz_info_title: [u16; 64],
        dw_info_flags: u32,
        guid_item: [u8; 16],
        h_balloon_icon: *mut c_void,
    }

    #[cfg(target_pointer_width = "64")]
    const _: () = assert!(std::mem::size_of::<NotifyIconDataW>() == 976);

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> *mut c_void;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateDIBSection(
            hdc: *mut c_void,
            pbmi: *const BitmapInfoHeader,
            usage: u32,
            ppv_bits: *mut *mut c_void,
            h_section: *mut c_void,
            offset: u32,
        ) -> *mut c_void;
        fn CreateBitmap(
            n_width: i32,
            n_height: i32,
            n_planes: u32,
            n_bits_per_pel: u32,
            lpv_bits: *const c_void,
        ) -> *mut c_void;
        fn DeleteObject(ho: *mut c_void) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn Shell_NotifyIconW(dw_message: u32, lp_data: *const NotifyIconDataW) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMetrics(index: i32) -> i32;
        fn LoadImageW(
            hinst: *mut c_void,
            name: *const u16,
            img_type: u32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> *mut c_void;
        fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn RegisterWindowMessageW(lp_string: *const u16) -> u32;
        fn CreateIconIndirect(piconinfo: *const IconInfo) -> *mut c_void;
        fn DestroyIcon(h_icon: *mut c_void) -> i32;
        fn FindWindowExW(
            hwnd_parent: isize,
            hwnd_child_after: isize,
            lpsz_class: *const u16,
            lpsz_window: *const u16,
        ) -> isize;
        fn ShowWindow(hwnd: isize, n_cmd_show: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        #[cfg(target_pointer_width = "64")]
        fn SetClassLongPtrW(hwnd: isize, index: i32, new_long: isize) -> isize;
    }

    pub fn focus_existing_instance(class_name: &str, exclude_hwnd: isize) -> bool {
        const SW_RESTORE: i32 = 9;
        let wide_class: Vec<u16> = class_name.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let mut prev = 0isize;
            loop {
                let hwnd = FindWindowExW(0, prev, wide_class.as_ptr(), std::ptr::null());
                if hwnd == 0 {
                    return false;
                }
                if hwnd != exclude_hwnd {
                    ShowWindow(hwnd, SW_RESTORE);
                    SetForegroundWindow(hwnd);
                    return true;
                }
                prev = hwnd;
            }
        }
    }

    pub fn register_taskbar_created_msg() -> u32 {
        let wide: Vec<u16> = "TaskbarCreated".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { RegisterWindowMessageW(wide.as_ptr()) }
    }

    pub fn apply_window_and_class_icons(hwnd: isize, resource_id: u16) {
        unsafe {
            let hinst = GetModuleHandleW(std::ptr::null());
            if hinst.is_null() {
                return;
            }
            let res_ptr = resource_id as usize as *const u16;
            let cx_big = GetSystemMetrics(SM_CXICON).max(32);
            let cy_big = GetSystemMetrics(SM_CYICON).max(32);
            let cx_sm = GetSystemMetrics(SM_CXSMICON).max(16);
            let cy_sm = GetSystemMetrics(SM_CYSMICON).max(16);

            let hicon_big = LoadImageW(hinst, res_ptr, IMAGE_ICON, cx_big, cy_big, LR_SHARED);
            let hicon_sm = LoadImageW(hinst, res_ptr, IMAGE_ICON, cx_sm, cy_sm, LR_SHARED);

            #[cfg(target_pointer_width = "64")]
            {
                if !hicon_big.is_null() {
                    SetClassLongPtrW(hwnd, GCLP_HICON, hicon_big as isize);
                }
                if !hicon_sm.is_null() {
                    SetClassLongPtrW(hwnd, GCLP_HICONSM, hicon_sm as isize);
                }
            }

            if !hicon_big.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon_big as isize);
            }
            if !hicon_sm.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon_sm as isize);
            }
        }
    }

    fn create_hicon_from_rgba(rgba: &[u8]) -> *mut c_void {
        let size = BADGE_SIZE as i32;
        if rgba.len() != (BADGE_SIZE * BADGE_SIZE * 4) as usize {
            return std::ptr::null_mut();
        }
        unsafe {
            let bmi = BitmapInfoHeader {
                bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                bi_width: size,
                bi_height: -size, // top-down DIB
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: 0,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            let hbm_color = CreateDIBSection(
                std::ptr::null_mut(),
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if hbm_color.is_null() || bits.is_null() {
                if !hbm_color.is_null() {
                    DeleteObject(hbm_color);
                }
                return std::ptr::null_mut();
            }

            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, rgba.len());
            for (src_px, dst_px) in rgba.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
                dst_px[0] = src_px[2]; // B
                dst_px[1] = src_px[1]; // G
                dst_px[2] = src_px[0]; // R
                dst_px[3] = src_px[3]; // A
            }

            let hbm_mask = CreateBitmap(size, size, 1, 1, std::ptr::null());
            let info = IconInfo {
                f_icon: 1,
                x_hotspot: 0,
                y_hotspot: 0,
                hbm_mask,
                hbm_color,
            };
            let hicon = CreateIconIndirect(&info);
            if !hbm_mask.is_null() {
                DeleteObject(hbm_mask);
            }
            DeleteObject(hbm_color);
            hicon
        }
    }

    pub struct TrayIcon {
        hwnd: isize,
        uid: u32,
        hicon: *mut c_void,
    }

    impl TrayIcon {
        fn build_nid(&self, flags: u32, tooltip: &str, hicon: *mut c_void) -> NotifyIconDataW {
            let mut nid = NotifyIconDataW {
                cb_size: std::mem::size_of::<NotifyIconDataW>() as u32,
                hwnd: self.hwnd,
                u_id: self.uid,
                u_flags: flags,
                u_callback_message: WM_TRAY_CALLBACK,
                h_icon: hicon,
                sz_tip: [0; 128],
                dw_state: 0,
                dw_state_mask: 0,
                sz_info: [0; 256],
                u_timeout_or_version: 0,
                sz_info_title: [0; 64],
                dw_info_flags: 0,
                guid_item: [0; 16],
                h_balloon_icon: std::ptr::null_mut(),
            };
            for (dst, ch) in nid.sz_tip.iter_mut().take(127).zip(tooltip.encode_utf16()) {
                *dst = ch;
            }
            nid
        }

        pub fn add(hwnd: isize, tooltip: &str, rgba: &[u8]) -> Self {
            let hicon = create_hicon_from_rgba(rgba);
            let tray = Self {
                hwnd,
                uid: 1,
                hicon,
            };
            let nid = tray.build_nid(NIF_MESSAGE | NIF_ICON | NIF_TIP, tooltip, hicon);
            unsafe {
                Shell_NotifyIconW(NIM_ADD, &nid);
            }
            tray
        }

        pub fn update(&mut self, tooltip: &str, rgba: &[u8]) {
            let next_hicon = create_hicon_from_rgba(rgba);
            let icon_to_use = if next_hicon.is_null() {
                self.hicon
            } else {
                next_hicon
            };
            let nid = self.build_nid(NIF_MESSAGE | NIF_ICON | NIF_TIP, tooltip, icon_to_use);
            let ok = unsafe { Shell_NotifyIconW(NIM_MODIFY, &nid) };
            if ok == 0 {
                unsafe {
                    Shell_NotifyIconW(NIM_ADD, &nid);
                }
            }
            if !next_hicon.is_null() && !self.hicon.is_null() {
                unsafe {
                    DestroyIcon(self.hicon);
                }
            }
            if !next_hicon.is_null() {
                self.hicon = next_hicon;
            }
        }

        pub fn re_add(&mut self, tooltip: &str, rgba: &[u8]) {
            let next_hicon = create_hicon_from_rgba(rgba);
            if !next_hicon.is_null() {
                if !self.hicon.is_null() {
                    unsafe {
                        DestroyIcon(self.hicon);
                    }
                }
                self.hicon = next_hicon;
            }
            let nid = self.build_nid(NIF_MESSAGE | NIF_ICON | NIF_TIP, tooltip, self.hicon);
            unsafe {
                Shell_NotifyIconW(NIM_ADD, &nid);
            }
        }

        pub fn remove(&mut self) {
            let nid = self.build_nid(0, "", std::ptr::null_mut());
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &nid);
                if !self.hicon.is_null() {
                    DestroyIcon(self.hicon);
                    self.hicon = std::ptr::null_mut();
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum IpcCommand {
    Ready,
    NewTab,
    Switch { id: u64 },
    Close { id: u64 },
    Rename { id: u64, name: String },
    Reload,
    DragWindow,
    ResizeNorth,
    Minimize,
    ToggleMaximize,
    CloseWindow,
}

#[derive(Debug)]
enum UserEvent {
    Ipc(IpcCommand),
    TitleChanged { tab_id: u64, title: String },
    #[cfg(target_os = "windows")]
    RestoreWindow,
    #[cfg(target_os = "windows")]
    ReRegisterTray,
}

struct TabRuntime {
    webview: WebView,
    unread: Option<String>,
}

fn compute_bounds(window: &Window) -> (Rect, Rect) {
    let logical = window
        .inner_size()
        .to_logical::<f64>(window.scale_factor());
    let width = logical.width.max(1.0);
    let height = logical.height.max(TAB_BAR_HEIGHT + 1.0);

    let bar_bounds = Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: LogicalSize::new(width, TAB_BAR_HEIGHT).into(),
    };
    let content_bounds = Rect {
        position: LogicalPosition::new(0.0, TAB_BAR_HEIGHT).into(),
        size: LogicalSize::new(width, (height - TAB_BAR_HEIGHT).max(1.0)).into(),
    };
    (bar_bounds, content_bounds)
}

fn create_whatsapp_webview(
    window: &Window,
    web_context: &mut WebContext,
    proxy: EventLoopProxy<UserEvent>,
    saved: &SavedTab,
    bounds: Rect,
    visible: bool,
) -> wry::Result<WebView> {
    let tab_id = saved.id;
    let mut builder = WebViewBuilder::new_with_web_context(web_context)
        .with_url(WHATSAPP_URL)
        .with_bounds(bounds)
        .with_visible(visible)
        .with_background_color((17, 27, 33, 255))
        .with_devtools(cfg!(debug_assertions))
        .with_navigation_handler(|url| is_allowed_url(&url))
        .with_new_window_req_handler(|_url, _features| NewWindowResponse::Deny)
        .with_document_title_changed_handler(move |title| {
            let _ = proxy.send_event(UserEvent::TitleChanged { tab_id, title });
        });

    #[cfg(target_os = "windows")]
    {
        builder = builder.with_profile_name(&saved.profile_id);
    }

    builder.build_as_child(window)
}

fn sync_tab_bar(tab_bar: &WebView, config: &AppConfig, runtimes: &HashMap<u64, TabRuntime>) {
    let views: Vec<TabViewState> = config
        .tabs
        .iter()
        .map(|tab| TabViewState {
            id: tab.id,
            name: tab.name.clone(),
            unread: runtimes.get(&tab.id).and_then(|rt| rt.unread.clone()),
        })
        .collect();
    let script = render_tabs_script(config.active_id, &views);
    let _ = tab_bar.evaluate_script(&script);
}

fn sync_maximized(tab_bar: &WebView, maximized: bool) {
    let _ = tab_bar.evaluate_script(&set_maximized_script(maximized));
}

fn update_notifications(
    window: &Window,
    runtimes: &HashMap<u64, TabRuntime>,
    last_badge: &mut Option<String>,
    #[cfg(target_os = "windows")] tray_icon: &mut win_taskbar::TrayIcon,
) {
    let (total, has_plus) = compute_total_unread(runtimes.values().map(|rt| rt.unread.as_deref()));

    let tooltip = if total > 0 {
        let suffix = if has_plus { "+" } else { "" };
        window.set_title(&format!("({total}{suffix}) WhatsApp Browser"));
        format!("WhatsApp Browser ({total}{suffix} não lidas)")
    } else {
        window.set_title("WhatsApp Browser");
        "WhatsApp Browser".to_string()
    };

    let next_badge = format_badge_label(total, has_plus);
    if next_badge != *last_badge {
        #[cfg(target_os = "windows")]
        {
            if let Some(ref label) = next_badge {
                let overlay = create_badge_icon(label);
                window.set_overlay_icon(overlay.as_ref());
                let rgba = render_badge_rgba(label);
                tray_icon.update(&tooltip, &rgba);
            } else {
                window.set_overlay_icon(None);
                let rgba = render_whatsapp_icon_rgba();
                tray_icon.update(&tooltip, &rgba);
            }
        }
        *last_badge = next_badge;
    }
}

fn main() -> wry::Result<()> {
    let data_dir = app_data_dir();
    let config_path = data_dir.join("tabs.json");
    let mut config = AppConfig::load(&config_path);
    let _ = config.save(&config_path);

    let mut event_loop_builder = EventLoopBuilder::<UserEvent>::with_user_event();

    #[cfg(target_os = "windows")]
    let hook_proxy: Arc<OnceLock<EventLoopProxy<UserEvent>>> = Arc::new(OnceLock::new());
    #[cfg(target_os = "windows")]
    {
        let taskbar_created_msg = win_taskbar::register_taskbar_created_msg();
        let hook_proxy_ref = Arc::clone(&hook_proxy);
        event_loop_builder.with_msg_hook(move |raw_msg| {
            let msg = unsafe { &*(raw_msg as *const win_taskbar::WinMsg) };
            if msg.message == win_taskbar::WM_TRAY_CALLBACK {
                let ev = (msg.l_param as usize) & 0xFFFF;
                if ev == 0x0202 || ev == 0x0203 {
                    if let Some(p) = hook_proxy_ref.get() {
                        let _ = p.send_event(UserEvent::RestoreWindow);
                    }
                }
            } else if taskbar_created_msg != 0 && msg.message == taskbar_created_msg {
                if let Some(p) = hook_proxy_ref.get() {
                    let _ = p.send_event(UserEvent::ReRegisterTray);
                }
            }
            false
        });
    }

    let event_loop = event_loop_builder.build();
    let proxy = event_loop.create_proxy();
    #[cfg(target_os = "windows")]
    let _ = hook_proxy.set(proxy.clone());

    #[cfg(target_os = "windows")]
    let app_icon = Icon::from_resource(1, None).ok();

    let mut window_builder = WindowBuilder::new()
        .with_title("WhatsApp Browser")
        .with_decorations(false)
        .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(520.0, 400.0));

    #[cfg(target_os = "windows")]
    {
        window_builder = window_builder
            .with_window_classname("WhatsAppBrowserMainWindow")
            .with_undecorated_shadow(true)
            .with_window_icon(app_icon.clone())
            .with_taskbar_icon(app_icon);
    }

    let window = window_builder
        .build(&event_loop)
        .expect("falha ao criar janela principal");

    #[cfg(target_os = "windows")]
    win_taskbar::apply_window_and_class_icons(window.hwnd(), 1);

    #[cfg(target_os = "windows")]
    let mut tray_icon = win_taskbar::TrayIcon::add(
        window.hwnd(),
        "WhatsApp Browser",
        &render_whatsapp_icon_rgba(),
    );

    let (bar_bounds, content_bounds) = compute_bounds(&window);
    let mut web_context = WebContext::new(Some(data_dir.join("webview_data")));

    let ipc_proxy = proxy.clone();
    let mut bar_builder = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_html(TAB_BAR_HTML)
        .with_bounds(bar_bounds)
        .with_background_color((17, 27, 33, 255))
        .with_devtools(false)
        .with_ipc_handler(move |request| {
            if let Ok(cmd) = serde_json::from_str::<IpcCommand>(request.body()) {
                let _ = ipc_proxy.send_event(UserEvent::Ipc(cmd));
            }
        });

    #[cfg(target_os = "windows")]
    {
        bar_builder = bar_builder.with_profile_name("ui_bar");
    }

    let tab_bar = match bar_builder.build_as_child(&window) {
        Ok(wv) => wv,
        Err(err) => {
            #[cfg(target_os = "windows")]
            {
                tray_icon.remove();
                if win_taskbar::focus_existing_instance("WhatsAppBrowserMainWindow", window.hwnd())
                {
                    return Ok(());
                }
            }
            return Err(err);
        }
    };

    let mut runtimes: HashMap<u64, TabRuntime> = HashMap::new();
    for saved in &config.tabs {
        let visible = saved.id == config.active_id;
        let webview = create_whatsapp_webview(
            &window,
            &mut web_context,
            proxy.clone(),
            saved,
            content_bounds,
            visible,
        )?;
        runtimes.insert(
            saved.id,
            TabRuntime {
                webview,
                unread: None,
            },
        );
    }

    let mut last_badge: Option<String> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _keep_context_alive = &web_context;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                let _ = config.save(&config_path);
                #[cfg(target_os = "windows")]
                tray_icon.remove();
                runtimes.clear();
                std::process::exit(0);
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                if new_size.width == 0 || new_size.height == 0 {
                    return;
                }
                let (new_bar_bounds, new_content_bounds) = compute_bounds(&window);
                let _ = tab_bar.set_bounds(new_bar_bounds);
                for rt in runtimes.values() {
                    let _ = rt.webview.set_bounds(new_content_bounds);
                }
                sync_maximized(&tab_bar, window.is_maximized());
            }
            Event::WindowEvent {
                event: WindowEvent::ScaleFactorChanged { .. },
                ..
            } => {
                let (new_bar_bounds, new_content_bounds) = compute_bounds(&window);
                let _ = tab_bar.set_bounds(new_bar_bounds);
                for rt in runtimes.values() {
                    let _ = rt.webview.set_bounds(new_content_bounds);
                }
                sync_maximized(&tab_bar, window.is_maximized());
            }
            Event::UserEvent(UserEvent::TitleChanged { tab_id, title }) => {
                if let Some(rt) = runtimes.get_mut(&tab_id) {
                    rt.unread = parse_unread_count(&title);
                    update_notifications(
                        &window,
                        &runtimes,
                        &mut last_badge,
                        #[cfg(target_os = "windows")]
                        &mut tray_icon,
                    );
                    sync_tab_bar(&tab_bar, &config, &runtimes);
                }
            }
            #[cfg(target_os = "windows")]
            Event::UserEvent(UserEvent::RestoreWindow) => {
                window.set_minimized(false);
                window.set_focus();
            }
            #[cfg(target_os = "windows")]
            Event::UserEvent(UserEvent::ReRegisterTray) => {
                let (rgba, tip) = if let Some(ref label) = last_badge {
                    (
                        render_badge_rgba(label),
                        format!("WhatsApp Browser ({label} não lidas)"),
                    )
                } else {
                    (render_whatsapp_icon_rgba(), "WhatsApp Browser".to_string())
                };
                tray_icon.re_add(&tip, &rgba);
            }
            Event::UserEvent(UserEvent::Ipc(cmd)) => match cmd {
                IpcCommand::Ready => {
                    sync_tab_bar(&tab_bar, &config, &runtimes);
                    sync_maximized(&tab_bar, window.is_maximized());
                }
                IpcCommand::NewTab => {
                    if let Some(prev) = runtimes.get(&config.active_id) {
                        let _ = prev.webview.set_visible(false);
                    }
                    let new_tab = config.add_tab();
                    let (_, current_content_bounds) = compute_bounds(&window);
                    if let Ok(webview) = create_whatsapp_webview(
                        &window,
                        &mut web_context,
                        proxy.clone(),
                        &new_tab,
                        current_content_bounds,
                        true,
                    ) {
                        let _ = webview.focus();
                        runtimes.insert(
                            new_tab.id,
                            TabRuntime {
                                webview,
                                unread: None,
                            },
                        );
                        let _ = config.save(&config_path);
                        update_notifications(
                            &window,
                            &runtimes,
                            &mut last_badge,
                            #[cfg(target_os = "windows")]
                            &mut tray_icon,
                        );
                        sync_tab_bar(&tab_bar, &config, &runtimes);
                    }
                }
                IpcCommand::Switch { id } => {
                    if id != config.active_id && runtimes.contains_key(&id) {
                        if let Some(prev) = runtimes.get(&config.active_id) {
                            let _ = prev.webview.set_visible(false);
                        }
                        if let Some(target) = runtimes.get(&id) {
                            let _ = target.webview.set_visible(true);
                            let _ = target.webview.focus();
                        }
                        config.active_id = id;
                        let _ = config.save(&config_path);
                        sync_tab_bar(&tab_bar, &config, &runtimes);
                    }
                }
                IpcCommand::Close { id } => {
                    runtimes.remove(&id);
                    if let Some(replacement) = config.remove_tab(id) {
                        let (_, current_content_bounds) = compute_bounds(&window);
                        if let Ok(webview) = create_whatsapp_webview(
                            &window,
                            &mut web_context,
                            proxy.clone(),
                            &replacement,
                            current_content_bounds,
                            true,
                        ) {
                            runtimes.insert(
                                replacement.id,
                                TabRuntime {
                                    webview,
                                    unread: None,
                                },
                            );
                        }
                    }
                    if let Some(active) = runtimes.get(&config.active_id) {
                        let _ = active.webview.set_visible(true);
                        let _ = active.webview.focus();
                    }
                    let _ = config.save(&config_path);
                    update_notifications(
                        &window,
                        &runtimes,
                        &mut last_badge,
                        #[cfg(target_os = "windows")]
                        &mut tray_icon,
                    );
                    sync_tab_bar(&tab_bar, &config, &runtimes);
                }
                IpcCommand::Rename { id, name } => {
                    if config.rename_tab(id, &name) {
                        let _ = config.save(&config_path);
                        sync_tab_bar(&tab_bar, &config, &runtimes);
                    }
                }
                IpcCommand::Reload => {
                    if let Some(active) = runtimes.get(&config.active_id) {
                        let _ = active.webview.load_url(WHATSAPP_URL);
                    }
                }
                IpcCommand::DragWindow => {
                    let _ = window.drag_window();
                }
                IpcCommand::ResizeNorth => {
                    if !window.is_maximized() {
                        let _ = window.drag_resize_window(ResizeDirection::North);
                    }
                }
                IpcCommand::Minimize => {
                    window.set_minimized(true);
                }
                IpcCommand::ToggleMaximize => {
                    let next = !window.is_maximized();
                    window.set_maximized(next);
                    sync_maximized(&tab_bar, next);
                }
                IpcCommand::CloseWindow => {
                    let _ = config.save(&config_path);
                    #[cfg(target_os = "windows")]
                    tray_icon.remove();
                    runtimes.clear();
                    std::process::exit(0);
                }
            },
            _ => (),
        }
    });
}
