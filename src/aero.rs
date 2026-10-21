//! Windows 2.03 look: flat black frames, inverted caption, 50% checkerboard dither
//! (the pattern Windows 1.x/2.x used for gray on EGA/CGA).

use windows::core::w;
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, ClientToScreen, CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
    CreatePen, CreatePatternBrush, CreateRectRgn, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse,
    EndPaint, ExcludeClipRect, FillRect, GetDC, GetRgnBox, GetStockObject, GetTextExtentPoint32W, GetTextMetricsW,
    GetWindowDC, GetWindowRgn, InvalidateRect, ReleaseDC, SelectClipRgn, SelectObject, SetBkMode, SetTextColor,
    SetWindowRgn, DEFAULT_QUALITY, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, HBRUSH, HDC, HFONT, HPEN, HRGN, NULL_BRUSH, PS_SOLID, TEXTMETRICW, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Controls::{GetComboBoxInfo, COMBOBOXINFO, EM_SETMARGINS, PBM_SETPOS, PBM_SETRANGE32};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    IsWindowEnabled, ReleaseCapture, SetCapture, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, GetClassNameW, GetClientRect, GetDlgCtrlID, GetParent, GetWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowTextW, KillTimer, LoadCursorW, RegisterClassW, SendMessageW,
    SetWindowLongPtrW, SetWindowPos, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, EC_LEFTMARGIN, EC_RIGHTMARGIN,
    GW_CHILD, GW_HWNDNEXT, GWL_STYLE, GWLP_USERDATA, GWLP_WNDPROC, HWND_TOP, IDC_ARROW, NCCALCSIZE_PARAMS,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WINDOW_LONG_PTR_INDEX, WM_COMMAND,
    WM_CREATE, WM_DESTROY, WM_ENABLE, WM_ERASEBKGND, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCCALCSIZE, WM_NCPAINT, WM_PAINT, WM_SETFOCUS, WM_SETFONT, WM_SIZE, WNDCLASSW,
};

pub const CLASS: &str = "OpusAeroBar";
pub const BUTTON: &str = "OpusAeroButton";

const BAR_POS: i32 = 0;
const BAR_MAX: i32 = 8;

const WHITE: (u8, u8, u8) = (255, 255, 255);
const BLACK: (u8, u8, u8) = (0, 0, 0);

static DITHER: std::sync::OnceLock<isize> = std::sync::OnceLock::new();

fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | (g << 8) | (b << 16))
}

fn rgb3(c: (u8, u8, u8)) -> COLORREF {
    rgb(c.0 as u32, c.1 as u32, c.2 as u32)
}

fn mix(a: COLORREF, b: COLORREF, t: f32) -> COLORREF {
    let t = t.clamp(0.0, 1.0);
    let ar = a.0 & 0xFF;
    let ag = (a.0 >> 8) & 0xFF;
    let ab = (a.0 >> 16) & 0xFF;
    let br = b.0 & 0xFF;
    let bg = (b.0 >> 8) & 0xFF;
    let bb = (b.0 >> 16) & 0xFF;
    rgb(
        (ar as f32 + (br as f32 - ar as f32) * t) as u32,
        (ag as f32 + (bg as f32 - ag as f32) * t) as u32,
        (ab as f32 + (bb as f32 - ab as f32) * t) as u32,
    )
}

pub fn dither_brush() -> HBRUSH {
    HBRUSH(*DITHER.get_or_init(|| unsafe {
        // 50% checkerboard, the gray Windows 2.x drew on 2-color / EGA.
        let bits: [u16; 8] = [
            0x5555, 0xAAAA, 0x5555, 0xAAAA, 0x5555, 0xAAAA, 0x5555, 0xAAAA,
        ];
        let bmp = CreateBitmap(8, 8, 1, 1, Some(bits.as_ptr() as *const _));
        let br = CreatePatternBrush(bmp);
        let _ = DeleteObject(bmp);
        br.0 as isize
    }) as *mut _)
}

pub fn fill(hdc: HDC, r: RECT, c: COLORREF) {
    unsafe {
        let br = CreateSolidBrush(c);
        let _ = FillRect(hdc, &r, br);
        let _ = DeleteObject(br);
    }
}

fn fill3(hdc: HDC, r: RECT, c: (u8, u8, u8)) {
    fill(hdc, r, rgb3(c));
}

pub fn fill_dither(hdc: HDC, r: RECT) {
    unsafe {
        let _ = FillRect(hdc, &r, dither_brush());
    }
}

pub fn vline(hdc: HDC, x: i32, y0: i32, y1: i32, c: COLORREF) {
    fill(hdc, RECT { left: x, top: y0, right: x + 1, bottom: y1 }, c);
}

pub fn hline(hdc: HDC, x0: i32, x1: i32, y: i32, c: COLORREF) {
    fill(hdc, RECT { left: x0, top: y, right: x1, bottom: y + 1 }, c);
}

fn vline3(hdc: HDC, x: i32, y0: i32, y1: i32, c: (u8, u8, u8)) {
    vline(hdc, x, y0, y1, rgb3(c));
}

fn hline3(hdc: HDC, x0: i32, x1: i32, y: i32, c: (u8, u8, u8)) {
    hline(hdc, x0, x1, y, rgb3(c));
}

