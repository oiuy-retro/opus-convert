//! Windows 2.03 wizard. Flat black frames, 50% checkerboard dither.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::Instant;

use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateRectRgn,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawEdge, DrawTextW, EndPaint, FillRect, GetDC, GetStockObject,
    GetDeviceCaps, GetMonitorInfoW, GetSysColor, InvalidateRect, MapWindowPoints, MonitorFromWindow,
    RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RedrawWindow, ReleaseDC, ScreenToClient, SelectObject,
    SetBkColor, SetBkMode, SetTextColor, SetWindowRgn, UpdateWindow, BF_RECT, COLOR_BTNTEXT,
    COLOR_GRAYTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, EDGE_ETCHED, HBRUSH, HDC, HFONT, HRGN, LOGPIXELSX,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, SRCCOPY, TRANSPARENT, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
    DT_RIGHT, DT_SINGLELINE, DT_VCENTER,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, SetScrollInfo, ShowScrollBar, ICC_BAR_CLASSES, ICC_LISTVIEW_CLASSES,
    ICC_STANDARD_CLASSES, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX, LVM_EDITLABELW, LVM_GETITEMCOUNT,
    LVM_GETITEMTEXTW, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETITEMTEXTW, LVM_SETEXTENDEDLISTVIEWSTYLE,
    LVN_ENDLABELEDITW, LVS_EX_FULLROWSELECT, NMITEMACTIVATE, NM_CLICK, NM_DBLCLK, PBM_SETPOS,
    PBM_SETRANGE32, TTM_ADDTOOLW, TTM_SETDELAYTIME, TTM_SETMAXTIPWIDTH, TTS_ALWAYSTIP, TTS_NOPREFIX,
    LVCF_FMT, LVCF_TEXT, LVCF_WIDTH, LVCFMT_LEFT, LVCOLUMNW, LVIF_TEXT, LVITEMW, NMHDR, NMLVDISPINFOW,
    TTF_IDISHWND, TTF_SUBCLASS, TTTOOLINFOW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, IsWindowEnabled, ReleaseCapture, SetCapture, SetFocus, VK_DOWN, VK_RETURN,
    VK_SHIFT, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileDialog, IShellItem, SHCreateItemFromParsingName, FOS_FORCEFILESYSTEM,
    FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, DrawIconEx, GetClientRect,
    GetClassNameW, GetCursorPos, GetDlgItem, GetMessageW, GetParent, GetSystemMetrics, GetWindow,
    GetWindowLongPtrW, GetWindowRect, IsWindowVisible, IsZoomed, KillTimer, LoadCursorW, LoadImageW,
    MessageBoxW, MoveWindow, PostQuitMessage, RegisterClassW, SendMessageW, SetTimer, SetWindowLongPtrW,
    SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage, WindowFromPoint, BM_GETCHECK,
    BM_SETCHECK, CB_ADDSTRING, CB_GETCURSEL, CB_RESETCONTENT, CB_SETCURSEL, CBN_SELCHANGE, CS_DBLCLKS,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, DI_NORMAL, EN_CHANGE, EN_KILLFOCUS, GW_CHILD,
    GW_HWNDNEXT, GWL_STYLE, GWLP_ID, HMENU, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT,
    HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, HWND_TOP, IDC_ARROW, IMAGE_ICON, MB_ICONEXCLAMATION,
    MB_OK, MINMAXINFO, MSG, NCCALCSIZE_PARAMS, SB_VERT, SCROLLINFO, SIF_PAGE, SIF_POS, SIF_RANGE,
    SM_CXFRAME, SM_CXPADDEDBORDER, SM_CXSCREEN, SM_CYFRAME, SM_CYSCREEN, SW_MAXIMIZE, SW_MINIMIZE,
    SW_RESTORE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WM_ACTIVATE, WM_CLOSE, WM_COMMAND,
    WM_DESTROY, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_GETTEXT, WM_GETTEXTLENGTH, WM_KEYDOWN,
    WM_CAPTURECHANGED, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCALCSIZE,
    WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NOTIFY, WM_PAINT, WM_SETFONT, WM_SIZE, WM_TIMER, WM_VSCROLL,
    WNDCLASSW, WS_CLIPCHILDREN, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_THICKFRAME,
};

use crate::aero::{self, CapHot};

use crate::model::{self, can_advance, GenrePolicy, Group, Options};
use crate::names::{self, InsertAt, TextTools};
use crate::paths;
use crate::readout;
use crate::run::{self, Live, RowKind, Shared, WorkRow};
use crate::scan;
use crate::store::{self, Memory, SavedProfile};
use crate::tools;

const ID_BACK: i32 = 1;
const ID_NEXT: i32 = 2;
const ID_PROFILE: i32 = 10;
const ID_SAVE: i32 = 11;
const ID_NAME: i32 = 12;
const ID_AUDIO: i32 = 18;
const ID_VIDEO: i32 = 19;
const ID_INPUT: i32 = 22;
const ID_BROWSE_IN: i32 = 23;
const ID_OUTPUT: i32 = 24;
const ID_BROWSE_OUT: i32 = 25;
const ID_GENRE: i32 = 26;
const ID_GENRE_REPLACE: i32 = 27;
const ID_GENRE_KEEP: i32 = 28;
const ID_GENRE_YOUTUBE: i32 = 29;
const ID_LASTFM_KEY: i32 = 30;
const ID_LASTFM: i32 = 54;
const ID_LASTFM_PCT: i32 = 55;
const ID_MUSICNN: i32 = 56;
const ID_MUSICNN_PCT: i32 = 57;
const ID_STATUS: i32 = 50;
const ID_PROGRESS: i32 = 51;
const ID_PERCENT: i32 = 53;
const ID_READOUT: i32 = 60;
const ID_REMOVE: i32 = 1000;
const ID_INSERT: i32 = 2000;
const ID_MODE: i32 = 3000;
const ID_FIND: i32 = 4000;
const ID_STRIP: i32 = 5000;
const ID_APPLY: i32 = 6000;
const ID_LIST: i32 = 7000;
const ID_ALBUM: i32 = 8000;
const ID_ARTIST: i32 = 9000;
const ID_RESET: i32 = 16000;
const ID_META_RESET: i32 = 17000;
const ID_NAME_ERROR: i32 = 18000;
const ID_FOLDER: i32 = 19000;
const ID_TITLE_SAME_ALL: i32 = 21000;
const ID_TITLE_FIELD: i32 = 22000;
const ID_TITLE_SAME: i32 = 43000;
const GROUP_LIMIT: usize = 1000;

const PAGE_NAMES: [&str; 5] = ["Options", "Names", "Titles", "Metadata", "Convert"];

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
    static FACE: Cell<HBRUSH> = const { Cell::new(HBRUSH(std::ptr::null_mut())) };
    static WHITE: Cell<HBRUSH> = const { Cell::new(HBRUSH(std::ptr::null_mut())) };
    static WINDOW_BRUSH: Cell<HBRUSH> = const { Cell::new(HBRUSH(std::ptr::null_mut())) };
    static WORK_DRAG: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Busy {
    Idle,
    Scan,
    Convert,
}

enum Action {
    None,
    Pass,
    Result(isize),
    Popup(String),
    BrowseInput,
    BrowseOutput,
    StartScan,
    StartConvert,
    Cancel,
    Close,
    Min,
    Max,
    DragCaption,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Anchor {
    Fixed,
    StretchX,
    Right,
    Fill,
}

struct Kid {
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    anchor: Anchor,
    right: i32,
    bottom: i32,
}

struct App {
    hwnd: HWND,
    host: HWND,
    sheet: HWND,
    back: HWND,
    next: HWND,
    tip: HWND,
    instance: HINSTANCE,
    dpi: i32,
    font: HFONT,
    title_font: HFONT,
    page: usize,
    memory: Memory,
    options: Options,
    active_profile: String,
    loading: bool,
    kids: Vec<Kid>,
    scroll: i32,
    content_height: i32,
    groups: Vec<Group>,
    name_tools: Vec<TextTools>,
    title_tools: Vec<TextTools>,
    scan_ok: bool,
    scan_stamp: Options,
    convert_done: bool,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    shared: Option<std::sync::Arc<Shared>>,
    worker: Option<std::thread::JoinHandle<()>>,
    busy: Busy,
    rows: Vec<WorkRow>,
    list_hwnd: HWND,
    list_scroll: i32,
    samples: VecDeque<(Instant, u64)>,
    item_ticks: VecDeque<Instant>,
    sample_start: Instant,
    last_done: u32,
    started: Option<Instant>,
    status_hwnd: HWND,
    progress_hwnd: HWND,
    percent_hwnd: HWND,
    lastfm_hwnd: HWND,
    lastfm_percent: HWND,
    musicnn_hwnd: HWND,
    musicnn_percent: HWND,
    readout: [HWND; 5],
    profile_hwnd: HWND,
    name_hwnd: HWND,
    input_hwnd: HWND,
    output_hwnd: HWND,
    audio_hwnd: HWND,
    video_hwnd: HWND,
    genre_hwnd: HWND,
    genre_replace_hwnd: HWND,
    genre_keep_hwnd: HWND,
    genre_youtube_hwnd: HWND,
    lastfm_key_hwnd: HWND,
    lastfm_key_label: HWND,
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    caption_font: HFONT,
    active: bool,
    cap_hot: CapHot,
    cap_press: CapHot,
    sb_drag: Option<(i32, i32)>,
}

pub fn run() -> i32 {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES | ICC_STANDARD_CLASSES | ICC_BAR_CLASSES | ICC_LISTVIEW_CLASSES,
        };
        let _ = InitCommonControlsEx(&icc);
        let module = GetModuleHandleW(None).unwrap_or_default();
        let instance = HINSTANCE(module.0);
        crate::aero::register(instance);
        let face = CreateSolidBrush(rgb(255, 255, 255));
        let white = CreateSolidBrush(rgb(255, 255, 255));
        let window_brush = CreateSolidBrush(COLORREF(GetSysColor(COLOR_WINDOW)));
        FACE.set(face);
        WHITE.set(white);
        WINDOW_BRUSH.set(window_brush);
        register_classes(instance, face);
        APP.with(|slot| *slot.borrow_mut() = Some(App::boot(instance)));
        let hwnd = create_main(instance);
        if hwnd.0.is_null() {
            return 1;
        }
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
        let mut msg = MSG::default();
        loop {
            let got = GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0);
            if got.0 <= 0 {
                break;
            }
            if wheel(&msg) {
                continue;
            }
            if shortcut(&msg) {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = CoUninitialize();
        0
    }
}

fn register_classes(instance: HINSTANCE, face: HBRUSH) {
    unsafe {
        let icon = load_icon(instance, 32);
        let small = load_icon(instance, 16);
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let main = WNDCLASSW {
            style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: face,
            lpszClassName: w!("OpusConvertWindow"),
            ..Default::default()
        };
        let host = WNDCLASSW {
            lpfnWndProc: Some(host_proc),
            hInstance: instance,
            hCursor: cursor,
            hbrBackground: face,
            lpszClassName: w!("OpusConvertHost"),
            ..Default::default()
        };
        let list = WNDCLASSW {
            lpfnWndProc: Some(list_proc),
            hInstance: instance,
            hCursor: cursor,
            hbrBackground: white_brush(),
            lpszClassName: w!("OpusWorkList"),
            ..Default::default()
        };
        let _ = RegisterClassW(&main);
        let _ = RegisterClassW(&host);
        let _ = RegisterClassW(&list);
        let _ = small;
    }
}

fn load_icon(instance: HINSTANCE, px: i32) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    unsafe {
        let handle = LoadImageW(instance, PCWSTR(1 as *const u16), IMAGE_ICON, px, px, Default::default());
        windows::Win32::UI::WindowsAndMessaging::HICON(handle.unwrap_or_default().0)
    }
}

fn frame_style() -> windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE {
    WS_OVERLAPPED | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_THICKFRAME | WS_CLIPCHILDREN
}

fn minimum_window() -> (i32, i32) {
    let dpi = screen_dpi();
    (scale(640, dpi), scale(480, dpi))
}

fn create_main(instance: HINSTANCE) -> HWND {
    let (w, h) = minimum_window();
    let style = frame_style();
    unsafe {
        let x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2;
        let y = (GetSystemMetrics(SM_CYSCREEN) - h) / 2;
        CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
            w!("OpusConvertWindow"),
            w!("Opus Convert"),
            style,
            x,
            y,
            w,
            h,
            HWND(std::ptr::null_mut()),
            HMENU(std::ptr::null_mut()),
            instance,
            None,
        )
        .unwrap_or_default()
    }
}

fn screen_dpi() -> i32 {
    unsafe {
        let dc = GetDC(None);
        let dpi = GetDeviceCaps(dc, LOGPIXELSX);
        let _ = ReleaseDC(None, dc);
        if dpi <= 0 {
            96
        } else {
            dpi
        }
    }
}

fn scale(px: i32, dpi: i32) -> i32 {
    px * dpi / 96
}

unsafe fn nccalcsize(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if wparam.0 != 0 {
        let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
        if IsZoomed(hwnd).as_bool() || rect_covers_work(hwnd, params.rgrc[0]) {
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut mi).as_bool() {
                params.rgrc[0] = mi.rcWork;
            } else {
                let pad = GetSystemMetrics(SM_CXFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                let pady = GetSystemMetrics(SM_CYFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                params.rgrc[0].left += pad;
                params.rgrc[0].top += pady;
                params.rgrc[0].right -= pad;
                params.rgrc[0].bottom -= pady;
            }
        }
    }
    LRESULT(0)
}

fn rect_covers_work(hwnd: HWND, proposed: RECT) -> bool {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut mi).as_bool() {
            return false;
        }
        let pw = proposed.right - proposed.left;
        let ph = proposed.bottom - proposed.top;
        let ww = mi.rcWork.right - mi.rcWork.left;
        let wh = mi.rcWork.bottom - mi.rcWork.top;
        pw >= ww && ph >= wh
    }
}

