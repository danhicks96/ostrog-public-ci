//! Windows GDI/User32 launcher window (WP-02 slice 2.5).
//!
//! Dependency choice (documented per WP-02 slice 2.5):
//! We use raw Win32 FFI (`user32.dll` and `gdi32.dll`) directly in Rust rather than adding a third-
//! party GUI crate (`eframe`, `iced`, `tao`, or even `windows-sys`). Both DLLs are guaranteed on
//! every Windows installation, add zero crates to `Cargo.lock`, keep the launcher binary tiny, and
//! render immediately on double-click with `windows_subsystem = "windows"` (no console window).
//!
//! Architecture:
//! - The UI runs on the main thread with a 60 ms `SetTimer` heartbeat so the progress bar and
//!   activity dots animate continuously even when no network bytes arrive.
//! - The update check, download, verification, staging, and process spawn run on a background
//!   worker thread (`std::thread::spawn`) that pushes `(LauncherStage, bytes, total)` snapshots
//!   over an `mpsc` channel.
//! - A Cancel button is enabled during `Downloading`, and `Retry` / `Close` buttons are enabled
//!   when the window enters a retryable `Offline` (first install) or `Failure` state.

#![cfg(windows)]
#![allow(clippy::upper_case_acronyms, clippy::type_complexity)]

use crate::{
    append_launcher_log, run_with_progress, LaunchOutcome, LauncherError, LauncherSettings,
    LauncherStage, ProgressSink,
};
use flate2::{write::ZlibEncoder, Compression};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, OnceLock,
    },
    time::Instant,
};

type HINSTANCE = *mut core::ffi::c_void;
type HWND = *mut core::ffi::c_void;
type HDC = *mut core::ffi::c_void;
type HGDIOBJ = *mut core::ffi::c_void;
type HBRUSH = *mut core::ffi::c_void;
type HPEN = *mut core::ffi::c_void;
type HFONT = *mut core::ffi::c_void;
type HICON = *mut core::ffi::c_void;
type HCURSOR = *mut core::ffi::c_void;
type HMENU = *mut core::ffi::c_void;
type HBITMAP = *mut core::ffi::c_void;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type UINT = u32;
type DWORD = u32;
type WORD = u16;
type LONG = i32;
type BOOL = i32;
type COLORREF = u32;
type ATOM = u16;

const WS_OVERLAPPED: DWORD = 0x0000_0000;
const WS_CAPTION: DWORD = 0x00C0_0000;
const WS_SYSMENU: DWORD = 0x0008_0000;
const WS_MINIMIZEBOX: DWORD = 0x0002_0000;
const WS_VISIBLE: DWORD = 0x1000_0000;
const WS_CHILD: DWORD = 0x4000_0000;
const WS_TABSTOP: DWORD = 0x0001_0000;
const WS_DISABLED: DWORD = 0x0800_0000;
const BS_PUSHBUTTON: DWORD = 0x0000_0000;

const WM_CREATE: UINT = 0x0001;
const WM_DESTROY: UINT = 0x0002;
const WM_PAINT: UINT = 0x000F;
const WM_CLOSE: UINT = 0x0010;
const WM_ERASEBKGND: UINT = 0x0014;
const WM_SETFONT: UINT = 0x0030;
const WM_COMMAND: UINT = 0x0111;
const WM_TIMER: UINT = 0x0113;

const IDC_ARROW: *const u16 = 32512 as *const u16;
const COLOR_WINDOW: i32 = 5;
const TRANSPARENT: i32 = 1;
const DT_LEFT: UINT = 0x0000_0000;
const DT_WORDBREAK: UINT = 0x0000_0010;
const DT_NOPREFIX: UINT = 0x0000_0800;
const SRCCOPY: DWORD = 0x00CC_0020;
const BI_RGB: DWORD = 0;
const DIB_RGB_COLORS: UINT = 0;
const FW_NORMAL: i32 = 400;
const FW_SEMIBOLD: i32 = 600;
const DEFAULT_CHARSET: DWORD = 1;
const OUT_DEFAULT_PRECIS: DWORD = 0;
const CLIP_DEFAULT_PRECIS: DWORD = 0;
const CLEARTYPE_QUALITY: DWORD = 5;
const DEFAULT_PITCH: DWORD = 0;
const SW_SHOW: i32 = 5;
const SW_HIDE: i32 = 0;

const TIMER_HEARTBEAT_ID: usize = 1;
const HEARTBEAT_INTERVAL_MS: UINT = 60;

const BTN_CANCEL_ID: usize = 101;
const BTN_RETRY_ID: usize = 102;
const BTN_CLOSE_ID: usize = 103;

const WIN_CLIENT_W: i32 = 520;
const WIN_CLIENT_H: i32 = 220;