pub fn frame_rect(hdc: HDC, r: RECT, c: COLORREF) {
    hline(hdc, r.left, r.right, r.top, c);
    hline(hdc, r.left, r.right, r.bottom - 1, c);
    vline(hdc, r.left, r.top, r.bottom, c);
    vline(hdc, r.right - 1, r.top, r.bottom, c);
}

fn black_box(hdc: HDC, r: RECT) {
    thick_box(hdc, r, 3);
}

fn thick_box(hdc: HDC, r: RECT, n: i32) {
    for i in 0..n {
        frame_rect(hdc, inset(r, i), rgb3(BLACK));
    }
}

pub fn hit(r: RECT, p: POINT) -> bool {
    p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom
}

pub fn inset(r: RECT, n: i32) -> RECT {
    RECT {
        left: r.left + n,
        top: r.top + n,
        right: r.right - n,
        bottom: r.bottom - n,
    }
}

pub fn scale(px: i32, dpi: i32) -> i32 {
    px * dpi / 96
}

pub fn scroll_width(dpi: i32) -> i32 {
    scale(20, dpi)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SbHit {
    None,
    Up,
    Down,
    Thumb,
    PageUp,
    PageDown,
}

pub struct Scrollbar {
    pub up: RECT,
    pub down: RECT,
    pub track: RECT,
    pub thumb: RECT,
    pub body: RECT,
}

pub fn scrollbar(area: RECT, pos: i32, page: i32, content: i32, dpi: i32) -> Option<Scrollbar> {
    if content <= page || area.right - area.left < 8 || area.bottom - area.top < scale(48, dpi) {
        return None;
    }
    let w = area.right - area.left;
    let up = RECT { left: area.left, top: area.top, right: area.right, bottom: area.top + w };
    let down = RECT { left: area.left, top: area.bottom - w, right: area.right, bottom: area.bottom };
    let track = RECT { left: area.left, top: up.bottom, right: area.right, bottom: down.top };
    let track_h = (track.bottom - track.top).max(1);
    let max = (content - page).max(1);
    let thumb_h = ((track_h as i64 * page as i64 / content.max(1) as i64) as i32).clamp(w, track_h);
    let travel = (track_h - thumb_h).max(0);
    let thumb_y = track.top + (travel as i64 * pos.clamp(0, max) as i64 / max as i64) as i32;
    let thumb = RECT { left: area.left, top: thumb_y, right: area.right, bottom: thumb_y + thumb_h };
    Some(Scrollbar { up, down, track, thumb, body: area })
}

pub fn paint_scrollbar(hdc: HDC, sb: &Scrollbar) {
    fill_dither(hdc, sb.body);
    thick_box(hdc, sb.body, 2);
    paint_arrow_box(hdc, sb.up, -1);
    paint_arrow_box(hdc, sb.down, 1);
    fill3(hdc, sb.thumb, WHITE);
    thick_box(hdc, sb.thumb, 2);
}

fn paint_arrow_box(hdc: HDC, r: RECT, dir: i32) {
    fill3(hdc, r, WHITE);
    thick_box(hdc, r, 2);
    let cx = (r.left + r.right) / 2;
    let cy = (r.top + r.bottom) / 2;
    arrow(hdc, cx, cy, ((r.right - r.left) / 2 - 4).max(4), dir, BLACK);
}

pub fn scrollbar_hit(sb: &Scrollbar, p: POINT) -> SbHit {
    if hit(sb.up, p) {
        SbHit::Up
    } else if hit(sb.down, p) {
        SbHit::Down
    } else if hit(sb.thumb, p) {
        SbHit::Thumb
    } else if hit(sb.track, p) && p.y < sb.thumb.top {
        SbHit::PageUp
    } else if hit(sb.track, p) {
        SbHit::PageDown
    } else {
        SbHit::None
    }
}

pub fn register(instance: windows::Win32::Foundation::HINSTANCE) {
    let _ = dither_brush();
    unsafe {
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(bar_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("OpusAeroBar"),
            cbWndExtra: 16,
            ..Default::default()
        };
        let _ = RegisterClassW(&class);
        let button = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(button_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("OpusAeroButton"),
            cbWndExtra: 32,
            ..Default::default()
        };
        let _ = RegisterClassW(&button);
        let check = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(check_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("OpusWin2Check"),
            cbWndExtra: 32,
            ..Default::default()
        };
        let _ = RegisterClassW(&check);
        let radio = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(radio_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("OpusWin2Radio"),
            cbWndExtra: 32,
            ..Default::default()
        };
        let _ = RegisterClassW(&radio);
    }
}

pub fn untheme(hwnd: HWND) {
    unsafe {
        let lib = LoadLibraryW(w!("uxtheme.dll")).unwrap_or_default();
        if lib.0.is_null() {
            return;
        }
        let Some(proc) = GetProcAddress(lib, windows::core::s!("SetWindowTheme")) else {
            return;
        };
        type SetTheme = unsafe extern "system" fn(HWND, *const u16, *const u16) -> i32;
        let set: SetTheme = std::mem::transmute(proc);
        let empty: [u16; 1] = [0];
        let _ = set(hwnd, empty.as_ptr(), empty.as_ptr());
    }
}

pub fn thicken(hwnd: HWND) {
    unsafe {
        let mut name = [0u16; 32];
        let n = GetClassNameW(hwnd, &mut name);
        let class = String::from_utf16_lossy(&name[..n as usize]);
        if class != "Edit" && class != "SysListView32" && class != "ComboBox" && class != "OpusWorkList" {
            return;
        }
        let old = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, old);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, thick_proc as isize);
        if class == "Edit" {
            pad_edit(hwnd);
        }
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

unsafe extern "system" fn thick_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let old = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    let prev: windows::Win32::UI::WindowsAndMessaging::WNDPROC = std::mem::transmute(old);
    const EDGE: i32 = 3;
    let mut cname = [0u16; 32];
    let cn = GetClassNameW(hwnd, &mut cname);
    let class = String::from_utf16_lossy(&cname[..cn as usize]);
    let combo = class == "ComboBox";
    let edit = class == "Edit";
    if msg == WM_NCCALCSIZE && !combo {
        let _ = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
        if wparam.0 != 0 {
            let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
            inset_client(&mut params.rgrc[0], hwnd, edit, EDGE);
        } else if lparam.0 != 0 {
            let rect = &mut *(lparam.0 as *mut RECT);
            inset_client(rect, hwnd, edit, EDGE);
        }
        return LRESULT(0);
    }
    if msg == WM_NCPAINT && !combo {
        paint_nc_frame(hwnd, EDGE);
        return LRESULT(0);
    }
    if msg == WM_PAINT && combo {
        let result = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
        paint_combo_face(hwnd);
        return result;
    }
    if msg == WM_PAINT && edit && !IsWindowEnabled(hwnd).as_bool() {
        paint_disabled_edit(hwnd);
        return LRESULT(0);
    }
    if msg == WM_ERASEBKGND && edit && !IsWindowEnabled(hwnd).as_bool() {
        let mut c = RECT::default();
        let _ = GetClientRect(hwnd, &mut c);
        fill_dither(HDC(wparam.0 as *mut _), c);
        return LRESULT(1);
    }
    if msg == WM_SETFONT && edit {
        let result = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
        pad_edit(hwnd);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        return result;
    }
    if msg == WM_ENABLE && !combo {
        let result = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
        let _ = InvalidateRect(hwnd, None, true);
        paint_nc_frame(hwnd, EDGE);
        return result;
    }
    if msg == WM_SIZE && !combo {
        let result = CallWindowProcW(prev, hwnd, msg, wparam, lparam);
        paint_nc_frame(hwnd, EDGE);
        return result;
    }
    CallWindowProcW(prev, hwnd, msg, wparam, lparam)
}