fn fill_minmax(hwnd: HWND, lparam: LPARAM) {
    unsafe {
        let info = &mut *(lparam.0 as *mut MINMAXINFO);
        let (w, h) = minimum_window();
        info.ptMinTrackSize.x = w;
        info.ptMinTrackSize.y = h;
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut mi).as_bool() {
            info.ptMaxPosition.x = mi.rcWork.left - mi.rcMonitor.left;
            info.ptMaxPosition.y = mi.rcWork.top - mi.rcMonitor.top;
            info.ptMaxSize.x = mi.rcWork.right - mi.rcWork.left;
            info.ptMaxSize.y = mi.rcWork.bottom - mi.rcWork.top;
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCALCSIZE {
        return nccalcsize(hwnd, wparam, lparam);
    }
    if msg == WM_GETMINMAXINFO {
        fill_minmax(hwnd, lparam);
        return LRESULT(0);
    }
    if matches!(msg, 0x0138 | 0x0135 | 0x0133) {
        return paint_control(msg, wparam, lparam);
    }
    if IN_CALLBACK.get() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let action = APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else {
            return Action::Pass;
        };
        let Some(app) = borrow.as_mut() else {
            return Action::Pass;
        };
        app.handle(hwnd, msg, wparam, lparam)
    });
    if let Action::Pass = action {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    if let Action::Result(value) = action {
        return LRESULT(value);
    }
    run_action(action);
    LRESULT(0)
}

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_ERASEBKGND {
        let hdc = HDC(wparam.0 as *mut _);
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        let _ = FillRect(hdc, &rect, FACE.get());
        return LRESULT(1);
    }
    if matches!(msg, 0x0138 | 0x0135 | 0x0133 | 0x0134) {
        return paint_control(msg, wparam, lparam);
    }
    // WM_DRAWITEM: items of the Windows 3.0 style drop-down lists.
    if msg == 0x002B && crate::aero::draw_combo_item(lparam) {
        return LRESULT(1);
    }
    // WM_MEASUREITEM: a sensible default; App::size_combo_items fixes it up
    // once the font is known.
    if msg == 0x002C && lparam.0 != 0 {
        let m = &mut *(lparam.0 as *mut windows::Win32::UI::Controls::MEASUREITEMSTRUCT);
        if m.CtlType.0 == 3 {
            m.itemHeight = 18;
            return LRESULT(1);
        }
    }
    if matches!(msg, WM_COMMAND | WM_NOTIFY | WM_VSCROLL | WM_MOUSEWHEEL) && !IN_CALLBACK.get() {
        let parent = GetParent(hwnd).unwrap_or(HWND(std::ptr::null_mut()));
        return SendMessageW(parent, msg, wparam, lparam);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn list_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if IN_CALLBACK.get() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    match msg {
        WM_PAINT => {
            paint_work(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_VSCROLL => {
            scroll_work(hwnd, wparam);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16 as i32) / 120;
            nudge_work(hwnd, -delta * 3);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            if list_scrollbar_click(hwnd, lparam) {
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_MOUSEMOVE if drag_work(hwnd, lparam) => LRESULT(0),
        WM_LBUTTONUP | WM_CAPTURECHANGED if WORK_DRAG.get().is_some() => {
            WORK_DRAG.set(None);
            if msg == WM_LBUTTONUP {
                let _ = ReleaseCapture();
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn paint_control(msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let hdc = HDC(wparam.0 as *mut _);
        let child = HWND(lparam.0 as *mut _);
        let enabled = IsWindowEnabled(child).0 != 0;
        if msg == 0x0133 && enabled {
            let _ = SetTextColor(hdc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
            let _ = SetBkColor(hdc, COLORREF(GetSysColor(COLOR_WINDOW)));
            return LRESULT(WINDOW_BRUSH.get().0 as isize);
        }
        let color = if enabled { COLOR_BTNTEXT } else { COLOR_GRAYTEXT };
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, COLORREF(0));
        let _ = SetBkColor(hdc, COLORREF(0x00FFFFFF));
        LRESULT(FACE.get().0 as isize)
    }
}

fn run_action(action: Action) {
    match action {
        Action::None | Action::Pass | Action::Result(_) => {}
        Action::Popup(text) => popup(&text),
        Action::BrowseInput => browse(true),
        Action::BrowseOutput => browse(false),
        Action::StartScan => start_scan(),
        Action::StartConvert => start_convert(),
        Action::Cancel => cancel_work(),
        Action::Close => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
        Action::Min => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
        }
        Action::Max => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe {
                let cmd = if IsZoomed(hwnd).as_bool() { SW_RESTORE } else { SW_MAXIMIZE };
                let _ = ShowWindow(hwnd, cmd);
            }
        }
        Action::DragCaption => {
            let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
            unsafe {
                let _ = ReleaseCapture();
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let packed = LPARAM((((pt.y as u32) << 16) | (pt.x as u32 & 0xFFFF)) as isize);
                let _ = SendMessageW(hwnd, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), packed);
            }
        }
    }
}

fn popup(text: &str) {
    let body = wide(text);
    let title = wide("Opus Convert");
    let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
    unsafe {
        MessageBoxW(hwnd, PCWSTR(body.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONEXCLAMATION);
    }
}

fn browse(input: bool) {
    let (hwnd, current) = APP.with(|slot| {
        let app = slot.borrow();
        let app = app.as_ref().unwrap();
        let current = if input {
            get_text(app.input_hwnd)
        } else {
            get_text(app.output_hwnd)
        };
        (app.hwnd, current)
    });
    let title = if input { "Input" } else { "Output" };
    // Open at this button's last-used folder from configuration.conf
    // ([folders] input / output), or its nearest existing parent. Otherwise
    // open where the box points, as before.
    let key = if input { "input" } else { "output" };
    let conf = crate::config::load();
    let start = tools::exe_dir()
        .ok()
        .and_then(|dir| crate::config::browse_start(&conf, key, &dir))
        .map(|dir| paths::display_path(&dir))
        .unwrap_or(current);
    let Some(path) = pick_folder(hwnd, &start, title) else {
        return;
    };
    let _ = crate::config::remember_folder(&conf, key, &path);
    APP.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let app = borrow.as_mut().unwrap();
        if input {
            set_text(app.input_hwnd, &path);
        } else {
            set_text(app.output_hwnd, &path);
        }
        app.commit_folders();
        app.refresh_nav();
        store::save(&app.memory);
    });
}

fn start_scan() {
    let hwnd = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd).unwrap_or_default());
    unsafe {
        let _ = UpdateWindow(hwnd);
    }
    match tools::load_bundle() {
        Err(err) => {
            popup(&err);
            APP.with(|slot| {
                let mut borrow = slot.borrow_mut();
                let app = borrow.as_mut().unwrap();
                app.busy = Busy::Idle;
                app.page = 0;
                app.rebuild();
            });
        }
        Ok(bundle) => {
            APP.with(|slot| {
                let mut borrow = slot.borrow_mut();
                let app = borrow.as_mut().unwrap();
                app.ffmpeg = bundle.ffmpeg;
                app.ffprobe = bundle.ffprobe;
                app.spawn(Busy::Scan);
            });
        }
    }
}

fn start_convert() {
    APP.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let app = borrow.as_mut().unwrap();
        if app.convert_done {
            app.refresh_nav();
            return;
        }
        app.convert_done = false;
        app.spawn(Busy::Convert);
    });
}

fn cancel_work() {
    let (shared, worker) = APP.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let app = borrow.as_mut().unwrap();
        if let Some(shared) = &app.shared {
            run::kill_child(shared);
        }
        (app.shared.clone(), app.worker.take())
    });
    if let Some(worker) = worker {
        let _ = worker.join();
    }
    let _ = shared;
    APP.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let app = borrow.as_mut().unwrap();
        app.after_cancel();
    });
}

impl App {
    fn boot(instance: HINSTANCE) -> Self {
        let dpi = screen_dpi();
        let memory = store::load();
        let active = memory.last_profile.clone();
        let mut options = memory
            .profiles
            .iter()
            .find(|profile| profile.name == active)
            .map(|profile| profile.options.clone())
            .unwrap_or_default();
        if memory.override_profile == active {
            if !memory.override_input.is_empty() {
                options.input = memory.override_input.clone();
            }
            if !memory.override_output.is_empty() {
                options.output = memory.override_output.clone();
            }
        }
        let null = HWND(std::ptr::null_mut());
        Self {
            hwnd: null,
            host: null,
            sheet: null,
            back: null,
            next: null,
            tip: null,
            instance,
            dpi,
            font: make_font(10, dpi, true),
            title_font: make_font(12, dpi, true),
            page: 0,
            memory,
            options,
            active_profile: active,
            loading: false,
            kids: Vec::new(),
            scroll: 0,
            content_height: 0,
            groups: Vec::new(),
            name_tools: Vec::new(),
            title_tools: Vec::new(),
            scan_ok: false,
            scan_stamp: Options::default(),
            convert_done: false,
            ffmpeg: PathBuf::new(),
            ffprobe: PathBuf::new(),
            shared: None,
            worker: None,
            busy: Busy::Idle,
            rows: Vec::new(),
            list_hwnd: null,
            list_scroll: 0,
            samples: VecDeque::new(),
            item_ticks: VecDeque::new(),
            sample_start: Instant::now(),
            last_done: 0,
            started: None,
            status_hwnd: null,
            progress_hwnd: null,
            percent_hwnd: null,
            lastfm_hwnd: null,
            lastfm_percent: null,
            musicnn_hwnd: null,
            musicnn_percent: null,
            readout: [null; 5],
            profile_hwnd: null,
            name_hwnd: null,
            input_hwnd: null,
            output_hwnd: null,
            audio_hwnd: null,
            video_hwnd: null,
            genre_hwnd: null,
            genre_replace_hwnd: null,
            genre_keep_hwnd: null,
            genre_youtube_hwnd: null,
            lastfm_key_hwnd: null,
            lastfm_key_label: null,
            icon: load_icon(instance, 16),
            caption_font: aero::caption_font(dpi),
            active: true,
            cap_hot: CapHot::None,
            cap_press: CapHot::None,
            sb_drag: None,
        }
    }

    fn s(&self, px: i32) -> i32 {
        scale(px, self.dpi)
    }