#[repr(C)]
struct WNDCLASSEXW {
    cb_size: UINT,
    style: UINT,
    lpfn_wnd_proc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HINSTANCE,
    h_icon: HICON,
    h_cursor: HCURSOR,
    hbr_background: HBRUSH,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: HICON,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RECT {
    left: LONG,
    top: LONG,
    right: LONG,
    bottom: LONG,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct POINT {
    x: LONG,
    y: LONG,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
    time: DWORD,
    pt: POINT,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    f_erase: BOOL,
    rc_paint: RECT,
    f_restore: BOOL,
    f_inc_update: BOOL,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct BITMAPINFOHEADER {
    bi_size: DWORD,
    bi_width: LONG,
    bi_height: LONG,
    bi_planes: WORD,
    bi_bit_count: WORD,
    bi_compression: DWORD,
    bi_size_image: DWORD,
    bi_x_pels_per_meter: LONG,
    bi_y_pels_per_meter: LONG,
    bi_clr_used: DWORD,
    bi_clr_important: DWORD,
}

#[repr(C)]
struct BITMAPINFO {
    bmi_header: BITMAPINFOHEADER,
    bmi_colors: [u32; 1],
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> ATOM;
    fn CreateWindowExW(
        dw_ex_style: DWORD,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: DWORD,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        h_wnd_parent: HWND,
        h_menu: HMENU,
        h_instance: HINSTANCE,
        lp_param: *mut core::ffi::c_void,
    ) -> HWND;
    fn DefWindowProcW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn ShowWindow(hwnd: HWND, n_cmd_show: i32) -> BOOL;
    fn UpdateWindow(hwnd: HWND) -> BOOL;
    fn DestroyWindow(hwnd: HWND) -> BOOL;
    fn GetMessageW(
        lp_msg: *mut MSG,
        hwnd: HWND,
        w_msg_filter_min: UINT,
        w_msg_filter_max: UINT,
    ) -> BOOL;
    fn PeekMessageW(
        lp_msg: *mut MSG,
        hwnd: HWND,
        w_msg_filter_min: UINT,
        w_msg_filter_max: UINT,
        w_remove_msg: UINT,
    ) -> BOOL;
    fn TranslateMessage(lp_msg: *const MSG) -> BOOL;
    fn DispatchMessageW(lp_msg: *const MSG) -> LRESULT;
    fn PostQuitMessage(n_exit_code: i32);
    fn SetTimer(
        hwnd: HWND,
        n_id_event: usize,
        u_elapse: UINT,
        lp_timer_func: *const core::ffi::c_void,
    ) -> usize;
    fn KillTimer(hwnd: HWND, u_id_event: usize) -> BOOL;
    fn InvalidateRect(hwnd: HWND, lp_rect: *const RECT, b_erase: BOOL) -> BOOL;
    fn BeginPaint(hwnd: HWND, lp_paint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hwnd: HWND, lp_paint: *const PAINTSTRUCT) -> BOOL;
    fn GetClientRect(hwnd: HWND, lp_rect: *mut RECT) -> BOOL;
    fn FillRect(hdc: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    fn DrawTextW(
        hdc: HDC,
        lpch_text: *const u16,
        cch_text: i32,
        lprc: *mut RECT,
        format: UINT,
    ) -> i32;
    fn LoadCursorW(h_instance: HINSTANCE, lp_cursor_name: *const u16) -> HCURSOR;
    fn SendMessageW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn EnableWindow(hwnd: HWND, b_enable: BOOL) -> BOOL;
    fn AdjustWindowRectEx(
        lp_rect: *mut RECT,
        dw_style: DWORD,
        b_menu: BOOL,
        dw_ex_style: DWORD,
    ) -> BOOL;
    fn GetSystemMetrics(n_index: i32) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    fn CreatePen(i_style: i32, c_width: i32, color: COLORREF) -> HPEN;
    fn CreateFontW(
        c_height: i32,
        c_width: i32,
        c_escapement: i32,
        c_orientation: i32,
        c_weight: i32,
        b_italic: DWORD,
        b_underline: DWORD,
        b_strike_out: DWORD,
        i_char_set: DWORD,
        i_out_precision: DWORD,
        i_clip_precision: DWORD,
        i_quality: DWORD,
        i_pitch_and_family: DWORD,
        psz_face_name: *const u16,
    ) -> HFONT;
    fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
    fn DeleteObject(ho: HGDIOBJ) -> BOOL;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    fn CreateCompatibleDC(hdc: HDC) -> HDC;
    fn DeleteDC(hdc: HDC) -> BOOL;
    fn CreateCompatibleBitmap(hdc: HDC, cx: i32, cy: i32) -> HBITMAP;
    fn BitBlt(
        hdc: HDC,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        hdc_src: HDC,
        x1: i32,
        y1: i32,
        rop: DWORD,
    ) -> BOOL;
    fn GetDIBits(
        hdc: HDC,
        hbm: HBITMAP,
        start: UINT,
        c_lines: UINT,
        lpv_bits: *mut core::ffi::c_void,
        lpbmi: *mut BITMAPINFO,
        usage: UINT,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> HINSTANCE;
}

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Clone, Debug)]
struct WorkerUpdate {
    stage: LauncherStage,
    bytes: u64,
    total: Option<u64>,
}

enum WorkerEvent {
    Progress(WorkerUpdate),
    Done(Result<(), String>),
}

struct WindowSharedState {
    settings: LauncherSettings,
    now: u64,
    game_args: Vec<OsString>,
    stage: LauncherStage,
    bytes: u64,
    total: Option<u64>,
    download_started: Option<Instant>,
    heartbeat_tick: u64,
    cancel_requested: Arc<AtomicBool>,
    rx: Option<Receiver<WorkerEvent>>,
    worker_running: bool,
    launching_hold_ticks: u32,
    exit_result: Result<(), String>,
    btn_cancel: HWND,
    btn_retry: HWND,
    btn_close: HWND,
    title_font: HFONT,
    body_font: HFONT,
    small_font: HFONT,
}

unsafe impl Send for WindowSharedState {}

static GLOBAL_STATE: OnceLock<Mutex<WindowSharedState>> = OnceLock::new();

struct ChannelProgressSink {
    tx: Sender<WorkerEvent>,
    cancel_requested: Arc<AtomicBool>,
}

impl ProgressSink for ChannelProgressSink {
    fn on_progress(&mut self, stage: LauncherStage, bytes: u64, total: Option<u64>) {
        if self.cancel_requested.load(Ordering::Relaxed) {
            return;
        }
        let _ = self.tx.send(WorkerEvent::Progress(WorkerUpdate {
            stage,
            bytes,
            total,
        }));
    }
}

fn spawn_worker(state: &mut WindowSharedState) {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    state.cancel_requested = cancel.clone();
    state.rx = Some(rx);
    state.worker_running = true;
    state.download_started = None;
    state.launching_hold_ticks = 0;

    let settings = state.settings.clone();
    let now = state.now;
    let game_args = state.game_args.clone();

    std::thread::spawn(move || {
        let mut sink = ChannelProgressSink {
            tx: tx.clone(),
            cancel_requested: cancel.clone(),
        };
        let outcome = run_with_progress(&settings, now, game_args, &mut sink);
        if cancel.load(Ordering::Relaxed) {
            let _ = append_launcher_log(&settings.user_data_dir, "user cancelled download");
            let _ = tx.send(WorkerEvent::Done(Err("cancelled by user".into())));
            return;
        }
        match outcome {
            Ok(LaunchOutcome::StartedCurrent { reason, .. }) => {
                let _ = append_launcher_log(
                    &settings.user_data_dir,
                    &format!("started installed build ({reason:?})"),
                );
                let _ = tx.send(WorkerEvent::Done(Ok(())));
            }
            Ok(LaunchOutcome::UpdatedAndStarted { version, .. }) => {
                let _ = append_launcher_log(
                    &settings.user_data_dir,
                    &format!("updated to {version} and started"),
                );
                let _ = tx.send(WorkerEvent::Done(Ok(())));
            }
            Err(LauncherError::NoInstalledBuild {
                reason: crate::LaunchReason::Offline,
            }) => {
                // Keep stage on Offline when there is no installed build so the friend sees
                // "Can't reach the server. Check your internet and try again." with Retry.
                let _ = tx.send(WorkerEvent::Progress(WorkerUpdate {
                    stage: LauncherStage::Offline,
                    bytes: 0,
                    total: None,
                }));
                let _ = tx.send(WorkerEvent::Done(Err("offline on first install".into())));
            }
            Err(error) => {
                let _ = append_launcher_log(
                    &settings.user_data_dir,
                    &format!("launcher error: {error}"),
                );
                let _ = tx.send(WorkerEvent::Done(Err(error.to_string())));
            }
        }
    });
}

unsafe fn sync_buttons(state: &WindowSharedState) {
    let is_downloading = matches!(state.stage, LauncherStage::Downloading { .. });
    let is_terminal_wait = !state.worker_running
        && (matches!(state.stage, LauncherStage::Failure { .. })
            || (matches!(state.stage, LauncherStage::Offline)
                && !state.settings.has_installed_build()));

    if !state.btn_cancel.is_null() {
        EnableWindow(state.btn_cancel, i32::from(is_downloading));
        ShowWindow(
            state.btn_cancel,
            if is_downloading { SW_SHOW } else { SW_HIDE },
        );
    }
    if !state.btn_retry.is_null() {
        EnableWindow(state.btn_retry, i32::from(is_terminal_wait));
        ShowWindow(
            state.btn_retry,
            if is_terminal_wait { SW_SHOW } else { SW_HIDE },
        );
    }
    if !state.btn_close.is_null() {
        EnableWindow(state.btn_close, i32::from(is_terminal_wait));
        ShowWindow(
            state.btn_close,
            if is_terminal_wait { SW_SHOW } else { SW_HIDE },
        );
    }
}

unsafe fn paint_into_dc(hdc: HDC, width: i32, height: i32, state: &WindowSharedState) {
    let bg_brush = CreateSolidBrush(rgb(24, 26, 31));
    let card_brush = CreateSolidBrush(rgb(34, 38, 46));
    let track_brush = CreateSolidBrush(rgb(48, 54, 66));
    let fill_brush = match state.stage {
        LauncherStage::Failure { .. } => CreateSolidBrush(rgb(198, 72, 72)),
        LauncherStage::Offline => CreateSolidBrush(rgb(209, 154, 62)),
        _ => CreateSolidBrush(rgb(104, 166, 120)),
    };
    let heartbeat_brush = CreateSolidBrush(rgb(138, 198, 152));
    let border_pen = CreatePen(0, 1, rgb(62, 68, 82));

    let full_rc = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    FillRect(hdc, &full_rc, bg_brush);

    let card_rc = RECT {
        left: 16,
        top: 16,
        right: width - 16,
        bottom: height - 16,
    };
    FillRect(hdc, &card_rc, card_brush);

    SetBkMode(hdc, TRANSPARENT);

    // Header title
    SelectObject(hdc, state.title_font as HGDIOBJ);
    SetTextColor(hdc, rgb(238, 232, 218));
    let mut title_rc = RECT {
        left: 32,
        top: 28,
        right: width - 120,
        bottom: 60,
    };
    let title_w = wide("Ostrog");
    DrawTextW(
        hdc,
        title_w.as_ptr(),
        (title_w.len() - 1) as i32,
        &mut title_rc,
        DT_LEFT | DT_NOPREFIX,
    );

    // Animated heartbeat indicator in the top-right corner so the user always sees the window is alive
    let dots = match state.heartbeat_tick % 4 {
        0 => "● ○ ○",
        1 => "○ ● ○",
        2 => "○ ○ ●",
        _ => "○ ● ○",
    };
    SelectObject(hdc, state.small_font as HGDIOBJ);
    SetTextColor(hdc, rgb(146, 178, 156));
    let mut hb_rc = RECT {
        left: width - 104,
        top: 34,
        right: width - 32,
        bottom: 58,
    };
    let hb_w = wide(dots);
    DrawTextW(
        hdc,
        hb_w.as_ptr(),
        (hb_w.len() - 1) as i32,
        &mut hb_rc,
        DT_LEFT | DT_NOPREFIX,
    );

    // Main stage status text
    let elapsed_secs = state
        .download_started
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(31.7);
    let status_text = state.stage.format_status(
        state.settings.has_installed_build(),
        state.bytes,
        state.total,
        elapsed_secs,
    );

    SelectObject(hdc, state.body_font as HGDIOBJ);
    SetTextColor(hdc, rgb(224, 228, 235));
    let mut status_rc = RECT {
        left: 32,
        top: 68,
        right: width - 32,
        bottom: 124,
    };
    let status_w = wide(&status_text);
    DrawTextW(
        hdc,
        status_w.as_ptr(),
        (status_w.len() - 1) as i32,
        &mut status_rc,
        DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
    );

    // Progress bar track
    let bar_left = 32;
    let bar_right = width - 32;
    let bar_top = 132;
    let bar_bottom = 150;
    let bar_w = (bar_right - bar_left).max(1);
    let track_rc = RECT {
        left: bar_left,
        top: bar_top,
        right: bar_right,
        bottom: bar_bottom,
    };
    FillRect(hdc, &track_rc, track_brush);

    match (&state.stage, state.total.filter(|&t| t > 0)) {
        (LauncherStage::Downloading { .. }, Some(total_bytes)) => {
            let ratio = (state.bytes as f64 / total_bytes as f64).clamp(0.0, 1.0);
            let fill_w = ((bar_w as f64) * ratio).round() as i32;
            let fill_rc = RECT {
                left: bar_left,
                top: bar_top,
                right: (bar_left + fill_w).min(bar_right),
                bottom: bar_bottom,
            };
            FillRect(hdc, &fill_rc, fill_brush);
        }
        (LauncherStage::Failure { .. }, _) => {
            FillRect(hdc, &track_rc, fill_brush);
        }
        (LauncherStage::Offline, _) if !state.settings.has_installed_build() => {
            FillRect(hdc, &track_rc, fill_brush);
        }
        _ => {
            // Indeterminate animated marquee bar driven by the 60 ms heartbeat timer
            let seg_w = (bar_w / 3).max(40);
            let cycle = bar_w + seg_w;
            let pos = ((state.heartbeat_tick as i32 * 18) % cycle) - seg_w;
            let seg_left = (bar_left + pos).clamp(bar_left, bar_right);
            let seg_right = (bar_left + pos + seg_w).clamp(bar_left, bar_right);
            if seg_right > seg_left {
                let seg_rc = RECT {
                    left: seg_left,
                    top: bar_top,
                    right: seg_right,
                    bottom: bar_bottom,
                };
                FillRect(hdc, &seg_rc, heartbeat_brush);
            }
        }
    }

    // Footer caption
    SelectObject(hdc, state.small_font as HGDIOBJ);
    SetTextColor(hdc, rgb(148, 155, 168));
    let footer = state.stage.footer();
    let mut footer_rc = RECT {
        left: 32,
        top: 168,
        right: width - 210,
        bottom: 196,
    };
    let footer_w = wide(&footer);
    DrawTextW(
        hdc,
        footer_w.as_ptr(),
        (footer_w.len() - 1) as i32,
        &mut footer_rc,
        DT_LEFT | DT_NOPREFIX,
    );

    DeleteObject(border_pen as HGDIOBJ);
    DeleteObject(heartbeat_brush as HGDIOBJ);
    DeleteObject(fill_brush as HGDIOBJ);
    DeleteObject(track_brush as HGDIOBJ);
    DeleteObject(card_brush as HGDIOBJ);
    DeleteObject(bg_brush as HGDIOBJ);
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            if let Some(lock) = GLOBAL_STATE.get() {
                if let Ok(mut state) = lock.lock() {
                    let h_instance = GetModuleHandleW(null());
                    let btn_class = wide("BUTTON");

                    let cancel_label = wide("Cancel");
                    state.btn_cancel = CreateWindowExW(
                        0,
                        btn_class.as_ptr(),
                        cancel_label.as_ptr(),
                        WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
                        WIN_CLIENT_W - 116,
                        162,
                        84,
                        28,
                        hwnd,
                        BTN_CANCEL_ID as HMENU,
                        h_instance,
                        null_mut(),
                    );
                    SendMessageW(state.btn_cancel, WM_SETFONT, state.small_font as WPARAM, 1);

                    let retry_label = wide("Retry");
                    state.btn_retry = CreateWindowExW(
                        0,
                        btn_class.as_ptr(),
                        retry_label.as_ptr(),
                        WS_CHILD | WS_TABSTOP | WS_DISABLED | BS_PUSHBUTTON,
                        WIN_CLIENT_W - 208,
                        162,
                        84,
                        28,
                        hwnd,
                        BTN_RETRY_ID as HMENU,
                        h_instance,
                        null_mut(),
                    );
                    SendMessageW(state.btn_retry, WM_SETFONT, state.small_font as WPARAM, 1);

                    let close_label = wide("Close");
                    state.btn_close = CreateWindowExW(
                        0,
                        btn_class.as_ptr(),
                        close_label.as_ptr(),
                        WS_CHILD | WS_TABSTOP | WS_DISABLED | BS_PUSHBUTTON,
                        WIN_CLIENT_W - 116,
                        162,
                        84,
                        28,
                        hwnd,
                        BTN_CLOSE_ID as HMENU,
                        h_instance,
                        null_mut(),
                    );
                    SendMessageW(state.btn_close, WM_SETFONT, state.small_font as WPARAM, 1);

                    sync_buttons(&state);
                }
            }
            SetTimer(hwnd, TIMER_HEARTBEAT_ID, HEARTBEAT_INTERVAL_MS, null());
            0
        }
        WM_TIMER => {
            let mut should_close = false;
            if let Some(lock) = GLOBAL_STATE.get() {
                if let Ok(mut state) = lock.lock() {
                    state.heartbeat_tick = state.heartbeat_tick.wrapping_add(1);
                    let mut disconnected = false;
                    let mut events = Vec::new();
                    if let Some(rx) = &state.rx {
                        while let Ok(ev) = rx.try_recv() {
                            events.push(ev);
                        }
                    }
                    for ev in events {
                        match ev {
                            WorkerEvent::Progress(update) => {
                                if matches!(update.stage, LauncherStage::Downloading { .. })
                                    && state.download_started.is_none()
                                {
                                    state.download_started = Some(Instant::now());
                                }
                                state.stage = update.stage;
                                state.bytes = update.bytes;
                                state.total = update.total;
                            }
                            WorkerEvent::Done(result) => {
                                state.worker_running = false;
                                state.exit_result = result;
                                disconnected = true;
                            }
                        }
                    }
                    if disconnected {
                        state.rx = None;
                    }
                    sync_buttons(&state);

                    if !state.worker_running && state.exit_result.is_ok() {
                        state.launching_hold_ticks += 1;
                        if state.launching_hold_ticks >= 4 {
                            should_close = true;
                        }
                    }
                }
            }
            InvalidateRect(hwnd, null(), 0);
            if should_close {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let id = w_param & 0xFFFF;
            match id {
                BTN_CANCEL_ID => {
                    if let Some(lock) = GLOBAL_STATE.get() {
                        if let Ok(mut state) = lock.lock() {
                            state.cancel_requested.store(true, Ordering::Relaxed);
                            state.worker_running = false;
                            state.rx = None;
                            state.stage = LauncherStage::Failure {
                                message: "Download cancelled".into(),
                            };
                            state.exit_result = Err("Download cancelled".into());
                            sync_buttons(&state);
                        }
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
                BTN_RETRY_ID => {
                    if let Some(lock) = GLOBAL_STATE.get() {
                        if let Ok(mut state) = lock.lock() {
                            if !state.worker_running {
                                state.stage = LauncherStage::Starting;
                                state.bytes = 0;
                                state.total = None;
                                spawn_worker(&mut state);
                                sync_buttons(&state);
                            }
                        }
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
                BTN_CLOSE_ID => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc);
            let w = (rc.right - rc.left).max(1);
            let h = (rc.bottom - rc.top).max(1);

            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bm = CreateCompatibleBitmap(hdc, w, h);
            let old_bm = SelectObject(mem_dc, mem_bm as HGDIOBJ);

            if let Some(lock) = GLOBAL_STATE.get() {
                if let Ok(state) = lock.lock() {
                    paint_into_dc(mem_dc, w, h, &state);
                }
            }

            BitBlt(hdc, 0, 0, w, h, mem_dc, 0, 0, SRCCOPY);
            SelectObject(mem_dc, old_bm);
            DeleteObject(mem_bm as HGDIOBJ);
            DeleteDC(mem_dc);

            EndPaint(hwnd, &ps);
            0
        }
        WM_CLOSE => {
            if let Some(lock) = GLOBAL_STATE.get() {
                if let Ok(state) = lock.lock() {
                    state.cancel_requested.store(true, Ordering::Relaxed);
                }
            }
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_HEARTBEAT_ID);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

unsafe fn create_fonts() -> (HFONT, HFONT, HFONT) {
    let face = wide("Segoe UI");
    let title = CreateFontW(
        -22,
        0,
        0,
        0,
        FW_SEMIBOLD,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH,
        face.as_ptr(),
    );
    let body = CreateFontW(
        -15,
        0,
        0,
        0,
        FW_NORMAL,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH,
        face.as_ptr(),
    );
    let small = CreateFontW(
        -13,
        0,
        0,
        0,
        FW_NORMAL,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH,
        face.as_ptr(),
    );
    (title, body, small)
}

unsafe fn create_launcher_hwnd() -> io::Result<HWND> {
    let h_instance = GetModuleHandleW(null());
    let class_name = wide("OstrogLauncherWindow");
    let wc = WNDCLASSEXW {
        cb_size: size_of::<WNDCLASSEXW>() as UINT,
        style: 0,
        lpfn_wnd_proc: Some(wnd_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance,
        h_icon: null_mut(),
        h_cursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbr_background: (COLOR_WINDOW + 1) as HBRUSH,
        lpsz_menu_name: null(),
        lpsz_class_name: class_name.as_ptr(),
        h_icon_sm: null_mut(),
    };
    let _ = RegisterClassExW(&wc);

    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: WIN_CLIENT_W,
        bottom: WIN_CLIENT_H,
    };
    AdjustWindowRectEx(&mut rect, style, 0, 0);
    let win_w = rect.right - rect.left;
    let win_h = rect.bottom - rect.top;
    let screen_w = GetSystemMetrics(0);
    let screen_h = GetSystemMetrics(1);
    let x = ((screen_w - win_w) / 2).max(40);
    let y = ((screen_h - win_h) / 2).max(40);

    let title = wide("Ostrog");
    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        title.as_ptr(),
        style,
        x,
        y,
        win_w,
        win_h,
        null_mut(),
        null_mut(),
        h_instance,
        null_mut(),
    );
    if hwnd.is_null() {
        return Err(io::Error::other("CreateWindowExW failed"));
    }
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    Ok(hwnd)
}

/// Runs the native Windows launcher window, executing startup/update work on a background worker
/// thread while animating the progress bar and heartbeat on the main thread.
pub fn run_windowed(
    settings: LauncherSettings,
    now: u64,
    game_args: Vec<OsString>,
) -> Result<(), String> {
    unsafe {
        let (title_font, body_font, small_font) = create_fonts();
        let mut initial_state = WindowSharedState {
            settings,
            now,
            game_args,
            stage: LauncherStage::Starting,
            bytes: 0,
            total: None,
            download_started: None,
            heartbeat_tick: 0,
            cancel_requested: Arc::new(AtomicBool::new(false)),
            rx: None,
            worker_running: false,
            launching_hold_ticks: 0,
            exit_result: Ok(()),
            btn_cancel: null_mut(),
            btn_retry: null_mut(),
            btn_close: null_mut(),
            title_font,
            body_font,
            small_font,
        };
        spawn_worker(&mut initial_state);
        if let Some(lock) = GLOBAL_STATE.get() {
            if let Ok(mut guard) = lock.lock() {
                *guard = initial_state;
            }
        } else {
            let _ = GLOBAL_STATE.set(Mutex::new(initial_state));
        }

        let _hwnd = create_launcher_hwnd().map_err(|e| e.to_string())?;
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        DeleteObject(title_font as HGDIOBJ);
        DeleteObject(body_font as HGDIOBJ);
        DeleteObject(small_font as HGDIOBJ);

        if let Some(lock) = GLOBAL_STATE.get() {
            if let Ok(guard) = lock.lock() {
                return guard.exit_result.clone();
            }
        }
        Ok(())
    }
}

/// Renders every stage in the WP-02 status table into the real Win32 launcher window and writes
/// a PNG screenshot for each stage into `output_dir`.
pub fn dump_stage_screenshots(
    settings: &LauncherSettings,
    output_dir: &Path,
) -> std::io::Result<Vec<PathBuf>> {
    fs::create_dir_all(output_dir)?;
    unsafe {
        let (title_font, body_font, small_font) = create_fonts();
        let initial_state = WindowSharedState {
            settings: settings.clone(),
            now: 0,
            game_args: Vec::new(),
            stage: LauncherStage::Starting,
            bytes: 0,
            total: None,
            download_started: None,
            heartbeat_tick: 1,
            cancel_requested: Arc::new(AtomicBool::new(false)),
            rx: None,
            worker_running: true,
            launching_hold_ticks: 0,
            exit_result: Ok(()),
            btn_cancel: null_mut(),
            btn_retry: null_mut(),
            btn_close: null_mut(),
            title_font,
            body_font,
            small_font,
        };
        if let Some(lock) = GLOBAL_STATE.get() {
            if let Ok(mut guard) = lock.lock() {
                *guard = initial_state;
            }
        } else {
            let _ = GLOBAL_STATE.set(Mutex::new(initial_state));
        }

        let hwnd = create_launcher_hwnd()?;
        pump_pending_messages();

        let mb = 1_048_576u64;
        let samples: Vec<(&str, LauncherStage, u64, Option<u64>, bool, u64)> = vec![
            ("01_starting", LauncherStage::Starting, 0, None, true, 4),
            ("02_checking", LauncherStage::Checking, 0, None, true, 9),
            ("03_offline", LauncherStage::Offline, 0, None, false, 14),
            (
                "04_downloading",
                LauncherStage::Downloading {
                    version: "0.4.1".into(),
                },
                46 * mb,
                Some(104 * mb),
                true,
                19,
            ),
            ("05_verifying", LauncherStage::Verifying, 0, None, true, 12),
            (
                "06_installing",
                LauncherStage::Installing,
                0,
                None,
                true,
                18,
            ),
            ("07_launching", LauncherStage::Launching, 0, None, true, 24),
            (
                "08_failure",
                LauncherStage::Failure {
                    message: "the update server closed the connection early".into(),
                },
                0,
                None,
                false,
                28,
            ),
        ];

        let mut written = Vec::new();
        for (filename, stage, bytes, total, worker_running, tick) in samples {
            if let Some(lock) = GLOBAL_STATE.get() {
                if let Ok(mut state) = lock.lock() {
                    state.stage = stage;
                    state.bytes = bytes;
                    state.total = total;
                    state.worker_running = worker_running;
                    state.heartbeat_tick = tick;
                    sync_buttons(&state);
                }
            }
            InvalidateRect(hwnd, null(), 0);
            UpdateWindow(hwnd);
            pump_pending_messages();

            let path = output_dir.join(format!("{filename}.png"));
            if let Some(lock) = GLOBAL_STATE.get() {
                if let Ok(state) = lock.lock() {
                    capture_state_png(&state, WIN_CLIENT_W, WIN_CLIENT_H, &path)?;
                }
            }
            written.push(path);
        }

        DestroyWindow(hwnd);
        pump_pending_messages();
        DeleteObject(title_font as HGDIOBJ);
        DeleteObject(body_font as HGDIOBJ);
        DeleteObject(small_font as HGDIOBJ);
        Ok(written)
    }
}

unsafe fn pump_pending_messages() {
    let mut msg: MSG = zeroed();
    while PeekMessageW(&mut msg, null_mut(), 0, 0, 1) != 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

unsafe fn capture_state_png(
    state: &WindowSharedState,
    width: i32,
    height: i32,
    path: &Path,
) -> std::io::Result<()> {
    let screen_dc = CreateCompatibleDC(null_mut());
    let mem_bm = CreateCompatibleBitmap(screen_dc, width, height);
    // CreateCompatibleBitmap on a memory DC creates a 1-bit bitmap unless selected from a real DC;
    // use GetDC(null) via CreateCompatibleDC(null) + a 32-bit DIB section or paint into a window DC.
    DeleteObject(mem_bm as HGDIOBJ);
    DeleteDC(screen_dc);

    // Paint into an offscreen 24/32-bit bitmap compatible with the desktop DC.
    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: HWND) -> HDC;
        fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    }
    let desktop_dc = GetDC(null_mut());
    let mem_dc = CreateCompatibleDC(desktop_dc);
    let bmp = CreateCompatibleBitmap(desktop_dc, width, height);
    let old = SelectObject(mem_dc, bmp as HGDIOBJ);

    paint_into_dc(mem_dc, width, height, state);
    draw_button_overlays_for_capture(mem_dc, width, state);

    let mut bmi = BITMAPINFO {
        bmi_header: BITMAPINFOHEADER {
            bi_size: size_of::<BITMAPINFOHEADER>() as DWORD,
            bi_width: width,
            bi_height: -height, // top-down
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            bi_size_image: 0,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        },
        bmi_colors: [0],
    };
    let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
    GetDIBits(
        mem_dc,
        bmp,
        0,
        height as UINT,
        bgra.as_mut_ptr() as *mut core::ffi::c_void,
        &mut bmi,
        DIB_RGB_COLORS,
    );

    SelectObject(mem_dc, old);
    DeleteObject(bmp as HGDIOBJ);
    DeleteDC(mem_dc);
    ReleaseDC(null_mut(), desktop_dc);

    let mut rgba = Vec::with_capacity(bgra.len());
    for px in bgra.chunks_exact(4) {
        rgba.push(px[2]);
        rgba.push(px[1]);
        rgba.push(px[0]);
        rgba.push(255);
    }
    write_png_rgba(path, width as u32, height as u32, &rgba)
}

unsafe fn draw_button_overlays_for_capture(hdc: HDC, width: i32, state: &WindowSharedState) {
    let is_downloading = matches!(state.stage, LauncherStage::Downloading { .. });
    let is_terminal_wait = !state.worker_running
        && (matches!(state.stage, LauncherStage::Failure { .. })
            || (matches!(state.stage, LauncherStage::Offline)
                && !state.settings.has_installed_build()));

    if is_downloading {
        draw_button_box(hdc, state.small_font, width - 116, 162, 84, 28, "Cancel");
    } else if is_terminal_wait {
        draw_button_box(hdc, state.small_font, width - 208, 162, 84, 28, "Retry");
        draw_button_box(hdc, state.small_font, width - 116, 162, 84, 28, "Close");
    }
}

unsafe fn draw_button_box(hdc: HDC, font: HFONT, x: i32, y: i32, w: i32, h: i32, label: &str) {
    let border_brush = CreateSolidBrush(rgb(96, 106, 124));
    let face_brush = CreateSolidBrush(rgb(54, 60, 74));
    let outer = RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    };
    let inner = RECT {
        left: x + 1,
        top: y + 1,
        right: x + w - 1,
        bottom: y + h - 1,
    };
    FillRect(hdc, &outer, border_brush);
    FillRect(hdc, &inner, face_brush);
    SelectObject(hdc, font as HGDIOBJ);
    SetTextColor(hdc, rgb(232, 236, 242));
    let mut text_rc = RECT {
        left: x + 22,
        top: y + 6,
        right: x + w - 4,
        bottom: y + h - 2,
    };
    let wide_label = wide(label);
    DrawTextW(
        hdc,
        wide_label.as_ptr(),
        (wide_label.len() - 1) as i32,
        &mut text_rc,
        DT_LEFT | DT_NOPREFIX,
    );
    DeleteObject(face_brush as HGDIOBJ);
    DeleteObject(border_brush as HGDIOBJ);
}

fn write_png_rgba(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    let mut out = Vec::new();
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA
    write_png_chunk(&mut out, b"IHDR", &ihdr);

    let stride = (width as usize) * 4;
    let mut raw = Vec::with_capacity((stride + 1) * (height as usize));
    for row in rgba.chunks_exact(stride) {
        raw.push(0); // filter type 0 (None)
        raw.extend_from_slice(row);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw)?;
    let compressed = encoder.finish()?;
    write_png_chunk(&mut out, b"IDAT", &compressed);
    write_png_chunk(&mut out, b"IEND", &[]);

    fs::write(path, out)
}

fn write_png_chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for &b in tag.iter().chain(data.iter()) {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}