fn inset_client(rect: &mut RECT, hwnd: HWND, edit: bool, edge: i32) {
    rect.left += edge;
    rect.top += edge;
    rect.right -= edge;
    rect.bottom -= edge;
    if !edit {
        return;
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    if style & 0x0004 != 0 {
        return;
    }
    let font_h = edit_font_height(hwnd);
    let h = rect.bottom - rect.top;
    let want = font_h + 2;
    let extra = h - want;
    if extra >= 2 {
        let top = extra / 2;
        rect.top += top;
        rect.bottom -= extra - top;
    }
}

fn edit_font_height(hwnd: HWND) -> i32 {
    unsafe {
        let dc = GetDC(hwnd);
        let font = HFONT(SendMessageW(hwnd, 0x0031, WPARAM(0), LPARAM(0)).0 as *mut _);
        let old = if !font.0.is_null() {
            SelectObject(dc, font)
        } else {
            std::mem::zeroed()
        };
        let mut tm = TEXTMETRICW::default();
        let _ = GetTextMetricsW(dc, &mut tm);
        if !font.0.is_null() {
            SelectObject(dc, old);
        }
        let _ = ReleaseDC(hwnd, dc);
        tm.tmHeight.max(12)
    }
}

fn pad_edit(hwnd: HWND) {
    unsafe {
        let _ = SendMessageW(
            hwnd,
            EM_SETMARGINS,
            WPARAM((EC_LEFTMARGIN | EC_RIGHTMARGIN) as usize),
            LPARAM(6 | (6 << 16)),
        );
    }
}

fn paint_disabled_edit(hwnd: HWND) {
    unsafe {
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut c = RECT::default();
        let _ = GetClientRect(hwnd, &mut c);
        fill_dither(hdc, c);
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            let font = HFONT(SendMessageW(hwnd, 0x0031, WPARAM(0), LPARAM(0)).0 as *mut _);
            let old = if !font.0.is_null() {
                SelectObject(hdc, font)
            } else {
                std::mem::zeroed()
            };
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, rgb3(BLACK));
            let mut tr = RECT {
                left: 2,
                top: 0,
                right: c.right - 2,
                bottom: c.bottom,
            };
            let mut wide: Vec<u16> = buf[..len as usize].to_vec();
            let _ = DrawTextW(
                hdc,
                &mut wide,
                &mut tr,
                windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
                    DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0 | DT_END_ELLIPSIS.0,
                ),
            );
            if !font.0.is_null() {
                SelectObject(hdc, old);
            }
        }
        let _ = EndPaint(hwnd, &ps);
    }
}