    fn handle(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Action {
        match msg {
            0x0001 => {
                self.hwnd = hwnd;
                self.on_create();
                Action::None
            }
            WM_SIZE => {
                if wparam.0 != 1 {
                    unsafe {
                        SendMessageW(self.hwnd, 0x000B, WPARAM(0), LPARAM(0));
                    }
                    self.apply_region();
                    aero::enable_glass(self.hwnd, self.active);
                    self.layout_chrome();
                    self.update_scroll();
                    self.place();
                    self.place_sheet();
                    unsafe {
                        SendMessageW(self.hwnd, 0x000B, WPARAM(1), LPARAM(0));
                    }
                    self.redraw();
                }
                Action::None
            }
            WM_PAINT => {
                self.paint_chrome();
                Action::None
            }
            WM_ERASEBKGND => Action::Result(1),
            x if x == WM_NCHITTEST => self.hit_test(lparam),
            x if x == WM_ACTIVATE => {
                self.active = (wparam.0 & 0xFFFF) != 0;
                aero::enable_glass(self.hwnd, self.active);
                self.invalidate_caption();
                Action::None
            }
            x if x == WM_MOUSEMOVE => self.cap_move(lparam),
            x if x == WM_LBUTTONDOWN => self.cap_down(lparam),
            x if x == WM_LBUTTONUP => self.cap_up(),
            x if x == WM_LBUTTONDBLCLK => self.cap_dbl(lparam),
            WM_TIMER => self.on_timer(),
            WM_COMMAND => self.on_command(wparam, lparam),
            WM_NOTIFY => self.on_notify(lparam),
            WM_VSCROLL => {
                self.on_scroll(wparam);
                self.redraw();
                Action::None
            }
            WM_MOUSEWHEEL => {
                let delta = ((wparam.0 >> 16) as i16 as i32) / 120;
                self.scroll_by(-delta * self.s(36));
                Action::None
            }
            WM_CLOSE => Action::Close,
            WM_DESTROY => {
                self.shutdown();
                Action::None
            }
            _ => Action::Pass,
        }
    }

    fn on_create(&mut self) {
        let host = self.make(
            self.hwnd,
            "OpusConvertHost",
            "",
            0x40000000 | 0x10000000 | 0x02000000 | 0x04000000,
            0,
            0,
            0,
            10,
            10,
            0,
        );
        self.host = host;
        self.sheet = self.create_sheet();
        self.back = self.make(self.hwnd, "OpusAeroButton", "< &Back", button_bits(false), 0, 0, 0, 75, 23, ID_BACK);
        self.next = self.make(self.hwnd, "OpusAeroButton", "Next >", button_bits(true), 0, 0, 0, 75, 23, ID_NEXT);
        unsafe {
            let _ = SetWindowPos(self.back, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = SetWindowPos(self.next, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = SetTimer(self.hwnd, 1, 100, None);
            SendMessageW(self.hwnd, 0x0080, WPARAM(1), LPARAM(load_icon(self.instance, 32).0 as isize));
            SendMessageW(self.hwnd, 0x0080, WPARAM(0), LPARAM(load_icon(self.instance, 16).0 as isize));
        }
        self.apply_region();
        aero::enable_glass(self.hwnd, true);
        self.layout_chrome();
        self.loading = true;
        self.rebuild();
        self.loading = false;
        self.tip_nav();
    }

    fn chrome(&self) -> aero::Layout {
        let (w, h) = client_of(self.hwnd);
        aero::layout(
            RECT { left: 0, top: 0, right: w, bottom: h },
            self.dpi,
            self.maximized(),
        )
    }

    fn maximized(&self) -> bool {
        unsafe { IsZoomed(self.hwnd).as_bool() }
    }

    fn apply_region(&self) {
        let (w, h) = client_of(self.hwnd);
        aero::apply_region(self.hwnd, w, h, self.dpi, self.maximized());
    }

    fn layout_chrome(&self) {
        let chrome = self.chrome();
        let header = self.s(46);
        let bar = self.s(46);
        let x = chrome.inner.left;
        let y = chrome.inner.top;
        let w = (chrome.inner.right - chrome.inner.left).max(0);
        let h = (chrome.inner.bottom - chrome.inner.top).max(0);
        unsafe {
            let need_sb = self.content_height > (h - header - bar).max(1);
            let sb = if need_sb { crate::aero::scroll_width(self.dpi) } else { 0 };
            let _ = MoveWindow(self.host, x, y + header, (w - sb).max(0), (h - header - bar).max(0), false);
            let bw = self.s(90);
            let bh = self.s(28);
            let by = chrome.inner.bottom - bar + (bar - bh) / 2;
            let next_x = chrome.inner.right - self.s(8) - bw;
            let back_x = next_x - self.s(6) - bw;
            let _ = MoveWindow(self.back, back_x, by, bw, bh, false);
            let _ = MoveWindow(self.next, next_x, by, bw, bh, false);
            let _ = SetWindowPos(self.back, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = SetWindowPos(self.next, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }

    fn paint_chrome(&self) {
        unsafe {
            let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
            let hdc = BeginPaint(self.hwnd, &mut ps);
            let (w, h) = client_of(self.hwnd);
            let chrome = self.chrome();
            let header = self.s(46);
            let bar = self.s(46);
            let body = RECT {
                left: chrome.inner.left,
                top: chrome.caption.bottom,
                right: chrome.inner.right,
                bottom: chrome.inner.bottom,
            };
            let _ = FillRect(hdc, &body, FACE.get());
            aero::window_frame(hdc, chrome.window, chrome.border, self.active);
            aero::paint_caption(
                hdc,
                self.hwnd,
                &chrome,
                "Opus Convert",
                self.caption_font,
                self.active,
                self.maximized(),
                self.cap_hot,
                self.cap_press,
            );
            // Control-menu box is painted in paint_caption. Do not stamp the app icon over it.
            let head = RECT {
                left: chrome.inner.left,
                top: chrome.inner.top,
                right: chrome.inner.right,
                bottom: chrome.inner.top + header,
            };
            if header > 0 {
                let cyan = CreateSolidBrush(rgb(0, 255, 255));
                let _ = FillRect(hdc, &head, cyan);
                let _ = DeleteObject(cyan);
                let old = SelectObject(hdc, self.title_font);
                let _ = SetBkMode(hdc, TRANSPARENT);
                let _ = SetTextColor(hdc, rgb(0, 0, 0));
                let mut title = RECT {
                    left: head.left + self.s(14),
                    top: head.top,
                    right: head.right - self.s(12),
                    bottom: head.bottom,
                };
                let mut name = wide(PAGE_NAMES[self.page]);
                let end = name.len().saturating_sub(1);
                let _ = DrawTextW(hdc, &mut name[..end], &mut title, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                SelectObject(hdc, old);
                let mut line = RECT {
                    left: head.left,
                    top: head.bottom - 1,
                    right: head.right,
                    bottom: head.bottom,
                };
                let br = CreateSolidBrush(rgb(0, 0, 0));
                let _ = FillRect(hdc, &line, br);
                let _ = DeleteObject(br);
            }
            let foot = RECT {
                left: chrome.inner.left,
                top: chrome.inner.bottom - bar,
                right: chrome.inner.right,
                bottom: chrome.inner.bottom,
            };
            let _ = FillRect(hdc, &foot, FACE.get());
            let br = CreateSolidBrush(rgb(0, 0, 0));
            let _ = FillRect(hdc, &RECT { left: foot.left, top: foot.top, right: foot.right, bottom: foot.top + 1 }, br);
            let _ = DeleteObject(br);
            if !self.maximized() {
                aero::gripper(hdc, RECT { left: 0, top: 0, right: w, bottom: h });
            }
            let host_w = chrome.inner.right - chrome.inner.left;
            if let Some(sb) = self.host_scrollbar() {
                if host_w > aero::scroll_width(self.dpi) {
                    aero::paint_scrollbar(hdc, &sb);
                }
            }
            let _ = EndPaint(self.hwnd, &ps);
        }
    }

    fn hit_test(&self, lparam: LPARAM) -> Action {
        if self.maximized() {
            return Action::Result(HTCLIENT as isize);
        }
        let mut pt = POINT {
            x: (lparam.0 as i16) as i32,
            y: ((lparam.0 >> 16) as i16) as i32,
        };
        unsafe {
            let _ = ScreenToClient(self.hwnd, &mut pt);
        }
        let chrome = self.chrome();
        if aero::is_button(aero::hot_at(&chrome, pt)) {
            return Action::Result(HTCLIENT as isize);
        }
        let r = chrome.window;
        let edge = chrome.border.max(3);
        let left = pt.x < r.left + edge;
        let right = pt.x >= r.right - edge;
        let top = pt.y < r.top + edge;
        let bottom = pt.y >= r.bottom - edge;
        let code = if top && left {
            HTTOPLEFT
        } else if top && right {
            HTTOPRIGHT
        } else if bottom && left {
            HTBOTTOMLEFT
        } else if bottom && right {
            HTBOTTOMRIGHT
        } else if left {
            HTLEFT
        } else if right {
            HTRIGHT
        } else if top {
            HTTOP
        } else if bottom {
            HTBOTTOM
        } else {
            HTCLIENT
        };
        Action::Result(code as isize)
    }

    fn cap_point(lparam: LPARAM) -> POINT {
        POINT {
            x: (lparam.0 as i16) as i32,
            y: ((lparam.0 >> 16) as i16) as i32,
        }
    }

    fn invalidate_caption(&self) {
        let caption = self.chrome().caption;
        unsafe {
            let _ = InvalidateRect(self.hwnd, Some(&caption), false);
        }
    }

    fn host_scrollbar(&self) -> Option<aero::Scrollbar> {
        let chrome = self.chrome();
        let header = self.s(46);
        let bar = self.s(46);
        let host_h = (chrome.inner.bottom - bar - (chrome.inner.top + header)).max(0);
        // The bar's top, right and bottom lines sit on the black header line,
        // window frame and footer line, as in a Windows 2.03 window.
        let u = aero::sb_unit(self.dpi);
        aero::scrollbar(
            RECT {
                left: chrome.inner.right - aero::scroll_width(self.dpi),
                top: chrome.inner.top + header - u,
                right: chrome.inner.right + u,
                bottom: chrome.inner.top + header + host_h + u,
            },
            self.scroll,
            host_h.max(1),
            self.content_height.max(1),
            self.dpi,
        )
    }

    fn cap_move(&mut self, lparam: LPARAM) -> Action {
        let p = Self::cap_point(lparam);
        if let Some((origin, start)) = self.sb_drag {
            if let Some(sb) = self.host_scrollbar() {
                let next = aero::drag_pos(&sb, start, p.y - origin);
                self.scroll = next.clamp(0, (self.content_height - self.view_h()).max(0));
                self.shift_kids();
                self.redraw();
            }
            return Action::None;
        }
        let hot = aero::hot_at(&self.chrome(), p);
        if hot == self.cap_hot {
            return Action::None;
        }
        let paint = aero::is_button(hot) || aero::is_button(self.cap_hot);
        self.cap_hot = hot;
        if paint {
            self.invalidate_caption();
        }
        Action::None
    }

    fn cap_down(&mut self, lparam: LPARAM) -> Action {
        let p = Self::cap_point(lparam);
        if let Some(sb) = self.host_scrollbar() {
            match aero::scrollbar_hit(&sb, p) {
                aero::SbHit::Up => {
                    self.scroll_by(-self.s(36));
                    self.redraw();
                    return Action::None;
                }
                aero::SbHit::Down => {
                    self.scroll_by(self.s(36));
                    self.redraw();
                    return Action::None;
                }
                aero::SbHit::PageUp => {
                    self.scroll_by(-self.view_h());
                    self.redraw();
                    return Action::None;
                }
                aero::SbHit::PageDown => {
                    self.scroll_by(self.view_h());
                    self.redraw();
                    return Action::None;
                }
                aero::SbHit::Thumb => {
                    self.sb_drag = Some((p.y, self.scroll));
                    return Action::None;
                }
                aero::SbHit::None => {}
            }
        }
        let hot = aero::hot_at(&self.chrome(), p);
        self.cap_press = hot;
        self.cap_hot = hot;
        match hot {
            CapHot::Caption => Action::DragCaption,
            CapHot::Min | CapHot::Max | CapHot::Close | CapHot::Icon => {
                self.invalidate_caption();
                Action::None
            }
            CapHot::None => Action::None,
        }
    }

    fn cap_up(&mut self) -> Action {
        self.sb_drag = None;
        let press = self.cap_press;
        self.cap_press = CapHot::None;
        let mut pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut pt);
            let _ = ScreenToClient(self.hwnd, &mut pt);
        }
        let hot = aero::hot_at(&self.chrome(), pt);
        self.cap_hot = hot;
        self.invalidate_caption();
        if press != hot {
            return Action::None;
        }
        match press {
            CapHot::Close | CapHot::Icon => Action::Close,
            CapHot::Min => Action::Min,
            CapHot::Max => Action::Max,
            _ => Action::None,
        }
    }

    fn cap_dbl(&mut self, lparam: LPARAM) -> Action {
        let hot = aero::hot_at(&self.chrome(), Self::cap_point(lparam));
        match hot {
            CapHot::Caption | CapHot::Max => Action::Max,
            CapHot::Icon => Action::Close,
            _ => Action::None,
        }
    }

    fn redraw(&self) {
        unsafe {
            let flags = RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN;
            let _ = RedrawWindow(self.hwnd, None, None, flags);
            if !self.host.0.is_null() {
                let _ = RedrawWindow(self.host, None, None, flags);
            }
            if !self.sheet.0.is_null() {
                let _ = RedrawWindow(self.sheet, None, None, flags);
            }
        }
    }

    fn rebuild(&mut self) {
        self.harvest_soft();
        self.clear_kids();
        self.scroll = 0;
        self.list_scroll = 0;
        let bottom = match self.page {
            0 => self.build_options(),
            1 => self.build_names(),
            2 => self.build_titles(),
            3 => self.build_meta(),
            _ => self.build_work(),
        };
        self.content_height = bottom + self.s(8);
        self.layout_chrome();
        self.update_scroll();
        self.place();
        self.place_sheet();
        self.refresh_nav();
        self.retip();
        self.redraw();
    }

    fn build_options(&mut self) -> i32 {
        let w = self.host_width();
        let mut y = self.s(10);
        let label_w = self.s(108);
        let x = self.s(8);
        let field_x = x + label_w + self.s(8);
        let field_w = self.s(200);
        self.label(x, y + self.s(3), label_w, "&Profile");
        self.profile_hwnd = self.combo(field_x, y, field_w, ID_PROFILE);
        for profile in &self.memory.profiles {
            combo_add(self.profile_hwnd, &profile.name);
        }
        let sel = self
            .memory
            .profiles
            .iter()
            .position(|profile| profile.name == self.active_profile)
            .unwrap_or(0);
        set_combo(self.profile_hwnd, sel);
        self.button(field_x + field_w + self.s(8), y, self.s(75), "&Save", ID_SAVE, false);
        y += self.s(28);
        self.label(x, y + self.s(3), label_w, "&Name");
        self.name_hwnd = self.edit(field_x, y, field_w + self.s(83), ID_NAME, false, false);
        set_text(self.name_hwnd, &self.active_profile);
        y += self.s(32);
        let browse_w = self.s(75);
        let loc_w = (w - field_x - browse_w - self.s(16)).max(self.s(80));
        self.label(x, y + self.s(3), label_w, "&Input");
        self.input_hwnd = self.edit(field_x, y, loc_w, ID_INPUT, false, false);
        set_text(self.input_hwnd, &self.options.input);
        self.button(field_x + loc_w + self.s(8), y, browse_w, "Bro&wse", ID_BROWSE_IN, false);
        y += self.s(28);
        self.label(x, y + self.s(3), label_w, "&Output");
        self.output_hwnd = self.edit(field_x, y, loc_w, ID_OUTPUT, false, false);
        set_text(self.output_hwnd, &self.options.output);
        self.button(field_x + loc_w + self.s(8), y, browse_w, "&Browse", ID_BROWSE_OUT, false);
        y += self.s(32);
        self.audio_hwnd = self.check(x, y, w - self.s(16), "&Audio files", ID_AUDIO, self.options.convert_audio);
        y += self.s(24);
        self.video_hwnd = self.check(x, y, w - self.s(16), "&Video files", ID_VIDEO, self.options.convert_video);
        y += self.s(24);
        self.genre_hwnd = self.check(
            x,
            y,
            w - self.s(16),
            "Look up &genre",
            ID_GENRE,
            self.options.lookup_genre,
        );
        y += self.s(22);
        let radio_x = x + self.s(18);
        let radio_w = w - self.s(34);
        let policy = self.options.genre_policy;
        self.genre_replace_hwnd = self.radio(
            radio_x,
            y,
            radio_w,
            "&Replace existing genre",
            ID_GENRE_REPLACE,
            policy == GenrePolicy::Replace,
            true,
        );
        y += self.s(20);
        self.genre_keep_hwnd = self.radio(
            radio_x,
            y,
            radio_w,
            "Keep existing genre on &audio",
            ID_GENRE_KEEP,
            policy == GenrePolicy::KeepAudio,
            false,
        );
        y += self.s(20);
        self.genre_youtube_hwnd = self.radio(
            radio_x,
            y,
            radio_w,
            "Keep existing genre on audio, except &YouTube categories",
            ID_GENRE_YOUTUBE,
            policy == GenrePolicy::KeepAudioExceptYoutube,
            false,
        );
        y += self.s(22);
        self.lastfm_key_label = self.label(radio_x, y + self.s(3), label_w, "Last.fm &key");
        let key_x = radio_x + label_w + self.s(8);
        let key_w = (w - key_x - self.s(16)).max(self.s(80));
        self.lastfm_key_hwnd = self.edit(key_x, y, key_w, ID_LASTFM_KEY, false, false);
        set_text(self.lastfm_key_hwnd, &crate::genre::key_for_box());
        self.set_anchor(self.lastfm_key_hwnd, Anchor::StretchX);
        y += self.s(28);
        self.sync_genre_radios();
        y
    }

    fn build_names(&mut self) -> i32 {
        if !self.scan_ok || self.busy == Busy::Scan {
            return self.build_wait("Reading the folder…");
        }
        self.fit_tools();
        let w = self.host_width();
        let mut y = self.s(8);
        if self.groups.iter().all(|group| group.items.is_empty()) {
            self.static_text(self.s(8), y, w - self.s(16), self.s(32), "No audio or video files were found.", false);
            return y + self.s(40);
        }
        let count = self.groups.len().min(GROUP_LIMIT);
        for group_index in 0..count {
            let rel = self.groups[group_index].rel.clone();
            let name = self.groups[group_index].name.clone();
            let titles: Vec<String> = self.groups[group_index].items.iter().map(|item| item.stem.clone()).collect();
            if !rel.is_empty() {
                self.static_text(self.s(8), y + self.s(3), self.s(52), self.s(20), "Folder", false);
                let folder = self.edit(self.s(64), y, w - self.s(72), ID_FOLDER + group_index as i32, false, false);
                set_text(folder, &name);
                y += self.s(28);
            }
            if titles.is_empty() {
                y += self.s(8);
                continue;
            }
            let tools = self.name_tools.get(group_index).cloned().unwrap_or_default();
            y = self.build_tool_row(group_index, y, w, &tools);
            let rows = titles.len().clamp(3, 8) as i32;
            let list_h = self.s(20) + rows * self.s(18) + self.s(4);
            let list = self.listview(self.s(8), y, w - self.s(16), list_h, ID_LIST + group_index as i32);
            fill_titles(list, &titles, w - self.s(24));
            y += list_h + self.s(4);
            let problem = paths::name_problem_line(std::slice::from_ref(&self.groups[group_index])).unwrap_or_default();
            let error = self.static_text(self.s(8), y, w - self.s(16), self.s(32), &problem, true);
            set_id(error, ID_NAME_ERROR + group_index as i32);
            y += self.s(36);
        }
        y
    }

    fn build_titles(&mut self) -> i32 {
        if !self.scan_ok || self.busy == Busy::Scan {
            return self.build_wait("Reading the folder…");
        }
        self.fit_tools();
        let w = self.host_width();
        let mut y = self.s(8);
        if self.groups.iter().all(|group| group.items.is_empty()) {
            self.static_text(self.s(8), y, w - self.s(16), self.s(32), "No audio or video files were found.", false);
            return y + self.s(40);
        }
        let mut flat = 0usize;
        let count = self.groups.len().min(GROUP_LIMIT);
        let same_w = self.s(132);
        let all_w = self.s(168);
        for group_index in 0..count {
            let label = self.groups[group_index].rel.clone();
            let rows: Vec<(String, String)> = self.groups[group_index]
                .items
                .iter()
                .map(|item| (item.stem.clone(), item.meta_title.clone()))
                .collect();
            if rows.is_empty() {
                continue;
            }
            if !label.is_empty() {
                self.static_text(self.s(8), y, w - self.s(16), self.s(18), &label, false);
                y += self.s(22);
            }
            let tools = self.title_tools.get(group_index).cloned().unwrap_or_default();
            y = self.build_tool_row(group_index, y, w, &tools);
            let all = self.button(
                w - self.s(8) - all_w,
                y,
                all_w,
                "&All same as file name",
                ID_TITLE_SAME_ALL + group_index as i32,
                false,
            );
            self.set_anchor(all, Anchor::Right);
            y += self.s(28);
            for (file_name, title) in rows {
                if ID_TITLE_SAME + flat as i32 > 65000 {
                    break;
                }
                self.make(
                    self.host,
                    "Static",
                    &file_name,
                    0x40000000 | 0x10000000 | 0x80 | 0x4000,
                    0,
                    self.s(8),
                    y,
                    w - self.s(16),
                    self.s(16),
                    0,
                );
                y += self.s(16);
                let edit_w = (w - self.s(16) - same_w - self.s(8)).max(self.s(80));
                let field = self.edit(self.s(8), y, edit_w, ID_TITLE_FIELD + flat as i32, false, false);
                self.set_anchor(field, Anchor::StretchX);
                set_text(field, &title);
                let same = self.button(
                    w - self.s(8) - same_w,
                    y,
                    same_w,
                    "Same as file name",
                    ID_TITLE_SAME + flat as i32,
                    false,
                );
                self.set_anchor(same, Anchor::Right);
                y += self.s(26);
                flat += 1;
            }
            y += self.s(8);
        }
        y
    }

    fn build_tool_row(&mut self, group_index: usize, mut y: i32, w: i32, tools: &TextTools) -> i32 {
        let row_h = self.s(22);
        self.static_text(self.s(8), y + self.s(3), self.s(52), row_h, "Remove", false);
        let remove = self.edit(self.s(62), y, self.s(120), ID_REMOVE + group_index as i32, false, false);
        set_text(remove, &tools.remove);
        self.static_text(self.s(188), y + self.s(3), self.s(40), row_h, "Insert", false);
        let insert = self.edit(self.s(230), y, self.s(110), ID_INSERT + group_index as i32, false, false);
        set_text(insert, &tools.insert);
        let mode = self.combo(self.s(346), y, self.s(110), ID_MODE + group_index as i32);
        for label in ["At the start", "At the end", "Before", "After"] {
            combo_add(mode, label);
        }
        set_combo(mode, insert_index(tools.at));
        y += self.s(28);
        self.static_text(self.s(8), y + self.s(3), self.s(40), row_h, "Find", false);
        let find = self.edit(self.s(50), y, self.s(140), ID_FIND + group_index as i32, false, false);
        set_text(find, &tools.find);
        self.check(
            self.s(200),
            y,
            self.s(220),
            "Strip the prefix and the note",
            ID_STRIP + group_index as i32,
            tools.strip,
        );
        self.button(w - self.s(8) - self.s(156), y, self.s(75), "&Reset", ID_RESET + group_index as i32, false);
        self.button(w - self.s(8) - self.s(75), y, self.s(75), "Apply", ID_APPLY + group_index as i32, false);
        y += self.s(30);
        self.sync_apply(group_index);
        y
    }

    fn build_meta(&mut self) -> i32 {
        let w = self.host_width();
        let mut y = self.s(8);
        let mut flat = 0usize;
        let pages: Vec<(String, String, Vec<(String, String)>)> = self
            .groups
            .iter()
            .take(GROUP_LIMIT)
            .map(|group| {
                let label = group.rel.clone();
                (
                    label,
                    group.album.clone(),
                    group
                        .items
                        .iter()
                        .map(|item| (item.stem.clone(), item.artists.join("\r\n")))
                        .collect(),
                )
            })
            .collect();
        for (group_index, (label, album_text, rows)) in pages.iter().enumerate() {
            if rows.is_empty() {
                continue;
            }
            if !label.is_empty() {
                self.static_text(self.s(8), y, w - self.s(16), self.s(18), label, false);
                y += self.s(22);
            }
            self.static_text(self.s(8), y + self.s(3), self.s(52), self.s(20), "Album", false);
            let album_w = (w - self.s(64) - self.s(90)).max(self.s(80));
            let album = self.edit(self.s(64), y, album_w, ID_ALBUM + group_index as i32, false, false);
            set_text(album, album_text);
            self.button(self.s(64) + album_w + self.s(8), y, self.s(75), "&Reset", ID_META_RESET + group_index as i32, false);
            y += self.s(28);
            for (file_name, artists_text) in rows {
                if flat + ID_ARTIST as usize > 65000 {
                    break;
                }
                self.static_text(self.s(8), y, w - self.s(16), self.s(18), file_name, false);
                y += self.s(18);
                let artists = self.edit(self.s(8), y, w - self.s(16), ID_ARTIST + flat as i32, true, false);
                let height = self.s(48);
                let artist_w = w - self.s(16);
                unsafe {
                    let _ = MoveWindow(artists, self.s(8), y, artist_w, height, true);
                }
                if let Some(kid) = self.kids.last_mut() {
                    kid.h = height;
                    kid.w = artist_w;
                }
                set_text(artists, artists_text);
                y += height + self.s(8);
                flat += 1;
            }
            y += self.s(6);
        }
        y
    }

    fn build_work(&mut self) -> i32 {
        let w = self.host_width();
        let mut y = self.s(8);
        self.status_hwnd = self.static_text(self.s(8), y, w - self.s(16), self.s(18), "", false);
        set_id(self.status_hwnd, ID_STATUS);
        y += self.s(22);
        let extra = if self.options.lookup_genre { self.s(52) } else { 0 };
        let list_h = (self.view_h() - y - self.s(78) - extra).max(self.s(80));
        self.list_hwnd = self.make(
            self.host,
            "OpusWorkList",
            "",
            0x40000000 | 0x10000000 | 0x10000,
            0x200,
            self.s(8),
            y,
            w - self.s(16),
            list_h,
            52,
        );
        y += list_h + self.s(8);
        self.progress_hwnd = self.make(
            self.host,
            crate::aero::CLASS,
            "",
            0x40000000 | 0x10000000,
            0,
            self.s(8),
            y,
            w - self.s(70),
            self.s(18),
            ID_PROGRESS,
        );
        self.set_anchor(self.progress_hwnd, Anchor::StretchX);
        unsafe {
            SendMessageW(self.progress_hwnd, PBM_SETRANGE32, WPARAM(0), LPARAM(1000));
        }
        self.percent_hwnd = self.static_text(w - self.s(58), y, self.s(50), self.s(18), "0%", false);
        set_id(self.percent_hwnd, ID_PERCENT);
        self.set_anchor(self.percent_hwnd, Anchor::Right);
        y += self.s(22);
        if self.options.lookup_genre {
            self.static_text(self.s(8), y, self.s(72), self.s(16), "Last.fm", false);
            self.lastfm_hwnd = self.make(
                self.host,
                crate::aero::CLASS,
                "",
                0x40000000 | 0x10000000,
                0,
                self.s(84),
                y,
                w - self.s(146),
                self.s(18),
                ID_LASTFM,
            );
            unsafe {
                SendMessageW(self.lastfm_hwnd, PBM_SETRANGE32, WPARAM(0), LPARAM(1000));
            }
            self.lastfm_percent = self.static_text(w - self.s(58), y, self.s(50), self.s(18), "0%", false);
            set_id(self.lastfm_percent, ID_LASTFM_PCT);
            self.set_anchor(self.lastfm_hwnd, Anchor::StretchX);
            self.set_anchor(self.lastfm_percent, Anchor::Right);
            y += self.s(22);
            self.static_text(self.s(8), y, self.s(72), self.s(16), "musicnn", false);
            self.musicnn_hwnd = self.make(
                self.host,
                crate::aero::CLASS,
                "",
                0x40000000 | 0x10000000,
                0,
                self.s(84),
                y,
                w - self.s(146),
                self.s(18),
                ID_MUSICNN,
            );
            unsafe {
                SendMessageW(self.musicnn_hwnd, PBM_SETRANGE32, WPARAM(0), LPARAM(1000));
            }
            self.musicnn_percent = self.static_text(w - self.s(58), y, self.s(50), self.s(18), "0%", false);
            set_id(self.musicnn_percent, ID_MUSICNN_PCT);
            self.set_anchor(self.musicnn_hwnd, Anchor::StretchX);
            self.set_anchor(self.musicnn_percent, Anchor::Right);
            y += self.s(22);
        }
        let labels = ["Speed", "Items per second", "Files left", "Time left", "Elapsed"];
        let col_w = ((w - self.s(16)) / 2).max(self.s(260));
        let label_w = self.s(128);
        let value_w = (col_w - label_w - self.s(8)).max(self.s(110));
        let row_h = self.s(22);
        for (index, label) in labels.iter().enumerate() {
            let col = (index % 2) as i32;
            let row = (index / 2) as i32;
            let lx = self.s(8) + col * col_w;
            let ly = y + row * row_h;
            self.static_text(lx, ly, label_w, row_h, label, false);
            self.readout[index] = self.static_text(lx + label_w, ly, value_w, row_h, "--", false);
            set_id(self.readout[index], ID_READOUT + index as i32);
        }
        y + row_h * 3 + self.s(4)
    }

    fn build_wait(&mut self, text: &str) -> i32 {
        let w = self.host_width();
        let y = self.s(12);
        self.status_hwnd = self.static_text(self.s(8), y, w - self.s(16), self.s(18), text, false);
        let bar_y = y + self.s(24);
        self.progress_hwnd = self.make(
            self.host,
            crate::aero::CLASS,
            "",
            0x40000000 | 0x10000000,
            0,
            self.s(8),
            bar_y,
            w - self.s(16),
            self.s(18),
            ID_PROGRESS,
        );
        unsafe {
            SendMessageW(self.progress_hwnd, PBM_SETRANGE32, WPARAM(0), LPARAM(1000));
        }
        bar_y + self.s(24)
    }

    fn label(&mut self, x: i32, y: i32, w: i32, text: &str) -> HWND {
        self.static_text(x, y, w, self.s(16), text, false)
    }

    fn static_text(&mut self, x: i32, y: i32, w: i32, h: i32, text: &str, prefix_off: bool) -> HWND {
        let bits = if prefix_off { 0x800 } else { 0 };
        self.make(self.host, "Static", text, 0x40000000 | 0x10000000 | bits, 0, x, y, w, h, 0)
    }

    fn edit(&mut self, x: i32, y: i32, w: i32, id: i32, multi: bool, number: bool) -> HWND {
        let mut bits = 0x40000000 | 0x10000000 | 0x10000 | 0x0080;
        if multi {
            bits |= 0x0004 | 0x0040 | 0x1000 | 0x00200000;
        }
        if number {
            bits |= 0x2000;
        }
        self.make(self.host, "Edit", "", bits, 0, x, y, w, self.s(26), id)
    }

    fn combo(&mut self, x: i32, y: i32, w: i32, id: i32) -> HWND {
        // 0x10 = CBS_OWNERDRAWFIXED: the opened list's items are drawn by
        // aero::draw_combo_item in the Windows 3.0 style (black highlight).
        let hwnd = self.make(self.host, "ComboBox", "", 0x40000000 | 0x10000000 | 0x10000 | 0x3 | 0x10 | 0x200 | 0x00200000, 0, x, y, w, self.s(160), id);
        self.set_anchor(hwnd, Anchor::Fixed);
        self.size_combo_items(hwnd);
        crate::aero::style_combo_list(hwnd);
        self.clip_closed_combo(hwnd);
        hwnd
    }

    /// Owner-draw combos get their item heights from WM_MEASUREITEM before the
    /// font is set, so size them here: list rows fit the text, and the closed
    /// box keeps the app's usual 26px field height.
    fn size_combo_items(&self, hwnd: HWND) {
        if hwnd.0.is_null() {
            return;
        }
        unsafe {
            let font = HFONT(SendMessageW(hwnd, 0x0031, WPARAM(0), LPARAM(0)).0 as *mut _);
            let dc = GetDC(hwnd);
            let old = if !font.0.is_null() { SelectObject(dc, font) } else { windows::Win32::Graphics::Gdi::HGDIOBJ::default() };
            let mut tm = windows::Win32::Graphics::Gdi::TEXTMETRICW::default();
            let _ = windows::Win32::Graphics::Gdi::GetTextMetricsW(dc, &mut tm);
            if !font.0.is_null() {
                SelectObject(dc, old);
            }
            ReleaseDC(hwnd, dc);
            let row = (tm.tmHeight + self.s(2)).max(self.s(14));
            SendMessageW(hwnd, 0x0153, WPARAM(0), LPARAM(row as isize));
            // CB_SETITEMHEIGHT(-1) sets the box's text area; the frame adds a
            // few pixels, so set it, measure, and correct once.
            let want = self.s(26);
            let mut item = want - 6;
            SendMessageW(hwnd, 0x0153, WPARAM(usize::MAX), LPARAM(item as isize));
            let got = crate::aero::combo_face_height(hwnd);
            if got != want && got > 0 {
                item = (item + want - got).max(row);
                SendMessageW(hwnd, 0x0153, WPARAM(usize::MAX), LPARAM(item as isize));
            }
        }
    }

    fn clip_closed_combo(&self, hwnd: HWND) {
        if hwnd.0.is_null() {
            return;
        }
        let mut rect = RECT::default();
        unsafe {
            let _ = GetWindowRect(hwnd, &mut rect);
        }
        let width = (rect.right - rect.left).max(1);
        let height = crate::aero::combo_face_height(hwnd).max(self.s(26));
        unsafe {
            let region = CreateRectRgn(0, 0, width, height);
            if region.0.is_null() {
                return;
            }
            if SetWindowRgn(hwnd, region, BOOL(1)) == 0 {
                let _ = DeleteObject(region);
            }
        }
    }

    fn check(&mut self, x: i32, y: i32, w: i32, text: &str, id: i32, on: bool) -> HWND {
        let hwnd = self.make(self.host, "OpusWin2Check", text, 0x40000000 | 0x10000000 | 0x10000, 0, x, y, w, self.s(18), id);
        set_check(hwnd, on);
        hwnd
    }

    fn radio(&mut self, x: i32, y: i32, w: i32, text: &str, id: i32, on: bool, group: bool) -> HWND {
        let mut style = 0x40000000 | 0x10000000 | 0x10000 | 0x9;
        if group {
            style |= 0x00020000;
        }
        let hwnd = self.make(self.host, "OpusWin2Radio", text, style, 0, x, y, w, self.s(18), id);
        self.set_anchor(hwnd, Anchor::StretchX);
        set_check(hwnd, on);
        hwnd
    }

    fn sync_genre_radios(&self) {
        let on = !self.genre_hwnd.0.is_null() && checked(self.genre_hwnd);
        enable(self.genre_replace_hwnd, on);
        enable(self.genre_keep_hwnd, on);
        enable(self.genre_youtube_hwnd, on);
        enable(self.lastfm_key_hwnd, on);
        enable(self.lastfm_key_label, on);
    }

    fn persist_lastfm_key(&self) {
        if self.lastfm_key_hwnd.0.is_null() {
            return;
        }
        let _ = crate::genre::save_stored_key(&get_text(self.lastfm_key_hwnd));
    }

    fn selected_genre_policy(&self) -> GenrePolicy {
        if checked(self.genre_keep_hwnd) {
            GenrePolicy::KeepAudio
        } else if checked(self.genre_youtube_hwnd) {
            GenrePolicy::KeepAudioExceptYoutube
        } else {
            GenrePolicy::Replace
        }
    }

    fn button(&mut self, x: i32, y: i32, w: i32, text: &str, id: i32, default: bool) -> HWND {
        self.make(self.host, "OpusAeroButton", text, button_bits(default), 0, x, y, w, self.s(23), id)
    }

    fn listview(&mut self, x: i32, y: i32, w: i32, h: i32, id: i32) -> HWND {
        let bits = 0x40000000 | 0x10000000 | 0x10000 | 0x02000000 | 0x0001 | 0x0200 | 0x0004 | 0x0008 | 0x8000;
        self.make(self.host, "SysListView32", "", bits, 0, x, y, w, h, id)
    }

    fn make(&mut self, parent: HWND, class: &str, text: &str, style: u32, ex: u32, x: i32, y: i32, w: i32, h: i32, id: i32) -> HWND {
        let class_w = wide(class);
        let text_w = wide(text);
        let on_page = parent == self.host;
        let parent = if on_page && !self.sheet.0.is_null() { self.sheet } else { parent };
        IN_CALLBACK.set(true);
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(ex),
                PCWSTR(class_w.as_ptr()),
                PCWSTR(text_w.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(style),
                x,
                y,
                w.max(1),
                h.max(1),
                parent,
                HMENU(id as *mut _),
                self.instance,
                None,
            )
            .unwrap_or_default()
        };
        IN_CALLBACK.set(false);
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, WPARAM(font_bits(self.font)), LPARAM(1));
        }
        crate::aero::untheme(hwnd);
        crate::aero::thicken(hwnd);
        if on_page {
            let host_w = self.host_width().max(1);
            let view_h = self.view_h().max(1);
            let right = (host_w - (x + w)).max(0);
            let in_view = y >= 0 && y + h <= view_h + 2;
            let bottom = if in_view { view_h - (y + h) } else { 0 };
            let anchor = if in_view && h >= 100 && bottom <= 120 {
                Anchor::Fill
            } else if right <= 20 && w >= 160 {
                Anchor::StretchX
            } else if right <= 16 && w <= 110 {
                Anchor::Right
            } else {
                Anchor::Fixed
            };
            self.kids.push(Kid { hwnd, x, y, w, h, anchor, right, bottom });
        }
        hwnd
    }

    fn set_anchor(&mut self, hwnd: HWND, anchor: Anchor) {
        if let Some(kid) = self.kids.iter_mut().rev().find(|kid| kid.hwnd == hwnd) {
            kid.anchor = anchor;
        }
    }

    fn clear_kids(&mut self) {
        IN_CALLBACK.set(true);
        for kid in self.kids.drain(..) {
            unsafe {
                let _ = DestroyWindow(kid.hwnd);
            }
        }
        IN_CALLBACK.set(false);
        let null = HWND(std::ptr::null_mut());
        self.list_hwnd = null;
        self.status_hwnd = null;
        self.progress_hwnd = null;
        self.percent_hwnd = null;
        self.lastfm_hwnd = null;
        self.lastfm_percent = null;
        self.musicnn_hwnd = null;
        self.musicnn_percent = null;
        self.readout = [null; 5];
        self.profile_hwnd = null;
        self.name_hwnd = null;
        self.input_hwnd = null;
        self.output_hwnd = null;
        self.audio_hwnd = null;
        self.video_hwnd = null;
        self.genre_hwnd = null;
        self.genre_replace_hwnd = null;
        self.genre_keep_hwnd = null;
        self.genre_youtube_hwnd = null;
        self.lastfm_key_hwnd = null;
        self.lastfm_key_label = null;
    }

    fn place(&self) {
        let host_w = self.host_width().max(1);
        let view_h = self.view_h().max(1);
        for kid in &self.kids {
            let (x, y, w, h) = match kid.anchor {
                Anchor::Fixed => (kid.x, kid.y, kid.w, kid.h),
                Anchor::StretchX => (kid.x, kid.y, (host_w - kid.x - kid.right).max(40), kid.h),
                Anchor::Right => (host_w - kid.right - kid.w, kid.y, kid.w, kid.h),
                Anchor::Fill => {
                    let w = (host_w - kid.x - kid.right).max(40);
                    let h = (view_h - kid.y - kid.bottom).max(40);
                    (kid.x, kid.y, w, h)
                }
            };
            unsafe {
                let _ = MoveWindow(kid.hwnd, x, y, w, h, false);
            }
            if class_name(kid.hwnd) == "SysListView32" {
                unsafe {
                    // Fit the one column inside the client area (it is narrower
                // while the scroll bar shows) so no horizontal bar appears.
                let (client_w, _) = client_of(kid.hwnd);
                SendMessageW(kid.hwnd, 0x101E, WPARAM(0), LPARAM((client_w - 2).max(40) as isize));
                }
            }
        }
    }

    fn place_sheet(&self) {
        if self.sheet.0.is_null() {
            return;
        }
        let w = self.host_width().max(1);
        let h = self.content_height.max(self.view_h()).max(1);
        unsafe {
            let _ = MoveWindow(self.sheet, 0, -self.scroll, w, h, true);
        }
    }

    fn create_sheet(&self) -> HWND {
        let class_w = wide("OpusConvertHost");
        IN_CALLBACK.set(true);
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
                PCWSTR(class_w.as_ptr()),
                PCWSTR(wide("").as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x40000000 | 0x10000000),
                0,
                0,
                10,
                10,
                self.host,
                HMENU(std::ptr::null_mut()),
                self.instance,
                None,
            )
            .unwrap_or_default()
        };
        IN_CALLBACK.set(false);
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, WPARAM(font_bits(self.font)), LPARAM(0));
        }
        hwnd
    }

    fn pane(&self) -> HWND {
        if self.sheet.0.is_null() {
            self.host
        } else {
            self.sheet
        }
    }

    fn update_scroll(&self) {
        let view = self.view_h().max(1);
        let info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
            nMin: 0,
            nMax: (self.content_height - 1).max(0),
            nPage: view as u32,
            nPos: self.scroll,
            nTrackPos: 0,
        };
        unsafe {
            SetScrollInfo(self.host, SB_VERT, &info, false);
            let _ = ShowScrollBar(self.host, SB_VERT, false);
        }
    }

    fn on_scroll(&mut self, wparam: WPARAM) {
        let code = (wparam.0 & 0xFFFF) as u16;
        let view = self.view_h();
        let max_scroll = (self.content_height - view).max(0);
        let mut next = self.scroll;
        match code {
            0 => next -= self.s(18),
            1 => next += self.s(18),
            2 => next -= view,
            3 => next += view,
            5 | 4 => next = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32,
            6 => next = 0,
            7 => next = max_scroll,
            _ => {}
        }
        self.scroll = next.clamp(0, max_scroll);
        self.shift_kids();
    }

    fn scroll_by(&mut self, delta: i32) {
        let max_scroll = (self.content_height - self.view_h()).max(0);
        let next = (self.scroll + delta).clamp(0, max_scroll);
        if next == self.scroll {
            return;
        }
        self.scroll = next;
        self.shift_kids();
        self.redraw();
    }

    fn reveal_host_rect(&mut self, rect: RECT) {
        let view = self.view_h();
        let margin = self.s(4);
        let delta = if rect.bottom - rect.top >= view - margin * 2 {
            if rect.top < margin { rect.top - margin } else { 0 }
        } else if rect.top < margin {
            rect.top - margin
        } else if rect.bottom > view - margin {
            rect.bottom - (view - margin)
        } else {
            0
        };
        if delta == 0 {
            return;
        }
        let max_scroll = (self.content_height - view).max(0);
        let next = (self.scroll + delta).clamp(0, max_scroll);
        if next == self.scroll {
            return;
        }
        self.scroll = next;
        self.shift_kids();
    }

    fn shift_kids(&self) {
        self.place_sheet();
        self.update_scroll();
    }

    fn view_h(&self) -> i32 {
        let (_, h) = client_of(self.hwnd);
        (h - self.s(46) - self.s(46)).max(1)
    }

    fn host_width(&self) -> i32 {
        let (w, _) = client_of(self.host);
        if w > 0 { w } else { self.s(640) }
    }

    fn refresh_nav(&mut self) {
        let ready = self.page_ready();
        set_enabled(self.back, self.page > 0);
        set_enabled(self.next, ready);
        let caption = if self.page == 4 { "&Finish" } else { "Next >" };
        set_text(self.next, caption);
        let title = if self.busy != Busy::Idle {
            let left = self.rows.iter().filter(|row| matches!(row.kind, RowKind::Queued | RowKind::Active | RowKind::Waiting)).count();
            if left > 0 {
                format!("Opus Convert - {left} left")
            } else {
                "Opus Convert".into()
            }
        } else {
            "Opus Convert".into()
        };
        set_text(self.hwnd, &title);
        self.refresh_name_errors();
    }

    fn refresh_name_errors(&self) {
        if self.page != 1 {
            return;
        }
        let problem = paths::name_problem_line(&self.groups).unwrap_or_default();
        for index in 0..self.groups.len().min(GROUP_LIMIT) {
            if let Ok(label) = child(self.pane(), ID_NAME_ERROR + index as i32) {
                set_text(label, if index == 0 { &problem } else { "" });
            }
        }
    }

    fn page_ready(&self) -> bool {
        if self.busy != Busy::Idle && self.page >= 1 {
            return false;
        }
        match self.page {
            0 => can_advance(&self.current_options()),
            1 => self.scan_ok && self.names_ready(),
            2 | 3 => self.scan_ok,
            4 => self.convert_done,
            _ => false,
        }
    }

    fn names_ready(&self) -> bool {
        self.groups.iter().any(|group| !group.items.is_empty() || !group.junk.is_empty())
            && paths::name_problem_line(&self.groups).is_none()
    }

    fn on_command(&mut self, wparam: WPARAM, lparam: LPARAM) -> Action {
        let id = (wparam.0 & 0xFFFF) as i32;
        let code = ((wparam.0 >> 16) & 0xFFFF) as u16;
        if code == 7 || code == 8 {
            let hwnd = HWND(lparam.0 as *mut _);
            if class_name(hwnd) == "ComboBox" {
                if code == 7 {
                    unsafe { SetWindowRgn(hwnd, HRGN(std::ptr::null_mut()), BOOL(1)); }
                } else {
                    self.clip_closed_combo(hwnd);
                }
                // Repaint the 3.0 arrow button pressed (open) or raised (closed).
                unsafe { let _ = InvalidateRect(hwnd, None, false); }
            }
        }
        if id == ID_BACK && code == 0 {
            return self.go_back();
        }
        if id == ID_NEXT && code == 0 {
            return self.go_next();
        }
        if id == ID_SAVE && code == 0 {
            return self.save_profile();
        }
        if id == ID_BROWSE_IN && code == 0 {
            return Action::BrowseInput;
        }
        if id == ID_BROWSE_OUT && code == 0 {
            return Action::BrowseOutput;
        }
        if id == ID_PROFILE && code == CBN_SELCHANGE as u16 {
            self.change_profile();
            return Action::None;
        }
        if self.page == 0 && (code == 0 || code == EN_CHANGE as u16 || code == EN_KILLFOCUS as u16) {
            if matches!(id, ID_INPUT | ID_OUTPUT) && code == EN_KILLFOCUS as u16 {
                self.commit_folders();
                store::save(&self.memory);
            }
            if id == ID_LASTFM_KEY && code == EN_KILLFOCUS as u16 {
                self.persist_lastfm_key();
            }
            if matches!(
                id,
                ID_AUDIO
                    | ID_VIDEO
                    | ID_GENRE
                    | ID_GENRE_REPLACE
                    | ID_GENRE_KEEP
                    | ID_GENRE_YOUTUBE
                    | ID_INPUT
                    | ID_OUTPUT
            ) {
                self.options = self.current_options();
                if id == ID_GENRE {
                    self.sync_genre_radios();
                }
                self.refresh_nav();
            }
        }
        if code == 0 && (ID_RESET..ID_META_RESET).contains(&id) {
            let group = (id - ID_RESET) as usize;
            if self.page == 2 {
                self.reset_titles(group);
            } else {
                self.reset_names(group);
            }
            return Action::None;
        }
        if code == 0 && (ID_META_RESET..ID_NAME_ERROR).contains(&id) {
            self.reset_meta((id - ID_META_RESET) as usize);
            return Action::None;
        }
        if code == 0 && (ID_TITLE_SAME_ALL..ID_TITLE_FIELD).contains(&id) {
            self.use_file_names((id - ID_TITLE_SAME_ALL) as usize);
            return Action::None;
        }
        if code == 0 && (ID_TITLE_SAME..).contains(&id) {
            self.use_file_name((id - ID_TITLE_SAME) as usize);
            return Action::None;
        }
        if (self.page == 1 || self.page == 2) && id >= ID_REMOVE && id < ID_ARTIST {
            let group = group_of(id);
            if code == 0 && id >= ID_APPLY && id < ID_LIST {
                if self.page == 2 {
                    self.apply_titles(group);
                } else {
                    self.apply_group(group);
                }
            } else if code == CBN_SELCHANGE as u16 || code == EN_CHANGE as u16 || code == 0 {
                self.sync_apply(group);
            }
        }
        Action::None
    }

    fn on_notify(&mut self, lparam: LPARAM) -> Action {
        unsafe {
            let header = &*(lparam.0 as *const NMHDR);
            if header.code == NM_CLICK || header.code == NM_DBLCLK {
                let id = child_id(header.hwndFrom);
                if (ID_LIST..ID_ALBUM).contains(&id) {
                    let activate = &*(lparam.0 as *const NMITEMACTIVATE);
                    if activate.iItem >= 0 {
                        SendMessageW(header.hwndFrom, LVM_EDITLABELW, WPARAM(activate.iItem as usize), LPARAM(0));
                    }
                }
                return Action::None;
            }
            if header.code == LVN_ENDLABELEDITW {
                let info = &*(lparam.0 as *const NMLVDISPINFOW);
                if info.item.pszText.0.is_null() {
                    return Action::Result(0);
                }
                let title = pw_string(info.item.pszText);
                let id = child_id(header.hwndFrom);
                if (ID_LIST..ID_ALBUM).contains(&id) {
                    let group = (id - ID_LIST) as usize;
                    if let Some(item) = self.groups.get_mut(group).and_then(|group| group.items.get_mut(info.item.iItem as usize)) {
                        item.stem = title;
                    }
                }
                self.refresh_nav();
                let _ = InvalidateRect(header.hwndFrom, None, true);
                return Action::Result(1);
            }
        }
        Action::None
    }

    fn go_next(&mut self) -> Action {
        self.harvest();
        match self.page {
            0 => {
                self.options = self.current_options();
                if !can_advance(&self.options) {
                    let text = model::problems(&self.options).join("\n\n");
                    if text.is_empty() {
                        return Action::None;
                    }
                    return Action::Popup(text);
                }
                match tools::load_bundle() {
                    Ok(bundle) => {
                        self.ffmpeg = bundle.ffmpeg;
                        self.ffprobe = bundle.ffprobe;
                    }
                    Err(err) => return Action::Popup(err),
                }
                self.memory.last_profile = self.active_profile.clone();
                self.commit_folders();
                store::save(&self.memory);
                let same = self.scan_ok && self.scan_stamp == self.options;
                self.page = 1;
                self.rebuild();
                if same {
                    Action::None
                } else {
                    self.scan_ok = false;
                    self.rebuild();
                    Action::StartScan
                }
            }
            1 => {
                if !self.names_ready() {
                    let problem = paths::name_problem_line(&self.groups).unwrap_or_else(|| "Type a file name for every file.".into());
                    return Action::Popup(problem);
                }
                self.page = 2;
                self.rebuild();
                Action::None
            }
            2 => {
                self.page = 3;
                self.rebuild();
                Action::None
            }
            3 => {
                if self.options.lookup_genre {
                    if let Err(err) = crate::tools::find_python() {
                        return Action::Popup(err);
                    }
                    if let Err(err) = crate::genre::script_next_to_exe() {
                        return Action::Popup(err);
                    }
                }
                self.page = 4;
                self.rebuild();
                Action::StartConvert
            }
            _ => Action::Close,
        }
    }

    fn go_back(&mut self) -> Action {
        if self.page == 0 {
            return Action::None;
        }
        self.harvest();
        if self.busy != Busy::Idle {
            return Action::Cancel;
        }
        if self.page == 4 {
            self.convert_done = false;
        }
        self.page -= 1;
        self.rebuild();
        Action::None
    }

    fn save_profile(&mut self) -> Action {
        let name = get_text(self.name_hwnd).trim().to_string();
        if name.is_empty() {
            return Action::Popup("Type a profile name.".into());
        }
        self.options = self.current_options();
        if let Some(profile) = self.memory.profiles.iter_mut().find(|profile| profile.name == name) {
            profile.options = self.options.clone();
        } else {
            self.memory.profiles.push(SavedProfile { name: name.clone(), options: self.options.clone() });
        }
        if self.memory.override_profile == name {
            self.memory.override_profile.clear();
            self.memory.override_input.clear();
            self.memory.override_output.clear();
        }
        self.active_profile = name.clone();
        self.memory.last_profile = name;
        self.loading = true;
        combo_clear(self.profile_hwnd);
        for profile in &self.memory.profiles {
            combo_add(self.profile_hwnd, &profile.name);
        }
        let sel = self.memory.profiles.iter().position(|profile| profile.name == self.active_profile).unwrap_or(0);
        set_combo(self.profile_hwnd, sel);
        self.loading = false;
        store::save(&self.memory);
        Action::None
    }

    fn change_profile(&mut self) {
        if self.loading {
            return;
        }
        self.commit_folders();
        let sel = combo_sel(self.profile_hwnd);
        let Some(name) = self.memory.profiles.get(sel).map(|profile| profile.name.clone()) else {
            return;
        };
        let mut options = self.memory.profiles[sel].options.clone();
        if self.memory.override_profile == name {
            if !self.memory.override_input.is_empty() {
                options.input = self.memory.override_input.clone();
            }
            if !self.memory.override_output.is_empty() {
                options.output = self.memory.override_output.clone();
            }
        }
        self.options = options;
        self.active_profile = name;
        self.memory.last_profile = self.active_profile.clone();
        store::save(&self.memory);
        self.loading = true;
        self.rebuild();
        self.loading = false;
    }

    fn commit_folders(&mut self) {
        if self.input_hwnd.0.is_null() {
            return;
        }
        let input = get_text(self.input_hwnd);
        let output = get_text(self.output_hwnd);
        let saved = self.memory.profiles.iter().find(|profile| profile.name == self.active_profile).map(|profile| profile.options.clone());
        let Some(saved) = saved else {
            return;
        };
        if input != saved.input || output != saved.output {
            self.memory.override_profile = self.active_profile.clone();
            self.memory.override_input = input;
            self.memory.override_output = output;
        } else if self.memory.override_profile == self.active_profile {
            self.memory.override_profile.clear();
            self.memory.override_input.clear();
            self.memory.override_output.clear();
        }
    }

    fn current_options(&self) -> Options {
        if self.profile_hwnd.0.is_null() {
            return self.options.clone();
        }
        Options {
            input: get_text(self.input_hwnd).trim().to_string(),
            output: get_text(self.output_hwnd).trim().to_string(),
            convert_audio: checked(self.audio_hwnd),
            convert_video: checked(self.video_hwnd),
            lookup_genre: checked(self.genre_hwnd),
            genre_policy: self.selected_genre_policy(),
        }
    }

    fn harvest_soft(&mut self) {
        if self.page == 0 && !self.profile_hwnd.0.is_null() {
            self.options = self.current_options();
            self.persist_lastfm_key();
        }
    }

    fn harvest(&mut self) {
        self.harvest_soft();
        if self.page == 1 && self.scan_ok {
            self.read_names();
        }
        if self.page == 2 && self.scan_ok {
            self.read_titles();
        }
        if self.page == 3 {
            self.read_meta();
        }
        if self.page == 0 {
            self.commit_folders();
        }
    }

    fn read_names(&mut self) {
        let pane = self.pane();
        for (group_index, group) in self.groups.iter_mut().enumerate() {
            if let Ok(list) = child(pane, ID_LIST + group_index as i32) {
                for (index, item) in group.items.iter_mut().enumerate() {
                    item.stem = list_text(list, index as i32);
                }
            }
            if let Ok(folder) = child(pane, ID_FOLDER + group_index as i32) {
                let name = get_text(folder);
                if !name.is_empty() {
                    group.name = name;
                }
            }
        }
        self.capture_tools();
    }

    fn capture_tools(&mut self) {
        self.fit_tools();
        let titles = self.page == 2;
        for index in 0..self.groups.len().min(GROUP_LIMIT) {
            let tools = self.tools_of(index);
            if titles {
                self.title_tools[index] = tools;
            } else {
                self.name_tools[index] = tools;
            }
        }
    }

    fn tools_of(&self, group: usize) -> TextTools {
        TextTools {
            remove: text_of(self.pane(), ID_REMOVE + group as i32),
            insert: text_of(self.pane(), ID_INSERT + group as i32),
            at: match combo_sel_id(self.pane(), ID_MODE + group as i32) {
                1 => InsertAt::End,
                2 => InsertAt::Before,
                3 => InsertAt::After,
                _ => InsertAt::Start,
            },
            find: text_of(self.pane(), ID_FIND + group as i32),
            strip: checked_id(self.pane(), ID_STRIP + group as i32),
        }
    }

    fn sync_apply(&self, group: usize) {
        let tools = self.tools_of(group);
        let find_on = matches!(tools.at, InsertAt::Before | InsertAt::After);
        if let Ok(find) = child(self.pane(), ID_FIND + group as i32) {
            set_enabled(find, find_on);
        }
        if let Ok(apply) = child(self.pane(), ID_APPLY + group as i32) {
            set_enabled(apply, names::tools_ready(&tools));
        }
    }

    fn apply_group(&mut self, group: usize) {
        let tools = self.tools_of(group);
        if !names::tools_ready(&tools) {
            return;
        }
        if let Ok(folder) = child(self.pane(), ID_FOLDER + group as i32) {
            let next = names::apply_tools(&get_text(folder), &tools);
            set_text(folder, &next);
            if let Some(item) = self.groups.get_mut(group) {
                item.name = next;
            }
        }
        let Ok(list) = child(self.pane(), ID_LIST + group as i32) else {
            if group < self.name_tools.len() {
                self.name_tools[group] = tools;
            }
            self.refresh_nav();
            return;
        };
        let count = unsafe { SendMessageW(list, LVM_GETITEMCOUNT, WPARAM(0), LPARAM(0)).0.max(0) as usize };
        for index in 0..count {
            let next = names::apply_tools(&list_text(list, index as i32), &tools);
            set_list_text(list, index as i32, &next);
            if let Some(item) = self.groups.get_mut(group).and_then(|group| group.items.get_mut(index)) {
                item.stem = next;
            }
        }
        if group < self.name_tools.len() {
            self.name_tools[group] = tools;
        }
        self.refresh_nav();
    }

    fn apply_titles(&mut self, group: usize) {
        let tools = self.tools_of(group);
        if !names::tools_ready(&tools) {
            return;
        }
        let Some(count) = self.groups.get(group).map(|group| group.items.len()) else {
            return;
        };
        let start = flat_start(&self.groups, group);
        for index in 0..count {
            let id = ID_TITLE_FIELD + (start + index) as i32;
            let Ok(hwnd) = child(self.pane(), id) else {
                continue;
            };
            let next = names::apply_tools(&get_text(hwnd), &tools);
            self.groups[group].items[index].meta_title = next.clone();
            set_text(hwnd, &next);
        }
        if group < self.title_tools.len() {
            self.title_tools[group] = tools;
        }
    }

    fn read_titles(&mut self) {
        let pane = self.pane();
        let mut flat = 0usize;
        for group in self.groups.iter_mut().take(GROUP_LIMIT) {
            for item in &mut group.items {
                if ID_TITLE_SAME + flat as i32 > 65000 {
                    break;
                }
                if let Ok(hwnd) = child(pane, ID_TITLE_FIELD + flat as i32) {
                    if !hwnd.0.is_null() {
                        item.meta_title = get_text(hwnd);
                    }
                }
                flat += 1;
            }
        }
        self.capture_tools();
    }

    fn use_file_name(&mut self, flat: usize) {
        let Some((group_index, item_index)) = locate_flat(&self.groups, flat) else {
            return;
        };
        let name = self.groups[group_index].items[item_index].stem.clone();
        self.groups[group_index].items[item_index].meta_title = name.clone();
        if let Ok(hwnd) = child(self.pane(), ID_TITLE_FIELD + flat as i32) {
            set_text(hwnd, &name);
        }
    }

    fn use_file_names(&mut self, group_index: usize) {
        let Some(group) = self.groups.get(group_index) else {
            return;
        };
        let start = flat_start(&self.groups, group_index);
        let names: Vec<(usize, String)> = group.items.iter().enumerate().map(|(index, item)| (start + index, item.stem.clone())).collect();
        for (flat, name) in names {
            let item_index = flat - start;
            self.groups[group_index].items[item_index].meta_title = name.clone();
            if let Ok(hwnd) = child(self.pane(), ID_TITLE_FIELD + flat as i32) {
                set_text(hwnd, &name);
            }
        }
    }

    fn reset_names(&mut self, group: usize) {
        let Some(source) = self.groups.get_mut(group) else {
            return;
        };
        source.name = source.original_name.clone();
        for item in &mut source.items {
            item.stem = item.original_stem.clone();
        }
        if let Ok(folder) = child(self.pane(), ID_FOLDER + group as i32) {
            set_text(folder, &self.groups[group].name);
        }
        if let Ok(list) = child(self.pane(), ID_LIST + group as i32) {
            for (index, item) in self.groups[group].items.iter().enumerate() {
                set_list_text(list, index as i32, &item.stem);
            }
        }
        self.refresh_nav();
    }

    fn reset_titles(&mut self, group: usize) {
        let Some(source) = self.groups.get_mut(group) else {
            return;
        };
        for item in &mut source.items {
            item.meta_title = item.original_title.clone();
        }
        let start = flat_start(&self.groups, group);
        let titles: Vec<String> = self.groups[group].items.iter().map(|item| item.meta_title.clone()).collect();
        for (index, title) in titles.iter().enumerate() {
            if let Ok(hwnd) = child(self.pane(), ID_TITLE_FIELD + (start + index) as i32) {
                set_text(hwnd, title);
            }
        }
    }

    fn reset_meta(&mut self, group: usize) {
        let Some(source) = self.groups.get_mut(group) else {
            return;
        };
        source.album = source.original_album.clone();
        for item in &mut source.items {
            item.artists = item.original_artists.clone();
        }
        let album = self.groups[group].album.clone();
        if let Ok(hwnd) = child(self.pane(), ID_ALBUM + group as i32) {
            set_text(hwnd, &album);
        }
        let mut flat = self.groups.iter().take(group).map(|group| group.items.len()).sum::<usize>();
        let rows: Vec<String> = self.groups[group].items.iter().map(|item| item.artists.join("\r\n")).collect();
        for artists in rows {
            if let Ok(hwnd) = child(self.pane(), ID_ARTIST + flat as i32) {
                set_text(hwnd, &artists);
            }
            flat += 1;
        }
    }

    fn read_meta(&mut self) {
        let pane = self.pane();
        let mut flat = 0usize;
        for (group_index, group) in self.groups.iter_mut().enumerate() {
            group.album = text_of(pane, ID_ALBUM + group_index as i32).trim().to_string();
            for item in &mut group.items {
                let text = text_of(pane, ID_ARTIST + flat as i32);
                item.artists = text.lines().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect();
                flat += 1;
            }
        }
    }

    fn fit_tools(&mut self) {
        if self.name_tools.len() < self.groups.len() {
            self.name_tools.resize(self.groups.len(), TextTools::default());
        }
        if self.title_tools.len() < self.groups.len() {
            self.title_tools.resize(self.groups.len(), TextTools::default());
        }
    }

    fn spawn(&mut self, kind: Busy) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let shared = Shared::new();
        let shared_thread = shared.clone();
        let ffmpeg = self.ffmpeg.clone();
        let ffprobe = self.ffprobe.clone();
        let opts = self.options.clone();
        let groups = self.groups.clone();
        self.shared = Some(shared);
        self.busy = kind;
        self.rows.clear();
        self.reset_speed();
        self.started = Some(Instant::now());
        self.worker = Some(std::thread::spawn(move || match kind {
            Busy::Scan => scan::scan(shared_thread, ffprobe, opts),
            Busy::Convert => run::convert(shared_thread, ffmpeg, ffprobe, opts, groups),
            Busy::Idle => {}
        }));
        self.refresh_nav();
    }

    fn reset_speed(&mut self) {
        self.samples.clear();
        self.item_ticks.clear();
        self.sample_start = Instant::now();
        self.last_done = 0;
    }

    fn on_timer(&mut self) -> Action {
        self.poll_work()
    }

    fn poll_work(&mut self) -> Action {
        let Some(shared) = self.shared.clone() else {
            return Action::None;
        };
        let live = match shared.live.lock() {
            Ok(live) => live.clone(),
            Err(_) => return Action::None,
        };
        if self.busy == Busy::Idle {
            return Action::None;
        }
        self.rows = live.rows.clone();
        self.track_speed(&live);
        self.paint_readouts(&live);
        if !self.list_hwnd.0.is_null() {
            clamp_list(self, self.rows.len());
            unsafe { let _ = InvalidateRect(self.list_hwnd, None, false); }
        }
        self.refresh_nav();
        if !live.done {
            return Action::None;
        }
        let taken = shared.groups.lock().ok().and_then(|mut slot| slot.take());
        if let Some(groups) = taken {
            self.groups = groups;
            self.fit_tools();
        }
        let kind = self.busy;
        self.busy = Busy::Idle;
        let notice = live.error.clone();
        match kind {
            Busy::Scan => {
                self.scan_ok = notice.is_empty();
                self.scan_stamp = self.options.clone();
                self.rebuild();
                if !notice.is_empty() {
                    return Action::Popup(notice);
                }
            }
            Busy::Convert => {
                self.convert_done = true;
                self.refresh_nav();
                unsafe { let _ = InvalidateRect(self.list_hwnd, None, false); }
            }
            Busy::Idle => {}
        }
        Action::None
    }

    fn track_speed(&mut self, live: &Live) {
        let now = Instant::now();
        let bytes = live.banked_bytes.saturating_add(live.current_bytes);
        self.samples.push_back((now, bytes));
        while self.samples.len() > 2 && now.saturating_duration_since(self.samples[0].0).as_millis() > 2000 {
            self.samples.pop_front();
        }
        let done = live.ok.saturating_add(live.bad);
        if done > self.last_done {
            for _ in 0..(done - self.last_done) {
                self.item_ticks.push_back(now);
            }
            self.last_done = done;
        }
        while self.item_ticks.front().is_some_and(|tick| now.saturating_duration_since(*tick).as_millis() > 2000) {
            self.item_ticks.pop_front();
        }
    }

    fn paint_readouts(&self, live: &Live) {
        if self.status_hwnd.0.is_null() {
            return;
        }
        set_text(self.status_hwnd, &live.status);
        if self.progress_hwnd.0.is_null() {
            return;
        }
        let ratio = work_ratio(live);
        unsafe { SendMessageW(self.progress_hwnd, PBM_SETPOS, WPARAM(readout::percent(ratio) as usize * 10), LPARAM(0)); }
        if !self.percent_hwnd.0.is_null() {
            set_text(self.percent_hwnd, &format!("{}%", readout::percent(ratio)));
        }
        if !self.lastfm_hwnd.0.is_null() {
            let lastfm = if live.lastfm_total == 0 {
                0.0
            } else {
                live.lastfm_done as f64 / live.lastfm_total as f64
            };
            unsafe {
                SendMessageW(self.lastfm_hwnd, PBM_SETPOS, WPARAM(readout::percent(lastfm) as usize * 10), LPARAM(0));
            }
            if !self.lastfm_percent.0.is_null() {
                set_text(self.lastfm_percent, &format!("{}%", readout::percent(lastfm)));
            }
        }
        if !self.musicnn_hwnd.0.is_null() {
            let musicnn = if live.musicnn_total == 0 {
                0.0
            } else {
                live.musicnn_done as f64 / live.musicnn_total as f64
            };
            unsafe {
                SendMessageW(self.musicnn_hwnd, PBM_SETPOS, WPARAM(readout::percent(musicnn) as usize * 10), LPARAM(0));
            }
            if !self.musicnn_percent.0.is_null() {
                set_text(self.musicnn_percent, &format!("{}%", readout::percent(musicnn)));
            }
        }
        if self.readout[0].0.is_null() {
            return;
        }
        let (speed, items) = speed_of(self);
        let left = live.rows.iter().filter(|row| matches!(row.kind, RowKind::Queued | RowKind::Active | RowKind::Waiting)).count();
        let eta = if items > 0.0 && left > 0 {
            readout::format_eta(left as f64 / items)
        } else if live.done || left == 0 {
            "--".to_string()
        } else {
            "Calculating...".to_string()
        };
        let elapsed = self.started.map(|start| readout::format_elapsed(start.elapsed().as_secs())).unwrap_or_else(|| "0:00".into());
        let values = [
            readout::format_rate(speed),
            readout::format_items(items),
            left.to_string(),
            eta.to_string(),
            elapsed,
        ];
        for (index, value) in values.iter().enumerate() {
            set_text(self.readout[index], value);
        }
    }

    fn after_cancel(&mut self) {
        let kind = self.busy;
        self.busy = Busy::Idle;
        if kind == Busy::Scan {
            self.scan_ok = false;
        }
        if kind == Busy::Convert {
            self.convert_done = false;
            let taken = self.shared.as_ref().and_then(|shared| shared.groups.lock().ok().and_then(|mut slot| slot.take()));
            if let Some(groups) = taken {
                self.groups = groups;
                self.fit_tools();
            }
        }
        if self.page > 0 {
            self.page -= 1;
        }
        self.rebuild();
    }

    fn shutdown(&mut self) {
        unsafe {
            let _ = KillTimer(self.hwnd, 1);
            if !self.caption_font.0.is_null() {
                let _ = DeleteObject(self.caption_font);
                self.caption_font = HFONT(std::ptr::null_mut());
            }
        }
        if let Some(shared) = &self.shared {
            run::kill_child(shared);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.harvest();
        self.memory.last_profile = self.active_profile.clone();
        store::save(&self.memory);
        unsafe { PostQuitMessage(0); }
    }

    fn tip_nav(&mut self) {
        add_tip(self, self.back, "Previous page");
        add_tip(self, self.next, "Next page");
    }

    fn retip(&mut self) {
        if !self.tip.0.is_null() {
            unsafe { let _ = DestroyWindow(self.tip); }
        }
        self.tip = tooltip(self.hwnd, self.instance);
        self.tip_nav();
        let pairs: Vec<(HWND, &str)> = match self.page {
            0 => vec![
                (self.profile_hwnd, "Saved options"),
                (self.name_hwnd, "Name for this profile"),
                (self.input_hwnd, "Folder to read"),
                (self.output_hwnd, "Folder for finished files"),
                (self.audio_hwnd, "Convert audio; copy video"),
                (self.video_hwnd, "Convert video; copy audio"),
                (self.genre_hwnd, "Last.fm first, then musicnn"),
                (self.genre_replace_hwnd, "Overwrite the genre on every file"),
                (self.genre_keep_hwnd, "Skip audio that already has a genre; movies still look up"),
                (
                    self.genre_youtube_hwnd,
                    "Skip audio with a real genre; strip YouTube categories like Music or People & Blogs and look those up",
                ),
                (self.lastfm_key_hwnd, "Last.fm API key; kept after restart"),
            ],
            _ => Vec::new(),
        };
        for (hwnd, text) in pairs {
            add_tip(self, hwnd, text);
        }
        if self.page == 0 {
            if let Ok(save) = child(self.pane(), ID_SAVE) {
                add_tip(self, save, "Save these options");
            }
            if let Ok(browse) = child(self.pane(), ID_BROWSE_IN) {
                add_tip(self, browse, "Choose the input folder");
            }
            if let Ok(browse) = child(self.pane(), ID_BROWSE_OUT) {
                add_tip(self, browse, "Choose the output folder");
            }
        }
        if self.page == 1 {
            for index in 0..self.groups.len().min(GROUP_LIMIT) {
                if let Ok(reset) = child(self.pane(), ID_RESET + index as i32) {
                    add_tip(self, reset, "Put the file names back");
                }
            }
        }
        if self.page == 2 {
            for index in 0..self.groups.len().min(GROUP_LIMIT) {
                if let Ok(reset) = child(self.pane(), ID_RESET + index as i32) {
                    add_tip(self, reset, "Put the titles back");
                }
                if let Ok(all) = child(self.pane(), ID_TITLE_SAME_ALL + index as i32) {
                    add_tip(self, all, "Use each file name as the title");
                }
            }
        }
        if self.page == 3 {
            for index in 0..self.groups.len().min(GROUP_LIMIT) {
                if let Ok(reset) = child(self.pane(), ID_META_RESET + index as i32) {
                    add_tip(self, reset, "Put the album and artists back");
                }
            }
        }
    }
}