fn paint_nc_frame(hwnd: HWND, edge: i32) {
    unsafe {
        let dc = GetWindowDC(hwnd);
        let mut wr = RECT::default();
        let _ = GetWindowRect(hwnd, &mut wr);
        let w = wr.right - wr.left;
        let h = wr.bottom - wr.top;
        let mut cr = RECT::default();
        let _ = GetClientRect(hwnd, &mut cr);
        let mut origin = POINT { x: 0, y: 0 };
        let _ = ClientToScreen(hwnd, &mut origin);
        let cl = origin.x - wr.left;
        let ct = origin.y - wr.top;
        let _ = ExcludeClipRect(dc, cl, ct, cl + cr.right, ct + cr.bottom);
        fill3(dc, RECT { left: 0, top: 0, right: w, bottom: h }, WHITE);
        thick_box(dc, RECT { left: 0, top: 0, right: w, bottom: h }, edge);
        let _ = SelectClipRgn(dc, HRGN(std::ptr::null_mut()));
        let _ = ReleaseDC(hwnd, dc);
    }
}

pub fn combo_face_height(hwnd: HWND) -> i32 {
    unsafe {
        let mut info = COMBOBOXINFO {
            cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
            ..Default::default()
        };
        if GetComboBoxInfo(hwnd, &mut info).is_ok() {
            let h = info.rcItem.bottom.max(info.rcButton.bottom);
            if (9..=96).contains(&h) {
                return h;
            }
        }
        let rgn = CreateRectRgn(0, 0, 0, 0);
        if GetWindowRgn(hwnd, rgn).0 != 0 {
            let mut boxr = RECT::default();
            if GetRgnBox(rgn, &mut boxr).0 != 0 {
                let h = boxr.bottom - boxr.top;
                let _ = DeleteObject(rgn);
                if (9..=96).contains(&h) {
                    return h;
                }
            } else {
                let _ = DeleteObject(rgn);
            }
        } else {
            let _ = DeleteObject(rgn);
        }
        26
    }
}

fn paint_combo_face(hwnd: HWND) {
    unsafe {
        let dc = GetWindowDC(hwnd);
        let mut wr = RECT::default();
        let _ = GetWindowRect(hwnd, &mut wr);
        let w = wr.right - wr.left;
        let h = combo_face_height(hwnd).max(22);
        let face = RECT { left: 0, top: 0, right: w, bottom: h };
        fill3(dc, face, WHITE);
        black_box(dc, face);
        let aw = h.max(16);
        let btn = RECT { left: w - aw, top: 0, right: w, bottom: h };
        fill3(dc, btn, WHITE);
        black_box(dc, btn);
        arrow(
            dc,
            (btn.left + btn.right) / 2,
            (btn.top + btn.bottom) / 2,
            (aw / 2 - 3).max(4),
            1,
            BLACK,
        );
        let mut buf = [0u16; 128];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            let text = String::from_utf16_lossy(&buf[..len as usize]);
            let font = HFONT(SendMessageW(hwnd, 0x0031, WPARAM(0), LPARAM(0)).0 as *mut _);
            let old = if !font.0.is_null() { SelectObject(dc, font) } else { std::mem::zeroed() };
            let mut tr = RECT { left: 8, top: 0, right: btn.left - 4, bottom: h };
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, rgb3(BLACK));
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let _ = DrawTextW(
                dc,
                &mut wide,
                &mut tr,
                windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
                    DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0 | DT_END_ELLIPSIS.0,
                ),
            );
            if !font.0.is_null() {
                SelectObject(dc, old);
            }
        }
        let _ = ReleaseDC(hwnd, dc);
    }
}

unsafe extern "system" fn bar_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            set_long(hwnd, BAR_POS, 0);
            set_long(hwnd, BAR_MAX, 1000);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(hwnd, 1);
            LRESULT(0)
        }
        msg if msg == PBM_SETRANGE32 => {
            let max = (lparam.0 as i32).max((wparam.0 as i32) + 1);
            set_long(hwnd, BAR_MAX, max as isize);
            LRESULT(0)
        }
        msg if msg == PBM_SETPOS => {
            let max = get_long(hwnd, BAR_MAX).max(1);
            let pos = (wparam.0 as isize).clamp(0, max);
            let old = get_long(hwnd, BAR_POS);
            set_long(hwnd, BAR_POS, pos);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(old)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_window(hwnd);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn paint_window(hwnd: HWND) {
    unsafe {
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let w = client.right;
        let h = client.bottom;
        if w <= 0 || h <= 0 {
            let _ = EndPaint(hwnd, &ps);
            return;
        }
        let mem = CreateCompatibleDC(hdc);
        let bitmap = CreateCompatibleBitmap(hdc, w, h);
        let old_bmp = SelectObject(mem, bitmap);
        fill3(mem, client, WHITE);
        let pos = get_long(hwnd, BAR_POS);
        let max = get_long(hwnd, BAR_MAX).max(1);
        paint(mem, client, pos as f64 / max as f64, false, false);
        let _ = BitBlt(hdc, 0, 0, w, h, mem, 0, 0, windows::Win32::Graphics::Gdi::SRCCOPY);
        SelectObject(mem, old_bmp);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem);
        let _ = EndPaint(hwnd, &ps);
    }
}

/// Flat black-bordered well; fill is solid black (Win 2.x had no glossy bar).
pub fn paint(hdc: HDC, bounds: RECT, fraction: f64, failed: bool, _animate: bool) {
    if bounds.right - bounds.left < 6 || bounds.bottom - bounds.top < 6 {
        return;
    }
    fill3(hdc, bounds, WHITE);
    black_box(hdc, bounds);
    let inner = inset(bounds, 2);
    fill_dither(hdc, inner);
    let w = (inner.right - inner.left).max(0);
    let fill_w = ((w as f64) * fraction.clamp(0.0, 1.0)).round() as i32;
    if fill_w > 0 {
        fill3(
            hdc,
            RECT {
                left: inner.left,
                top: inner.top,
                right: inner.left + fill_w.min(w),
                bottom: inner.bottom,
            },
            BLACK,
        );
    }
}

pub fn enable_glass(_hwnd: HWND, _active: bool) {}

pub fn caption_font(dpi: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -((12 * dpi + 36) / 72),
            0,
            0,
            0,
            700,
            0,
            0,
            0,
            1,
            0,
            0,
            DEFAULT_QUALITY.0 as u32,
            0,
            w!("MS Sans Serif"),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CapHot {
    None,
    Min,
    Max,
    Close,
    Icon,
    Caption,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CapBtn {
    Min,
    Max,
    Restore,
    Close,
}

pub struct Layout {
    pub window: RECT,
    pub caption: RECT,
    pub btn_min: RECT,
    pub btn_max: RECT,
    pub btn_close: RECT,
    pub icon: RECT,
    pub title: RECT,
    pub inner: RECT,
    pub border: i32,
    pub caption_h: i32,
    pub radius: i32,
}

pub fn layout(window: RECT, dpi: i32, maximized: bool) -> Layout {
    let border = if maximized { 0 } else { scale(5, dpi) };
    let caption_h = scale(22, dpi);
    let btn = caption_h;
    let gap = 0;
    let inner = RECT {
        left: window.left + border,
        top: window.top + border,
        right: window.right - border,
        bottom: window.bottom - border,
    };
    let caption = RECT {
        left: inner.left,
        top: inner.top,
        right: inner.right,
        bottom: inner.top + caption_h,
    };
    let by = caption.top + (caption_h - btn) / 2;
    let close = RECT {
        left: caption.left + gap,
        top: by,
        right: caption.left + gap + btn,
        bottom: by + btn,
    };
    let max = RECT {
        left: caption.right - gap - btn,
        top: by,
        right: caption.right - gap,
        bottom: by + btn,
    };
    let min = RECT {
        left: max.left - gap - btn,
        top: by,
        right: max.left - gap,
        bottom: by + btn,
    };
    let title = RECT {
        left: close.right + scale(4, dpi),
        top: caption.top,
        right: min.left - scale(4, dpi),
        bottom: caption.bottom,
    };
    Layout {
        window,
        caption,
        btn_min: min,
        btn_max: max,
        btn_close: close,
        icon: close,
        title,
        inner: RECT {
            left: inner.left,
            top: caption.bottom,
            right: inner.right,
            bottom: inner.bottom,
        },
        border,
        caption_h,
        radius: 0,
    }
}

pub fn apply_region(hwnd: HWND, _w: i32, _h: i32, _dpi: i32, _maximized: bool) {
    unsafe {
        SetWindowRgn(hwnd, HRGN(std::ptr::null_mut()), BOOL(1));
    }
}

pub fn window_frame(hdc: HDC, r: RECT, border: i32, _active: bool) {
    fill3(hdc, r, WHITE);
    if border > 0 {
        fill3(hdc, RECT { left: r.left, top: r.top, right: r.right, bottom: r.top + border }, BLACK);
        fill3(hdc, RECT { left: r.left, top: r.bottom - border, right: r.right, bottom: r.bottom }, BLACK);
        fill3(hdc, RECT { left: r.left, top: r.top, right: r.left + border, bottom: r.bottom }, BLACK);
        fill3(hdc, RECT { left: r.right - border, top: r.top, right: r.right, bottom: r.bottom }, BLACK);
    }
}

pub fn is_button(hot: CapHot) -> bool {
    matches!(hot, CapHot::Min | CapHot::Max | CapHot::Close | CapHot::Icon)
}

pub fn paint_caption(
    hdc: HDC,
    _hwnd: HWND,
    l: &Layout,
    title: &str,
    font: HFONT,
    active: bool,
    maximized: bool,
    hot: CapHot,
    press: CapHot,
) {
    fill3(hdc, l.caption, WHITE);
    hline3(hdc, l.caption.left, l.caption.right, l.caption.bottom - 1, BLACK);
    let title = title.trim();
    let mut name: Vec<u16> = title.encode_utf16().collect();
    let mut size = windows::Win32::Foundation::SIZE::default();
    unsafe {
        let old = SelectObject(hdc, font);
        let _ = GetTextExtentPoint32W(hdc, &name, &mut size);
        SelectObject(hdc, old);
    }
    let tw = size.cx.max(8);
    let th = size.cy.max(12);
    let cx = (l.title.left + l.title.right) / 2;
    let cy = (l.caption.top + l.caption.bottom) / 2;
    let pad = 6;
    let badge = RECT {
        left: (cx - tw / 2 - pad).max(l.title.left),
        top: (cy - th / 2).max(l.caption.top + 1),
        right: (cx + tw / 2 + pad).min(l.title.right),
        bottom: (cy + th / 2).min(l.caption.bottom - 1),
    };
    if active {
        fill3(hdc, badge, BLACK);
        text_center(hdc, font, badge, title, WHITE);
    } else {
        black_box(hdc, badge);
        text_center(hdc, font, badge, title, BLACK);
    }
    let max_kind = if maximized { CapBtn::Restore } else { CapBtn::Max };
    caption_button(hdc, l.btn_close, CapBtn::Close, hot == CapHot::Close || hot == CapHot::Icon, press == CapHot::Close || press == CapHot::Icon, active);
    caption_button(hdc, l.btn_min, CapBtn::Min, hot == CapHot::Min, press == CapHot::Min, active);
    caption_button(hdc, l.btn_max, max_kind, hot == CapHot::Max, press == CapHot::Max, active);
}

fn text_center(hdc: HDC, font: HFONT, r: RECT, s: &str, color: (u8, u8, u8)) {
    if s.is_empty() {
        return;
    }
    let mut w: Vec<u16> = s.encode_utf16().collect();
    unsafe {
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, rgb3(color));
        let _ = DrawTextW(
            hdc,
            &mut w,
            &mut { r },
            windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
                DT_CENTER.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0,
            ),
        );
        SelectObject(hdc, old);
    }
}