fn speed_of(app: &App) -> (f64, f64) {
    let speed = if app.samples.len() >= 2 {
        let older = app.samples.front().unwrap();
        let newer = app.samples.back().unwrap();
        let seconds = newer.0.saturating_duration_since(older.0).as_secs_f64().max(0.2);
        (newer.1.saturating_sub(older.1) as f64) / seconds
    } else {
        0.0
    };
    let run = app.sample_start.elapsed().as_secs_f64().max(0.25);
    let divisor = if run < 2.0 { run } else { 2.0 };
    let items = app.item_ticks.len() as f64 / divisor;
    (speed, items)
}

fn work_ratio(live: &Live) -> f64 {
    if live.total == 0 {
        return if live.done { 1.0 } else { 0.0 };
    }
    let active = live.rows.iter().find(|row| row.kind == RowKind::Active).map(|row| row.fraction).unwrap_or(0.0);
    ((live.ok.saturating_add(live.bad) as f64 + active) / live.total as f64).clamp(0.0, 1.0)
}

fn clamp_list(app: &mut App, count: usize) {
    let visible = visible_rows(app.list_hwnd, app.dpi);
    let max_scroll = (count as i32 - visible).max(0);
    if app.list_scroll > max_scroll {
        app.list_scroll = max_scroll;
    }
    let info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: (count as i32 - 1).max(0),
        nPage: visible.max(1) as u32,
        nPos: app.list_scroll,
        nTrackPos: 0,
    };
    unsafe {
        SetScrollInfo(app.list_hwnd, SB_VERT, &info, false);
        let _ = ShowScrollBar(app.list_hwnd, SB_VERT, false);
    }
}