fn text_left(hdc: HDC, font: HFONT, r: RECT, s: &str, color: (u8, u8, u8)) {
    if s.is_empty() {
        return;
    }
    let mut w: Vec<u16> = s.encode_utf16().collect();
    unsafe {
        let old = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, rgb3(color));
        let _ = DrawTextW(
            hdc,
            &mut w,
            &mut { r },
            windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
                DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0 | DT_END_ELLIPSIS.0,
            ),
        );
        SelectObject(hdc, old);
    }
}

pub fn caption_button(hdc: HDC, r: RECT, kind: CapBtn, _hover: bool, press: bool, _active: bool) {
    if press {
        fill3(hdc, r, BLACK);
    } else {
        fill3(hdc, r, WHITE);
    }
    black_box(hdc, r);
    let ink = if press { WHITE } else { BLACK };
    let cx = (r.left + r.right) / 2;
    let cy = (r.top + r.bottom) / 2;
    let half = ((r.right - r.left) / 2 - 4).max(4);
    match kind {
        CapBtn::Close => {
            fill(
                hdc,
                RECT {
                    left: r.left + 4,
                    top: cy - 1,
                    right: r.right - 4,
                    bottom: cy + 2,
                },
                rgb3(ink),
            );
        }
        CapBtn::Min => arrow(hdc, cx, cy, half, 1, ink),
        CapBtn::Max => arrow(hdc, cx, cy, half, -1, ink),
        CapBtn::Restore => {
            arrow(hdc, cx - 2, cy + 2, half - 1, 1, ink);
            arrow(hdc, cx + 2, cy - 2, half - 1, -1, ink);
        }
    }
}

fn arrow(hdc: HDC, cx: i32, cy: i32, half: i32, dir: i32, color: (u8, u8, u8)) {
    let half = half.max(4);
    let tip = cy + dir * half / 2;
    for i in 0..=half {
        let y = tip - i * dir;
        hline3(hdc, cx - i, cx + i + 1, y, color);
    }
}

pub fn hot_at(l: &Layout, p: POINT) -> CapHot {
    if hit(l.btn_close, p) {
        CapHot::Close
    } else if hit(l.btn_max, p) {
        CapHot::Max
    } else if hit(l.btn_min, p) {
        CapHot::Min
    } else if hit(l.caption, p) {
        CapHot::Caption
    } else {
        CapHot::None
    }
}

pub fn gripper(hdc: HDC, r: RECT) {
    // Win 2.x had no size grip; keep a tiny black corner so resize still reads.
    fill3(
        hdc,
        RECT {
            left: r.right - 8,
            top: r.bottom - 8,
            right: r.right - 2,
            bottom: r.bottom - 2,
        },
        BLACK,
    );
}

const BTN_HOVER: i32 = 0;
const BTN_PRESS: i32 = 8;
const BTN_FOCUS: i32 = 16;
const BTN_FONT: i32 = 24;