fn visible_rows(hwnd: HWND, dpi: i32) -> i32 {
    let (_, h) = client_of(hwnd);
    let header = scale(22, dpi);
    let row = scale(24, dpi).max(1);
    ((h - header) / row).max(1)
}

fn list_scrollbar_click(hwnd: HWND, lparam: LPARAM) -> bool {
    let p = POINT {
        x: (lparam.0 as i16) as i32,
        y: ((lparam.0 >> 16) as i16) as i32,
    };
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else { return false; };
        let Some(app) = borrow.as_mut() else { return false; };
        let (w, h) = client_of(hwnd);
        let visible = visible_rows(hwnd, app.dpi);
        let count = app.rows.len() as i32;
        let Some(sb) = crate::aero::scrollbar(
            work_bar_area(w, h, app.dpi),
            app.list_scroll,
            visible,
            count.max(1),
            app.dpi,
        ) else {
            return false;
        };
        let max_scroll = (count - visible).max(0);
        match crate::aero::scrollbar_hit(&sb, p) {
            crate::aero::SbHit::Up => app.list_scroll = (app.list_scroll - 1).clamp(0, max_scroll),
            crate::aero::SbHit::Down => app.list_scroll = (app.list_scroll + 1).clamp(0, max_scroll),
            crate::aero::SbHit::PageUp => app.list_scroll = (app.list_scroll - visible).clamp(0, max_scroll),
            crate::aero::SbHit::PageDown => app.list_scroll = (app.list_scroll + visible).clamp(0, max_scroll),
            crate::aero::SbHit::Thumb => {
                WORK_DRAG.set(Some((p.y, app.list_scroll)));
                unsafe { SetCapture(hwnd); }
                return true;
            }
            crate::aero::SbHit::None => return false,
        }
        unsafe { let _ = InvalidateRect(hwnd, None, false); }
        true
    })
}

/// Work list scroll bar body. Its top, right and bottom lines fall on the
/// list's black frame, just outside the client area.
fn work_bar_area(w: i32, h: i32, dpi: i32) -> RECT {
    let u = crate::aero::sb_unit(dpi);
    RECT { left: w - crate::aero::scroll_width(dpi), top: -u, right: w + u, bottom: h + u }
}

/// Dragging the work list's scroll box.
fn drag_work(hwnd: HWND, lparam: LPARAM) -> bool {
    let Some((origin, start)) = WORK_DRAG.get() else { return false; };
    let y = ((lparam.0 >> 16) as i16) as i32;
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else { return; };
        let Some(app) = borrow.as_mut() else { return; };
        let (w, h) = client_of(hwnd);
        let visible = visible_rows(hwnd, app.dpi);
        let count = app.rows.len() as i32;
        let Some(sb) = crate::aero::scrollbar(work_bar_area(w, h, app.dpi), app.list_scroll, visible, count.max(1), app.dpi) else {
            return;
        };
        let next = crate::aero::drag_pos(&sb, start, y - origin).clamp(0, (count - visible).max(0));
        if next != app.list_scroll {
            app.list_scroll = next;
            clamp_list(app, app.rows.len());
            unsafe { let _ = InvalidateRect(hwnd, None, false); }
        }
    });
    true
}