unsafe extern "system" fn button_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            set_long(hwnd, BTN_HOVER, 0);
            set_long(hwnd, BTN_PRESS, 0);
            set_long(hwnd, BTN_FOCUS, 0);
            set_long(hwnd, BTN_FONT, 0);
            LRESULT(0)
        }
        WM_SETFONT => {
            set_long(hwnd, BTN_FONT, wparam.0 as isize);
            if lparam.0 != 0 {
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            paint_button(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEMOVE => {
            if get_long(hwnd, BTN_HOVER) == 0 {
                set_long(hwnd, BTN_HOVER, 1);
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut tme);
            }
            LRESULT(0)
        }
        x if x == WM_MOUSELEAVE => {
            set_long(hwnd, BTN_HOVER, 0);
            set_long(hwnd, BTN_PRESS, 0);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            set_long(hwnd, BTN_PRESS, 1);
            let _ = SetCapture(hwnd);
            let _ = SetFocus(hwnd);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let pressed = get_long(hwnd, BTN_PRESS) != 0;
            set_long(hwnd, BTN_PRESS, 0);
            let _ = ReleaseCapture();
            let _ = InvalidateRect(hwnd, None, false);
            if pressed {
                let parent = GetParent(hwnd).unwrap_or(HWND(std::ptr::null_mut()));
                let id = GetDlgCtrlID(hwnd);
                let _ = SendMessageW(parent, WM_COMMAND, WPARAM(id as usize), LPARAM(hwnd.0 as isize));
            }
            LRESULT(0)
        }
        WM_SETFOCUS => {
            set_long(hwnd, BTN_FOCUS, 1);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            set_long(hwnd, BTN_FOCUS, 0);
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 == 0x0D || wparam.0 == 0x20 {
                let parent = GetParent(hwnd).unwrap_or(HWND(std::ptr::null_mut()));
                let id = GetDlgCtrlID(hwnd);
                let _ = SendMessageW(parent, WM_COMMAND, WPARAM(id as usize), LPARAM(hwnd.0 as isize));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn paint_button(hwnd: HWND) {
    unsafe {
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        let press = get_long(hwnd, BTN_PRESS) != 0;
        let focus = get_long(hwnd, BTN_FOCUS) != 0;
        let enabled = IsWindowEnabled(hwnd).as_bool();
        let font = HFONT(get_long(hwnd, BTN_FONT) as *mut _);
        let old = if !font.0.is_null() { SelectObject(hdc, font) } else { std::mem::zeroed() };
        command_button(hdc, client, &button_text(hwnd), enabled, false, press, focus, false);
        if !font.0.is_null() {
            SelectObject(hdc, old);
        }
        let _ = EndPaint(hwnd, &ps);
    }
}

fn button_text(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

pub fn command_button(
    hdc: HDC,
    bounds: RECT,
    text: &str,
    enabled: bool,
    _hover: bool,
    press: bool,
    focus: bool,
    is_default: bool,
) {
    if press {
        fill3(hdc, bounds, BLACK);
        black_box(hdc, bounds);
    } else {
        fill3(hdc, bounds, WHITE);
        unsafe {
            let pen = CreatePen(PS_SOLID, 3, rgb3(BLACK));
            let old_pen = SelectObject(hdc, HPEN(pen.0));
            let empty = GetStockObject(NULL_BRUSH);
            let old_brush = SelectObject(hdc, empty);
            let rad = ((bounds.bottom - bounds.top) / 2).clamp(6, 12);
            let _ = windows::Win32::Graphics::Gdi::RoundRect(
                hdc,
                bounds.left,
                bounds.top,
                bounds.right,
                bounds.bottom,
                rad,
                rad,
            );
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            let _ = DeleteObject(pen);
        }
        if is_default {
            black_box(hdc, inset(bounds, 2));
        }
    }
    let mut label = text.to_string();
    if let Some(i) = label.find('&') {
        label.remove(i);
    }
    let mut tr = inset(bounds, 2);
    if press {
        tr.left += 1;
        tr.top += 1;
    }
    let color = if !enabled {
        BLACK
    } else if press {
        WHITE
    } else {
        BLACK
    };
    let mut w: Vec<u16> = label.encode_utf16().collect();
    unsafe {
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, rgb3(color));
        let _ = DrawTextW(
            hdc,
            &mut w,
            &mut tr,
            windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
                DT_CENTER.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0 | DT_END_ELLIPSIS.0,
            ),
        );
    }
    if focus && enabled && !press {
        black_box(hdc, inset(bounds, 3));
    }
}

const CTL_CHECKED: i32 = 0;
const CTL_PRESS: i32 = 8;
const CTL_FONT: i32 = 16;

unsafe extern "system" fn check_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    toggle_proc(hwnd, msg, wparam, lparam, false)
}

unsafe extern "system" fn radio_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    toggle_proc(hwnd, msg, wparam, lparam, true)
}

unsafe fn toggle_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, radio: bool) -> LRESULT {
    match msg {
        WM_CREATE => {
            set_long(hwnd, CTL_CHECKED, 0);
            set_long(hwnd, CTL_PRESS, 0);
            set_long(hwnd, CTL_FONT, 0);
            LRESULT(0)
        }
        WM_SETFONT => {
            set_long(hwnd, CTL_FONT, wparam.0 as isize);
            if lparam.0 != 0 {
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            paint_toggle(hwnd, radio);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_ENABLE => {
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        0x00F0 => LRESULT(get_long(hwnd, CTL_CHECKED)), // BM_GETCHECK
        0x00F1 => {
            set_long(hwnd, CTL_CHECKED, if wparam.0 != 0 { 1 } else { 0 });
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_KEYDOWN => {
            if msg == WM_KEYDOWN && wparam.0 != 0x20 && wparam.0 != 0x0D {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            if radio {
                uncheck_radios(hwnd);
                set_long(hwnd, CTL_CHECKED, 1);
            } else {
                set_long(hwnd, CTL_CHECKED, 1 - get_long(hwnd, CTL_CHECKED));
            }
            let _ = SetFocus(hwnd);
            let _ = InvalidateRect(hwnd, None, false);
            let parent = GetParent(hwnd).unwrap_or(HWND(std::ptr::null_mut()));
            let id = GetDlgCtrlID(hwnd);
            let _ = SendMessageW(parent, WM_COMMAND, WPARAM(id as usize), LPARAM(hwnd.0 as isize));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn uncheck_radios(hwnd: HWND) {
    let parent = GetParent(hwnd).unwrap_or(HWND(std::ptr::null_mut()));
    let mut child = GetWindow(parent, GW_CHILD).unwrap_or(HWND(std::ptr::null_mut()));
    while !child.0.is_null() {
        if child != hwnd {
            let mut name = [0u16; 32];
            let n = GetClassNameW(child, &mut name);
            let class = String::from_utf16_lossy(&name[..n as usize]);
            if class == "OpusWin2Radio" {
                set_long(child, CTL_CHECKED, 0);
                let _ = InvalidateRect(child, None, false);
            }
        }
        child = GetWindow(child, GW_HWNDNEXT).unwrap_or(HWND(std::ptr::null_mut()));
    }
}

fn paint_toggle(hwnd: HWND, radio: bool) {
    unsafe {
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);
        fill3(hdc, client, WHITE);
        let enabled = IsWindowEnabled(hwnd).as_bool();
        let checked = get_long(hwnd, CTL_CHECKED) != 0;
        let box_s = 16;
        let by = client.top + ((client.bottom - client.top) - box_s).max(0) / 2;
        let boxr = RECT {
            left: client.left,
            top: by,
            right: client.left + box_s,
            bottom: by + box_s,
        };
        if radio {
            let pen = CreatePen(PS_SOLID, 2, rgb3(BLACK));
            let old_pen = SelectObject(hdc, HPEN(pen.0));
            let empty = GetStockObject(NULL_BRUSH);
            let old_brush = SelectObject(hdc, empty);
            let _ = Ellipse(hdc, boxr.left, boxr.top, boxr.right, boxr.bottom);
            SelectObject(hdc, old_brush);
            SelectObject(hdc, old_pen);
            let _ = DeleteObject(pen);
            if checked {
                fill(
                    hdc,
                    RECT {
                        left: boxr.left + 4,
                        top: boxr.top + 4,
                        right: boxr.right - 4,
                        bottom: boxr.bottom - 4,
                    },
                    rgb3(BLACK),
                );
                let br = CreateSolidBrush(rgb3(BLACK));
                let old_b = SelectObject(hdc, br);
                let pen2 = CreatePen(PS_SOLID, 1, rgb3(BLACK));
                let old_p = SelectObject(hdc, HPEN(pen2.0));
                let _ = Ellipse(hdc, boxr.left + 3, boxr.top + 3, boxr.right - 3, boxr.bottom - 3);
                SelectObject(hdc, old_p);
                SelectObject(hdc, old_b);
                let _ = DeleteObject(pen2);
                let _ = DeleteObject(br);
            }
        } else {
            fill3(hdc, boxr, WHITE);
            black_box(hdc, boxr);
            if checked {
                // X, two pixels thick.
                for i in 2..box_s - 2 {
                    fill(hdc, RECT { left: boxr.left + i, top: boxr.top + i, right: boxr.left + i + 2, bottom: boxr.top + i + 2 }, rgb3(BLACK));
                    fill(hdc, RECT { left: boxr.right - 2 - i, top: boxr.top + i, right: boxr.right - i, bottom: boxr.top + i + 2 }, rgb3(BLACK));
                }
            }
        }
        let mut label = button_text(hwnd);
        if let Some(i) = label.find('&') {
            label.remove(i);
        }
        let mut tr = RECT {
            left: boxr.right + 6,
            top: client.top,
            right: client.right,
            bottom: client.bottom,
        };
        let font = HFONT(get_long(hwnd, CTL_FONT) as *mut _);
        let color = if enabled { BLACK } else { (128, 128, 128) };
        if !font.0.is_null() {
            text_left(hdc, font, tr, &label, color);
        } else {
            let mut w: Vec<u16> = label.encode_utf16().collect();
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, rgb3(color));
            let _ = DrawTextW(
                hdc,
                &mut w,
                &mut tr,
                windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
                    DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_NOPREFIX.0 | DT_END_ELLIPSIS.0,
                ),
            );
        }
        let _ = EndPaint(hwnd, &ps);
    }
}

fn get_long(hwnd: HWND, index: i32) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(index)) }
}

fn set_long(hwnd: HWND, index: i32, value: isize) {
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(index), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_ends_stay_put() {
        let a = rgb(0, 0, 0);
        let b = rgb(255, 255, 255);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }
}