fn scroll_work(hwnd: HWND, wparam: WPARAM) {
    let code = (wparam.0 & 0xFFFF) as u16;
    let pos = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else { return; };
        let Some(app) = borrow.as_mut() else { return; };
        let visible = visible_rows(hwnd, app.dpi);
        let max_scroll = (app.rows.len() as i32 - visible).max(0);
        match code {
            0 => app.list_scroll -= 1,
            1 => app.list_scroll += 1,
            2 => app.list_scroll -= visible,
            3 => app.list_scroll += visible,
            4 | 5 => app.list_scroll = pos,
            6 => app.list_scroll = 0,
            7 => app.list_scroll = max_scroll,
            _ => {}
        }
        app.list_scroll = app.list_scroll.clamp(0, max_scroll);
        clamp_list(app, app.rows.len());
        unsafe { let _ = InvalidateRect(hwnd, None, false); }
    });
}

fn nudge_work(hwnd: HWND, delta: i32) {
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else { return; };
        let Some(app) = borrow.as_mut() else { return; };
        let visible = visible_rows(hwnd, app.dpi);
        let max_scroll = (app.rows.len() as i32 - visible).max(0);
        app.list_scroll = (app.list_scroll + delta).clamp(0, max_scroll);
        clamp_list(app, app.rows.len());
        unsafe { let _ = InvalidateRect(hwnd, None, false); }
    });
}

fn paint_work(hwnd: HWND) {
    unsafe {
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let (width, height) = client_of(hwnd);
        if width <= 0 || height <= 0 {
            let _ = EndPaint(hwnd, &ps);
            return;
        }
        let mem = CreateCompatibleDC(hdc);
        let bitmap = CreateCompatibleBitmap(hdc, width, height);
        let old_bmp = SelectObject(mem, bitmap);
        let white = white_brush();
        let face = FACE.get();
        let _ = FillRect(mem, &RECT { left: 0, top: 0, right: width, bottom: height }, white);
        APP.with(|slot| {
            let Ok(borrow) = slot.try_borrow() else { return; };
            let Some(app) = borrow.as_ref() else { return; };
            let font = SelectObject(mem, app.font);
            let header_h = app.s(22);
            let row_h = app.s(24);
            let count = app.rows.len() as i32;
            let visible = ((height - header_h) / row_h.max(1)).max(1);
            let sb_w = if count > visible { crate::aero::scroll_width(app.dpi) } else { 0 };
            let inner_w = (width - sb_w).max(1);
            let header = RECT { left: 0, top: 0, right: inner_w, bottom: header_h };
            let _ = FillRect(mem, &header, face);
            let (name_w, size_w, bar_w) = columns(inner_w);
            let _ = SetBkMode(mem, TRANSPARENT);
            let _ = SetTextColor(mem, rgb(32, 32, 32));
            text_in(mem, "File", RECT { left: 6, top: 0, right: name_w - 4, bottom: header_h }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
            text_in(mem, "Size", RECT { left: name_w, top: 0, right: name_w + size_w - 6, bottom: header_h }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            text_in(mem, "Progress", RECT { left: name_w + size_w + 6, top: 0, right: width - 4, bottom: header_h }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
            if app.rows.is_empty() {
                text_in(mem, "Finished files leave this list.", RECT { left: 8, top: header_h + 8, right: inner_w - 8, bottom: height - 8 }, DT_LEFT | DT_NOPREFIX);
            }
            let mut y = header_h;
            let mut index = app.list_scroll.max(0) as usize;
            while y < height && index < app.rows.len() {
                paint_row(mem, &app.rows[index], RECT { left: 0, top: y, right: inner_w, bottom: y + row_h }, name_w, size_w, bar_w);
                y += row_h;
                index += 1;
            }
            if let Some(sb) = crate::aero::scrollbar(
                work_bar_area(width, height, app.dpi),
                app.list_scroll,
                visible,
                count.max(1),
                app.dpi,
            ) {
                crate::aero::paint_scrollbar(mem, &sb);
            }
            SelectObject(mem, font);
            let _ = bar_w;
        });
        let _ = BitBlt(hdc, 0, 0, width, height, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old_bmp);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem);
        let _ = EndPaint(hwnd, &ps);
    }
}

fn paint_row(hdc: HDC, row: &WorkRow, rect: RECT, name_w: i32, size_w: i32, bar_w: i32) {
    let failed = row.kind == RowKind::Pinned || row.kind == RowKind::Failed;
    let back = if failed { rgb(255, 244, 244) } else { rgb(255, 255, 255) };
    let brush = unsafe { CreateSolidBrush(back) };
    unsafe {
        let _ = FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush);
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, if failed { rgb(164, 0, 0) } else { rgb(32, 32, 32) });
    }
    text_in(hdc, &row.name, RECT { left: rect.left + 6, top: rect.top, right: rect.left + name_w - 4, bottom: rect.bottom }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
    let size = if row.size > 0 { readout::format_bytes(row.size) } else { String::new() };
    text_in(hdc, &size, RECT { left: rect.left + name_w, top: rect.top, right: rect.left + name_w + size_w - 6, bottom: rect.bottom }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    if !row.note.is_empty() && row.kind != RowKind::Active {
        text_in(hdc, &row.note, RECT { left: rect.left + name_w + size_w + 6, top: rect.top, right: rect.right - 6, bottom: rect.bottom }, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
        return;
    }
    let bar_h = ((rect.bottom - rect.top) - 8).clamp(10, 16);
    let top = rect.top + (rect.bottom - rect.top - bar_h) / 2;
    let bar = RECT {
        left: rect.left + name_w + size_w + 6,
        top,
        right: rect.left + name_w + size_w + bar_w - 8,
        bottom: top + bar_h,
    };
    crate::aero::paint(hdc, bar, row.fraction, failed, row.kind == RowKind::Active);
}

fn columns(width: i32) -> (i32, i32, i32) {
    let mut bar = (width / 4).clamp(100, 240);
    let mut size = (width / 6).clamp(96, 140);
    let mut name = width - bar - size;
    if name < 80 {
        bar = ((width - 80 - 96) / 2).max(80);
        size = 96;
        name = (width - bar - size).max(60);
    }
    (name, size, bar)
}

fn text_in(hdc: HDC, text: &str, mut rect: RECT, format: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT) {
    let mut wide = wide(text);
    unsafe {
        let end = wide.len().saturating_sub(1);
        let _ = DrawTextW(hdc, &mut wide[..end], &mut rect, format);
    }
}

fn fill_titles(list: HWND, titles: &[String], width: i32) {
    let mut title = wide("File name");
    let column = LVCOLUMNW {
        mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
        fmt: LVCFMT_LEFT,
        cx: width,
        pszText: PWSTR(title.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        SendMessageW(list, LVM_SETEXTENDEDLISTVIEWSTYLE, WPARAM(0), LPARAM(LVS_EX_FULLROWSELECT as isize));
        SendMessageW(list, LVM_INSERTCOLUMNW, WPARAM(0), LPARAM(&column as *const _ as isize));
    }
    for (index, title) in titles.iter().enumerate() {
        let mut text = wide(title);
        let row = LVITEMW {
            mask: LVIF_TEXT,
            iItem: index as i32,
            pszText: PWSTR(text.as_mut_ptr()),
            ..Default::default()
        };
        unsafe { SendMessageW(list, LVM_INSERTITEMW, WPARAM(0), LPARAM(&row as *const _ as isize)); }
    }
}

fn list_text(list: HWND, index: i32) -> String {
    let mut buf = [0u16; 1024];
    let item = LVITEMW {
        pszText: PWSTR(buf.as_mut_ptr()),
        cchTextMax: buf.len() as i32,
        ..Default::default()
    };
    unsafe { SendMessageW(list, LVM_GETITEMTEXTW, WPARAM(index as usize), LPARAM(&item as *const _ as isize)); }
    let end = buf.iter().position(|ch| *ch == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn set_list_text(list: HWND, index: i32, text: &str) {
    let mut wide = wide(text);
    let item = LVITEMW {
        pszText: PWSTR(wide.as_mut_ptr()),
        ..Default::default()
    };
    unsafe { SendMessageW(list, LVM_SETITEMTEXTW, WPARAM(index as usize), LPARAM(&item as *const _ as isize)); }
}

fn tooltip(parent: HWND, instance: HINSTANCE) -> HWND {
    unsafe {
        let hwnd = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0x00000008),
            w!("tooltips_class32"),
            w!(""),
            windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x80000000 | 0x00000001 | TTS_ALWAYSTIP | TTS_NOPREFIX),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            parent,
            HMENU(std::ptr::null_mut()),
            instance,
            None,
        )
        .unwrap_or_default();
        SendMessageW(hwnd, TTM_SETDELAYTIME, WPARAM(3), LPARAM(500));
        SendMessageW(hwnd, TTM_SETDELAYTIME, WPARAM(2), LPARAM(3000));
        hwnd
    }
}

fn add_tip(app: &App, hwnd: HWND, text: &str) {
    if hwnd.0.is_null() || app.tip.0.is_null() {
        return;
    }
    let mut wide = wide(text);
    let info = TTTOOLINFOW {
        cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
        uFlags: TTF_IDISHWND | TTF_SUBCLASS,
        hwnd: app.hwnd,
        uId: hwnd.0 as usize,
        lpszText: PWSTR(wide.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        SendMessageW(app.tip, TTM_ADDTOOLW, WPARAM(0), LPARAM(&info as *const _ as isize));
        SendMessageW(app.tip, TTM_SETMAXTIPWIDTH, WPARAM(0), LPARAM(240));
    }
}

fn pick_folder(owner: HWND, current: &str, title: &str) -> Option<String> {
    unsafe {
        let dialog: IFileDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let options = dialog.GetOptions().ok()?;
        dialog.SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        let title_w = wide(title);
        dialog.SetTitle(PCWSTR(title_w.as_ptr())).ok()?;
        if !current.is_empty() {
            let wide = wide(current);
            let item: windows::core::Result<IShellItem> = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None);
            if let Ok(item) = item {
                let _ = dialog.SetFolder(&item);
            }
        }
        if dialog.Show(owner).is_err() {
            return None;
        }
        let result = dialog.GetResult().ok()?;
        let name = result.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let text = name.to_string().unwrap_or_default();
        CoTaskMemFree(Some(name.0 as *const _));
        if text.is_empty() { None } else { Some(text) }
    }
}

fn wheel(msg: &MSG) -> bool {
    if msg.message != WM_MOUSEWHEEL {
        return false;
    }
    unsafe {
        let point = POINT {
            x: (msg.lParam.0 as i16) as i32,
            y: ((msg.lParam.0 >> 16) as i16) as i32,
        };
        // Windows 2.03 scroll bars over list views and edits act like the
        // control underneath.
        let target = crate::aero::scroll_owner(WindowFromPoint(point));
        let class = class_name(target);
        if class == "OpusWorkList" || class == "SysListView32" || class == "ComboLBox" || class == "ListBox" {
            SendMessageW(target, WM_MOUSEWHEEL, msg.wParam, msg.lParam);
            return true;
        }
        let Some(main) = APP.with(|slot| slot.borrow().as_ref().map(|app| app.hwnd)) else {
            return false;
        };
        if main.0.is_null() || !owned_by(target, main) {
            return false;
        }
        SendMessageW(main, WM_MOUSEWHEEL, msg.wParam, msg.lParam);
        true
    }
}

fn owned_by(mut hwnd: HWND, root: HWND) -> bool {
    while !hwnd.0.is_null() {
        if hwnd == root {
            return true;
        }
        hwnd = unsafe { GetParent(hwnd).unwrap_or(HWND(std::ptr::null_mut())) };
    }
    false
}

fn shortcut(msg: &MSG) -> bool {
    if msg.message == WM_KEYDOWN {
        let key = msg.wParam.0;
        let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
        let up = key == VK_UP.0 as usize;
        let down = key == VK_DOWN.0 as usize;
        let tab = key == VK_TAB.0 as usize;
        if up || down || tab {
            let dir = if up || (tab && shift) { -1 } else { 1 };
            if move_edited_line(dir, tab) {
                return true;
            }
        }
        if tab {
            focus_next(shift);
            return true;
        }
        if key == VK_RETURN.0 as usize {
            return activate_default();
        }
    }
    if msg.message == 0x0106 {
        let ch = (msg.wParam.0 as u8 as char).to_ascii_uppercase();
        return activate_mnemonic(ch);
    }
    false
}

fn move_edited_line(dir: i32, tab: bool) -> bool {
    let focus = unsafe { GetFocus() };
    if class_name(focus) != "Edit" {
        return false;
    }
    let parent = unsafe { GetParent(focus).unwrap_or(HWND(std::ptr::null_mut())) };
    if class_name(parent) == "SysListView32" {
        let index = list_focus_index(parent);
        if index < 0 {
            return false;
        }
        if let Some((list, next)) = filename_neighbor(parent, index, dir) {
            unsafe { let _ = SetFocus(parent); }
            edit_filename(list, next);
            return true;
        }
        if tab {
            unsafe { let _ = SetFocus(parent); }
            return false;
        }
        return true;
    }
    let id = child_id(focus);
    if !(ID_TITLE_FIELD..ID_TITLE_SAME).contains(&id) || !enabled_visible(focus) {
        return false;
    }
    if let Some(next) = title_neighbor(focus, dir) {
        reveal_window(next);
        unsafe {
            let _ = SetFocus(next);
            SendMessageW(next, 0x00B1, WPARAM(0), LPARAM(-1));
        }
        return true;
    }
    if tab {
        return false;
    }
    true
}

fn filename_neighbor(list: HWND, index: i32, dir: i32) -> Option<(HWND, i32)> {
    let lists = filename_lists();
    let pos = lists.iter().position(|(hwnd, _)| *hwnd == list)?;
    let count = lists[pos].1;
    let next = index + dir;
    if next >= 0 && next < count {
        return Some((list, next));
    }
    if dir > 0 {
        lists.iter().skip(pos + 1).find(|(_, count)| *count > 0).map(|(hwnd, _)| (*hwnd, 0))
    } else {
        lists.iter().take(pos).rev().find(|(_, count)| *count > 0).map(|(hwnd, count)| (*hwnd, count - 1))
    }
}

fn filename_lists() -> Vec<(HWND, i32)> {
    tab_children()
        .into_iter()
        .filter(|hwnd| class_name(*hwnd) == "SysListView32" && (ID_LIST..ID_ALBUM).contains(&child_id(*hwnd)))
        .map(|hwnd| {
            let count = unsafe { SendMessageW(hwnd, LVM_GETITEMCOUNT, WPARAM(0), LPARAM(0)).0.max(0) as i32 };
            (hwnd, count)
        })
        .collect()
}

fn title_neighbor(current: HWND, dir: i32) -> Option<HWND> {
    let fields: Vec<HWND> = tab_children()
        .into_iter()
        .filter(|hwnd| class_name(*hwnd) == "Edit" && (ID_TITLE_FIELD..ID_TITLE_SAME).contains(&child_id(*hwnd)) && enabled_visible(*hwnd))
        .collect();
    let pos = fields.iter().position(|hwnd| *hwnd == current)?;
    let next = pos as i32 + dir;
    if next < 0 || next >= fields.len() as i32 {
        None
    } else {
        Some(fields[next as usize])
    }
}

fn tab_children() -> Vec<HWND> {
    let host = APP.with(|slot| slot.borrow().as_ref().map(|app| app.pane()).unwrap_or(HWND(std::ptr::null_mut())));
    if host.0.is_null() {
        return Vec::new();
    }
    let mut stops = Vec::new();
    unsafe {
        let mut child_hwnd = neighbor(host, GW_CHILD);
        while !child_hwnd.0.is_null() {
            stops.push(child_hwnd);
            child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
        }
    }
    stops.reverse();
    stops
}

fn list_focus_index(list: HWND) -> i32 {
    unsafe { SendMessageW(list, 0x100C, WPARAM(usize::MAX), LPARAM(0x0001)).0 as i32 }
}

fn edit_filename(list: HWND, index: i32) {
    unsafe { SendMessageW(list, 0x1013, WPARAM(index as usize), LPARAM(0)); }
    if let Some(rect) = list_item_host_rect(list, index) {
        reveal_host(rect);
    }
    unsafe {
        let edit = SendMessageW(list, LVM_EDITLABELW, WPARAM(index as usize), LPARAM(0));
        let edit = HWND(edit.0 as *mut _);
        if !edit.0.is_null() {
            SendMessageW(edit, 0x00B1, WPARAM(0), LPARAM(-1));
        }
    }
}

fn list_item_host_rect(list: HWND, index: i32) -> Option<RECT> {
    let host = APP.with(|slot| slot.borrow().as_ref().map(|app| app.host).unwrap_or(HWND(std::ptr::null_mut())));
    if host.0.is_null() {
        return None;
    }
    let mut rect = RECT::default();
    let ok = unsafe { SendMessageW(list, 0x100E, WPARAM(index as usize), LPARAM(&mut rect as *mut RECT as isize)) };
    if ok.0 == 0 {
        return None;
    }
    map_rect(list, host, &mut rect);
    Some(rect)
}

fn reveal_window(hwnd: HWND) {
    let host = APP.with(|slot| slot.borrow().as_ref().map(|app| app.host).unwrap_or(HWND(std::ptr::null_mut())));
    if host.0.is_null() || hwnd.0.is_null() {
        return;
    }
    let mut rect = RECT::default();
    unsafe { let _ = GetWindowRect(hwnd, &mut rect); }
    map_rect(HWND(std::ptr::null_mut()), host, &mut rect);
    reveal_host(rect);
}

fn map_rect(from: HWND, to: HWND, rect: &mut RECT) {
    let mut points = [POINT { x: rect.left, y: rect.top }, POINT { x: rect.right, y: rect.bottom }];
    unsafe { MapWindowPoints(from, to, &mut points); }
    rect.left = points[0].x;
    rect.top = points[0].y;
    rect.right = points[1].x;
    rect.bottom = points[1].y;
}

fn reveal_host(rect: RECT) {
    APP.with(|slot| {
        let Ok(mut borrow) = slot.try_borrow_mut() else { return; };
        let Some(app) = borrow.as_mut() else { return; };
        app.reveal_host_rect(rect);
    });
}

fn enabled_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowEnabled(hwnd).0 != 0 && IsWindowVisible(hwnd).0 != 0 }
}

fn focus_next(shift: bool) {
    APP.with(|slot| {
        let borrow = slot.borrow();
        let Some(app) = borrow.as_ref() else { return; };
        let mut stops = Vec::new();
        unsafe {
            let mut child_hwnd = neighbor(app.pane(), GW_CHILD);
            while !child_hwnd.0.is_null() {
                let style = GetWindowLongPtrW(child_hwnd, GWL_STYLE) as u32;
                if style & 0x00010000 != 0 && IsWindowVisible(child_hwnd).0 != 0 && IsWindowEnabled(child_hwnd).0 != 0 {
                    stops.push(child_hwnd);
                }
                child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
            }
        }
        stops.reverse();
        if unsafe { IsWindowEnabled(app.back).0 != 0 } {
            stops.push(app.back);
        }
        if unsafe { IsWindowEnabled(app.next).0 != 0 } {
            stops.push(app.next);
        }
        if stops.is_empty() {
            return;
        }
        let focus = unsafe { GetFocus() };
        let current = stops.iter().position(|hwnd| *hwnd == focus);
        let next = match current {
            Some(index) if shift => (index + stops.len() - 1) % stops.len(),
            Some(index) => (index + 1) % stops.len(),
            None => 0,
        };
        unsafe { let _ = SetFocus(stops[next]); }
    });
}

fn activate_default() -> bool {
    let focus = unsafe { GetFocus() };
    let class = class_name(focus);
    if class == "Edit" {
        let style = unsafe { GetWindowLongPtrW(focus, GWL_STYLE) as u32 };
        if style & 0x1000 != 0 {
            return false;
        }
        let parent = class_name(unsafe { GetParent(focus).unwrap_or(HWND(std::ptr::null_mut())) });
        if parent == "SysListView32" {
            return false;
        }
    }
    if class == "ComboBox" {
        let dropped = unsafe { SendMessageW(focus, 0x0157, WPARAM(0), LPARAM(0)).0 };
        if dropped != 0 {
            return false;
        }
    }
    if class == "Button" {
        unsafe { SendMessageW(focus, 0x00F5, WPARAM(0), LPARAM(0)); }
        return true;
    }
    APP.with(|slot| {
        let borrow = slot.borrow();
        let Some(app) = borrow.as_ref() else { return; };
        if unsafe { IsWindowEnabled(app.next).0 != 0 } {
            unsafe { SendMessageW(app.next, 0x00F5, WPARAM(0), LPARAM(0)); }
        }
    });
    true
}

fn activate_mnemonic(ch: char) -> bool {
    let targets = APP.with(|slot| {
        let borrow = slot.borrow();
        let Some(app) = borrow.as_ref() else { return Vec::new(); };
        let mut found = Vec::new();
        collect_mnemonics(app.pane(), ch, &mut found);
        collect_mnemonics(app.hwnd, ch, &mut found);
        found
    });
    let Some(target) = targets.into_iter().find(|hwnd| unsafe { IsWindowEnabled(*hwnd).0 != 0 && IsWindowVisible(*hwnd).0 != 0 }) else {
        return false;
    };
    let class = class_name(target);
    unsafe { let _ = SetFocus(target); }
    if class == "Button" {
        unsafe { SendMessageW(target, 0x00F5, WPARAM(0), LPARAM(0)); }
    }
    true
}

fn collect_mnemonics(parent: HWND, ch: char, out: &mut Vec<HWND>) {
    unsafe {
        let mut child_hwnd = neighbor(parent, GW_CHILD);
        while !child_hwnd.0.is_null() {
            let text = get_text(child_hwnd);
            if accel_char(&text) == Some(ch) {
                let class = class_name(child_hwnd);
                if class != "Static" {
                    out.push(child_hwnd);
                }
            }
            child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
        }
    }
    if out.is_empty() {
        let mut stops = Vec::new();
        unsafe {
            let mut child_hwnd = neighbor(parent, GW_CHILD);
            while !child_hwnd.0.is_null() {
                stops.push(child_hwnd);
                child_hwnd = neighbor(child_hwnd, GW_HWNDNEXT);
            }
        }
        stops.reverse();
        for (index, hwnd) in stops.iter().enumerate() {
            if accel_char(&get_text(*hwnd)) == Some(ch) && class_name(*hwnd) == "Static" {
                if let Some(next) = stops.iter().skip(index + 1).find(|candidate| {
                    let style = unsafe { GetWindowLongPtrW(**candidate, GWL_STYLE) as u32 };
                    style & 0x00010000 != 0
                }) {
                    out.push(*next);
                }
            }
        }
    }
}

fn enable(hwnd: HWND, value: bool) {
    unsafe { let _ = EnableWindow(hwnd, BOOL(i32::from(value))); }
}

fn neighbor(hwnd: HWND, cmd: windows::Win32::UI::WindowsAndMessaging::GET_WINDOW_CMD) -> HWND {
    unsafe { GetWindow(hwnd, cmd).unwrap_or(HWND(std::ptr::null_mut())) }
}

fn accel_char(text: &str) -> Option<char> {
    let chars: Vec<char> = text.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '&' && index + 1 < chars.len() && chars[index + 1] != '&' {
            return Some(chars[index + 1].to_ascii_uppercase());
        }
    }
    None
}

fn insert_index(at: InsertAt) -> usize {
    match at {
        InsertAt::Start => 0,
        InsertAt::End => 1,
        InsertAt::Before => 2,
        InsertAt::After => 3,
    }
}

fn flat_start(groups: &[Group], group: usize) -> usize {
    groups.iter().take(group).map(|group| group.items.len()).sum()
}

fn locate_flat(groups: &[Group], flat: usize) -> Option<(usize, usize)> {
    let mut seen = 0usize;
    for (group_index, group) in groups.iter().enumerate().take(GROUP_LIMIT) {
        if flat < seen + group.items.len() {
            return Some((group_index, flat - seen));
        }
        seen += group.items.len();
    }
    None
}

fn group_of(id: i32) -> usize {
    let base = match id {
        id if (ID_REMOVE..ID_INSERT).contains(&id) => ID_REMOVE,
        id if (ID_INSERT..ID_MODE).contains(&id) => ID_INSERT,
        id if (ID_MODE..ID_FIND).contains(&id) => ID_MODE,
        id if (ID_FIND..ID_STRIP).contains(&id) => ID_FIND,
        id if (ID_STRIP..ID_APPLY).contains(&id) => ID_STRIP,
        id if (ID_APPLY..ID_LIST).contains(&id) => ID_APPLY,
        id if (ID_LIST..ID_ALBUM).contains(&id) => ID_LIST,
        id if (ID_ALBUM..ID_ARTIST).contains(&id) => ID_ALBUM,
        _ => id,
    };
    (id - base) as usize
}

fn button_bits(default: bool) -> u32 {
    0x40000000 | 0x10000000 | 0x10000 | 0x04000000 | if default { 1 } else { 0 }
}

fn set_id(hwnd: HWND, id: i32) {
    unsafe { SetWindowLongPtrW(hwnd, GWLP_ID, id as isize); }
}

fn child_id(hwnd: HWND) -> i32 {
    unsafe { GetWindowLongPtrW(hwnd, GWLP_ID) as i32 }
}

fn child(parent: HWND, id: i32) -> windows::core::Result<HWND> {
    unsafe { GetDlgItem(parent, id) }
}

fn text_of(parent: HWND, id: i32) -> String {
    child(parent, id).map(get_text).unwrap_or_default()
}

fn checked_id(parent: HWND, id: i32) -> bool {
    child(parent, id).map(checked).unwrap_or(false)
}

fn combo_sel_id(parent: HWND, id: i32) -> usize {
    child(parent, id).map(combo_sel).unwrap_or(0)
}

fn get_text(hwnd: HWND) -> String {
    if hwnd.0.is_null() {
        return String::new();
    }
    unsafe {
        let len = SendMessageW(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0;
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        SendMessageW(hwnd, WM_GETTEXT, WPARAM(buf.len()), LPARAM(buf.as_mut_ptr() as isize));
        let end = buf.iter().position(|ch| *ch == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }
}

fn set_text(hwnd: HWND, text: &str) {
    if hwnd.0.is_null() || get_text(hwnd) == text {
        return;
    }
    let wide = wide(text);
    unsafe { let _ = SetWindowTextW(hwnd, PCWSTR(wide.as_ptr())); }
}

fn set_enabled(hwnd: HWND, on: bool) {
    if hwnd.0.is_null() {
        return;
    }
    let enabled = unsafe { IsWindowEnabled(hwnd).0 != 0 };
    if enabled != on {
        enable(hwnd, on);
    }
}

fn combo_add(hwnd: HWND, text: &str) {
    let wide = wide(text);
    unsafe { SendMessageW(hwnd, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize)); }
}

fn combo_clear(hwnd: HWND) {
    unsafe { SendMessageW(hwnd, CB_RESETCONTENT, WPARAM(0), LPARAM(0)); }
}

fn combo_sel(hwnd: HWND) -> usize {
    let sel = unsafe { SendMessageW(hwnd, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 };
    if sel < 0 { 0 } else { sel as usize }
}

fn set_combo(hwnd: HWND, index: usize) {
    unsafe { SendMessageW(hwnd, CB_SETCURSEL, WPARAM(index), LPARAM(0)); }
}

fn checked(hwnd: HWND) -> bool {
    unsafe { SendMessageW(hwnd, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1 }
}

fn set_check(hwnd: HWND, on: bool) {
    unsafe { SendMessageW(hwnd, BM_SETCHECK, WPARAM(if on { 1 } else { 0 }), LPARAM(0)); }
}

fn pw_string(text: PWSTR) -> String {
    if text.0.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0usize;
        while *text.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(text.0, len))
    }
}

fn class_name(hwnd: HWND) -> String {
    if hwnd.0.is_null() {
        return String::new();
    }
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

fn client_of(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    unsafe { let _ = GetClientRect(hwnd, &mut rect); }
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | (g << 8) | (b << 16))
}

fn white_brush() -> HBRUSH {
    WHITE.get()
}

fn make_font(pt: i32, dpi: i32, bold: bool) -> HFONT {
    unsafe {
        CreateFontW(
            -((pt * dpi + 36) / 72),
            0,
            0,
            0,
            if bold { 700 } else { 400 },
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            w!("MS Sans Serif"),
        )
    }
}

fn font_bits(font: HFONT) -> usize {
    font.0 as usize
}
