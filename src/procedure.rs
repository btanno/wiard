use crate::*;
use std::any::Any;
use std::cell::RefCell;
use std::path::PathBuf;
use tokio::sync::oneshot;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::{BeginPaint, EndPaint, GetUpdateRect, ScreenToClient, PAINTSTRUCT},
    UI::Controls::WM_MOUSELEAVE,
    UI::HiDpi::{EnableNonClientDpiScaling, GetDpiForWindow},
    UI::Input::Ime::{ISC_SHOWUIALLCANDIDATEWINDOW, ISC_SHOWUICOMPOSITIONWINDOW},
    UI::Input::KeyboardAndMouse::{
        ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VIRTUAL_KEY,
    },
    UI::Shell::{DragFinish, DragQueryFileW, DragQueryPoint, HDROP},
    UI::WindowsAndMessaging::*,
};

thread_local! {
    static UNWIND: RefCell<Option<Box<dyn Any + Send>>> = RefCell::new(None);
    static ENTERED: RefCell<Option<HWND>> = RefCell::new(None);
}

fn set_unwind(e: Box<dyn Any + Send>) {
    UNWIND.with_borrow_mut(|unwind| {
        *unwind = Some(e);
    });
}

pub(crate) fn get_unwind() -> Option<Box<dyn Any + Send>> {
    UNWIND.with_borrow_mut(|unwind| unwind.take())
}

unsafe fn on_paint(hwnd: HWND) -> LRESULT {
    let mut rc = RECT::default();
    let _ = GetUpdateRect(hwnd, Some(&mut rc), false);
    let mut ps = PAINTSTRUCT::default();
    let _hdc = BeginPaint(hwnd, &mut ps);
    let _ = EndPaint(hwnd, &ps);
    let invalidate_rect = PhysicalRect::new(rc.left, rc.top, rc.right, rc.bottom);
    let handle = WindowHandle::new(hwnd);
    Context::set_window_props(handle, |props| props.redrawing = false);
    Context::send_event(handle, Event::Draw(event::Draw { invalidate_rect }));
    LRESULT(0)
}

unsafe fn on_mouse_move(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let entered = ENTERED.with(|entered| *entered.borrow());
    let handle = WindowHandle::new(hwnd);
    if entered.is_none() {
        TrackMouseEvent(&mut TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        })
        .ok();
        ENTERED.with(|entered| *entered.borrow_mut() = Some(hwnd));
        Context::send_event(
            handle,
            event::Event::CursorEntered(event::CursorEntered {
                mouse_state: MouseState::from_params(wparam, lparam),
            }),
        );
    } else {
        Context::send_event(
            handle,
            event::Event::CursorMoved(event::CursorMoved {
                mouse_state: MouseState::from_params(wparam, lparam),
            }),
        );
    }
    LRESULT(0)
}

unsafe fn on_set_cursor(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if loword(lparam.0 as i32) != HTCLIENT as i16 {
        return DefWindowProcW(hwnd, WM_SETCURSOR, wparam, lparam);
    }
    let cursor =
        Context::get_window_props(WindowHandle::new(hwnd), |props| props.cursor.clone()).unwrap();
    cursor.set();
    LRESULT(0)
}

unsafe fn on_mouse_leave(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    ENTERED.with(|entered| {
        *entered.borrow_mut() = None;
    });
    let mut position = POINT::default();
    GetCursorPos(&mut position).ok();
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::CursorLeft(event::CursorLeft {
            mouse_state: MouseState {
                position: position.into(),
                buttons: wparam.into(),
            },
        }),
    );
    LRESULT(0)
}

unsafe fn on_mouse_input(
    hwnd: HWND,
    button: MouseButton,
    button_state: ButtonState,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match button_state {
        ButtonState::Pressed => {
            SetCapture(hwnd);
        }
        ButtonState::Released => {
            ReleaseCapture().ok();
        }
    }
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::MouseInput(event::MouseInput {
            button,
            button_state,
            mouse_state: MouseState::from_params(wparam, lparam),
        }),
    );
    LRESULT(0)
}

unsafe fn on_mouse_wheel(
    hwnd: HWND,
    axis: MouseWheelAxis,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let delta = hiword(wparam.0 as i32);
    let mouse_state = MouseState::from_params(wparam, lparam);
    let mut pt = POINT {
        x: mouse_state.position.x,
        y: mouse_state.position.y,
    };
    let _ = ScreenToClient(hwnd, &mut pt);
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::MouseWheel(event::MouseWheel {
            axis,
            distance: delta as i32,
            mouse_state: MouseState {
                position: PhysicalPosition::new(pt.x, pt.y),
                buttons: mouse_state.buttons,
            },
        }),
    );
    LRESULT(0)
}

unsafe fn on_key_input(hwnd: HWND, key_state: KeyState, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let vkey = VirtualKey::from(VIRTUAL_KEY(wparam.0 as u16));
    let scan_code = ScanCode(((lparam.0 >> 16) & 0x7f) as u32);
    let prev_pressed = (lparam.0 >> 30) & 0x01 != 0;
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::KeyInput(event::KeyInput {
            key_code: KeyCode::new(vkey, scan_code),
            key_state,
            prev_pressed,
        }),
    );
    LRESULT(0)
}

unsafe fn on_char(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    if let Some(c) = char::from_u32(wparam.0 as u32) {
        Context::send_event(
            WindowHandle::new(hwnd),
            Event::CharInput(event::CharInput { c }),
        );
    }
    LRESULT(0)
}

unsafe fn on_ime_set_context(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let lparam = {
        let mut value = lparam.0 as u32;
        value &= !ISC_SHOWUICOMPOSITIONWINDOW;
        let candidate = Context::get_window_props(WindowHandle::new(hwnd), |props| {
            props.visible_ime_candidate_window
        })
        .unwrap();
        if !candidate {
            value &= !ISC_SHOWUIALLCANDIDATEWINDOW;
        }
        LPARAM(value as isize)
    };
    DefWindowProcW(hwnd, WM_IME_SETCONTEXT, wparam, lparam)
}

unsafe fn on_ime_start_composition(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let dpi = GetDpiForWindow(hwnd) as i32;
    let (tx, rx) = oneshot::channel::<PhysicalPosition<i32>>();
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::ImeBeginComposition(event::ImeBeginComposition::new(dpi, tx)),
    );
    if let Ok(position) = rx.blocking_recv() {
        let imc = ime::Imc::get(hwnd);
        imc.set_candidate_window_position(position, false);
    }
    DefWindowProcW(hwnd, WM_IME_STARTCOMPOSITION, wparam, lparam)
}

unsafe fn on_ime_composition(hwnd: HWND, _wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    let imc = ime::Imc::get(hwnd);
    let Some(s) = imc.get_composition_string() else {
        return LRESULT(0);
    };
    let Some(clauses) = imc.get_composition_clauses() else {
        return LRESULT(0);
    };
    let composition = event::ImeUpdateComposition {
        chars: s.chars().collect(),
        clauses,
        cursor_position: imc.get_cursor_position(),
    };
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::ImeUpdateComposition(composition),
    );
    LRESULT(0)
}

unsafe fn on_ime_end_composition(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let imc = ime::Imc::get(hwnd);
    let result = imc.get_composition_result();
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::ImeEndComposition(event::ImeEndComposition { result }),
    );
    DefWindowProcW(hwnd, WM_IME_ENDCOMPOSITION, wparam, lparam)
}

unsafe fn on_sizing(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let d = {
        let window_rc = get_window_rect(hwnd);
        let client_rc = get_client_rect(hwnd);
        PhysicalSize::new(
            (window_rc.right - window_rc.left) - (client_rc.right - client_rc.left),
            (window_rc.bottom - window_rc.top) - (client_rc.bottom - client_rc.top),
        )
    };
    let rc = (lparam.0 as *mut RECT).as_mut().unwrap();
    let size = PhysicalSize::new(
        (rc.right - rc.left - d.width) as u32,
        (rc.bottom - rc.top - d.height) as u32,
    );
    let edge = match wparam.0 as u32 {
        WMSZ_LEFT => ResizingEdge::Left,
        WMSZ_RIGHT => ResizingEdge::Right,
        WMSZ_TOP => ResizingEdge::Top,
        WMSZ_BOTTOM => ResizingEdge::Bottom,
        WMSZ_TOPLEFT => ResizingEdge::TopLeft,
        WMSZ_TOPRIGHT => ResizingEdge::TopRight,
        WMSZ_BOTTOMLEFT => ResizingEdge::BottomLeft,
        WMSZ_BOTTOMRIGHT => ResizingEdge::BottomRight,
        _ => unreachable!(),
    };
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::Resizing(event::Resizing { size, edge }),
    );
    DefWindowProcW(hwnd, WM_SIZING, wparam, lparam)
}

unsafe fn on_size(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handle = WindowHandle::new(hwnd);
    match wparam.0 as u32 {
        SIZE_MINIMIZED => {
            Context::set_window_props(handle, |props| props.minimized = true);
            Context::send_event(handle, Event::Minizmized);
        }
        SIZE_MAXIMIZED => {
            let size = lparam_to_size(lparam);
            Context::send_event(handle, Event::Maximized(event::Maximized { size }));
        }
        SIZE_RESTORED => {
            let (resizing, minimized) =
                Context::get_window_props(handle, |props| (props.resizing, props.minimized))
                    .unwrap_or((false, false));
            let size = lparam_to_size(lparam);
            if minimized {
                Context::send_event(handle, Event::Restored(event::Restored { size }));
                Context::set_window_props(handle, |props| props.minimized = false);
            } else if !resizing {
                Context::send_event(handle, Event::Resized(event::Resized { size }));
            }
        }
        _ => {}
    }
    LRESULT(0)
}

unsafe fn on_window_pos_changed(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let pos = (lparam.0 as *const WINDOWPOS).as_ref().unwrap();
    if pos.flags.0 & SWP_NOMOVE.0 == 0 {
        Context::send_event(
            WindowHandle::new(hwnd),
            Event::Moved(event::Moved {
                position: ScreenPosition::new(pos.x, pos.y),
            }),
        );
    }
    DefWindowProcW(hwnd, WM_WINDOWPOSCHANGED, wparam, lparam)
}

unsafe fn on_enter_size_move(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handle = WindowHandle::new(hwnd);
    Context::set_window_props(handle, |props| props.resizing = true);
    Context::send_event(handle, Event::EnterResizing);
    DefWindowProcW(hwnd, WM_ENTERSIZEMOVE, wparam, lparam)
}

unsafe fn on_exit_size_move(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handle = WindowHandle::new(hwnd);
    let size = get_client_rect(hwnd);
    Context::set_window_props(handle, |props| props.resizing = false);
    Context::send_event(
        handle,
        Event::Resized(event::Resized {
            size: Size::new(
                (size.right - size.left) as u32,
                (size.bottom - size.top) as u32,
            ),
        }),
    );
    DefWindowProcW(hwnd, WM_EXITSIZEMOVE, wparam, lparam)
}

unsafe fn on_activate(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    let active = wparam.0 as u32 & (WA_ACTIVE | WA_CLICKACTIVE) != 0;
    let handle = WindowHandle::new(hwnd);
    if active {
        Context::send_event(handle, Event::Activated);
    } else {
        Context::send_event(handle, Event::Inactivate);
    }
    LRESULT(0)
}

unsafe fn on_dpi_changed(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let rc = *(lparam.0 as *const RECT);
    SetWindowPos(
        hwnd,
        HWND::default(),
        rc.left,
        rc.top,
        rc.right - rc.left,
        rc.bottom - rc.top,
        SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .ok();
    let new_dpi = hiword(wparam.0 as i32) as u32;
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::DpiChanged(event::DpiChanged { new_dpi }),
    );
    LRESULT(0)
}

unsafe fn on_get_dpi_scaled_size(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let prev_dpi = GetDpiForWindow(hwnd) as i32;
    let next_dpi = wparam.0 as i32;
    let rc = get_client_rect(hwnd);
    let size = PhysicalSize::new(
        ((rc.right - rc.left) * next_dpi / prev_dpi) as u32,
        ((rc.bottom - rc.top) * next_dpi / prev_dpi) as u32,
    );
    let rc = adjust_window_rect_ex_for_dpi(
        size,
        WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32),
        false,
        WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32),
        next_dpi as u32,
    );
    let ret = (lparam.0 as *mut SIZE).as_mut().unwrap();
    ret.cx = rc.right - rc.left;
    ret.cy = rc.bottom - rc.top;
    LRESULT(1)
}

unsafe fn on_drop_files(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    let hdrop = HDROP(wparam.0 as *mut _);
    let file_count = DragQueryFileW(hdrop, u32::MAX, None);
    let mut paths = Vec::with_capacity(file_count as usize);
    let mut buffer = Vec::new();
    for i in 0..file_count {
        let len = DragQueryFileW(hdrop, i, None) as usize + 1;
        buffer.resize(len, 0);
        DragQueryFileW(hdrop, i, Some(&mut buffer));
        buffer.pop();
        let path: PathBuf = String::from_utf16_lossy(&buffer).into();
        paths.push(path);
    }
    let mut position = POINT::default();
    let _ = DragQueryPoint(hdrop, &mut position);
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::DropFiles(event::DropFiles {
            paths,
            position: position.into(),
        }),
    );
    DragFinish(hdrop);
    LRESULT(0)
}

unsafe fn on_nc_create(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    EnableNonClientDpiScaling(hwnd).ok();
    DefWindowProcW(hwnd, WM_NCCREATE, wparam, lparam)
}

unsafe fn on_nc_hittest(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let hook =
        Context::get_window_props(WindowHandle::new(hwnd), |props| props.nc_hittest).unwrap();
    if !hook {
        return DefWindowProcW(hwnd, WM_NCHITTEST, wparam, lparam);
    }
    let (tx, rx) = oneshot::channel();
    Context::send_event(
        WindowHandle::new(hwnd),
        Event::NcHitTest(event::NcHitTest::new(lparam, tx)),
    );
    match rx.blocking_recv() {
        Ok(Some(value)) => LRESULT(value as isize),
        _ => DefWindowProcW(hwnd, WM_NCHITTEST, wparam, lparam),
    }
}

unsafe fn on_close(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handle = WindowHandle::new(hwnd);
    let auto_close = Context::get_window_props(handle, |props| props.auto_close).unwrap();
    if auto_close {
        return DefWindowProcW(hwnd, WM_CLOSE, wparam, lparam);
    }
    Context::send_event(
        handle,
        Event::CloseRequest(event::CloseRequest::new(handle)),
    );
    LRESULT(0)
}

unsafe fn on_destroy(hwnd: HWND) -> LRESULT {
    let handle = WindowHandle::new(hwnd);
    Context::send_event(handle, Event::Closed);
    Context::remove_window(handle);
    if Context::is_empty() {
        PostQuitMessage(0);
    }
    LRESULT(0)
}

fn wparam_to_button(wparam: WPARAM) -> MouseButton {
    match get_xbutton_wparam(wparam) {
        0x0001 => MouseButton::Ex(0),
        0x0002 => MouseButton::Ex(1),
        _ => unreachable!(),
    }
}

pub(crate) extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ret = std::panic::catch_unwind(|| unsafe {
        match msg {
            WM_PAINT => on_paint(hwnd),
            WM_MOUSEMOVE => on_mouse_move(hwnd, wparam, lparam),
            WM_SETCURSOR => on_set_cursor(hwnd, wparam, lparam),
            WM_MOUSELEAVE => on_mouse_leave(hwnd, wparam, lparam),
            WM_LBUTTONDOWN => on_mouse_input(
                hwnd,
                MouseButton::Left,
                ButtonState::Pressed,
                wparam,
                lparam,
            ),
            WM_RBUTTONDOWN => on_mouse_input(
                hwnd,
                MouseButton::Right,
                ButtonState::Pressed,
                wparam,
                lparam,
            ),
            WM_MBUTTONDOWN => on_mouse_input(
                hwnd,
                MouseButton::Middle,
                ButtonState::Pressed,
                wparam,
                lparam,
            ),
            WM_XBUTTONDOWN => on_mouse_input(
                hwnd,
                wparam_to_button(wparam),
                ButtonState::Pressed,
                wparam,
                lparam,
            ),
            WM_LBUTTONUP => on_mouse_input(
                hwnd,
                MouseButton::Left,
                ButtonState::Released,
                wparam,
                lparam,
            ),
            WM_RBUTTONUP => on_mouse_input(
                hwnd,
                MouseButton::Right,
                ButtonState::Released,
                wparam,
                lparam,
            ),
            WM_MBUTTONUP => on_mouse_input(
                hwnd,
                MouseButton::Middle,
                ButtonState::Released,
                wparam,
                lparam,
            ),
            WM_XBUTTONUP => on_mouse_input(
                hwnd,
                wparam_to_button(wparam),
                ButtonState::Released,
                wparam,
                lparam,
            ),
            WM_MOUSEWHEEL => on_mouse_wheel(hwnd, MouseWheelAxis::Vertical, wparam, lparam),
            WM_MOUSEHWHEEL => on_mouse_wheel(hwnd, MouseWheelAxis::Horizontal, wparam, lparam),
            WM_KEYDOWN => on_key_input(hwnd, KeyState::Pressed, wparam, lparam),
            WM_KEYUP => on_key_input(hwnd, KeyState::Released, wparam, lparam),
            WM_CHAR => on_char(hwnd, wparam, lparam),
            WM_IME_SETCONTEXT => on_ime_set_context(hwnd, wparam, lparam),
            WM_IME_STARTCOMPOSITION => on_ime_start_composition(hwnd, wparam, lparam),
            WM_IME_COMPOSITION => on_ime_composition(hwnd, wparam, lparam),
            WM_IME_ENDCOMPOSITION => on_ime_end_composition(hwnd, wparam, lparam),
            WM_SIZING => on_sizing(hwnd, wparam, lparam),
            WM_SIZE => on_size(hwnd, wparam, lparam),
            WM_WINDOWPOSCHANGED => on_window_pos_changed(hwnd, wparam, lparam),
            WM_ENTERSIZEMOVE => on_enter_size_move(hwnd, wparam, lparam),
            WM_EXITSIZEMOVE => on_exit_size_move(hwnd, wparam, lparam),
            WM_ACTIVATE => on_activate(hwnd, wparam, lparam),
            WM_DPICHANGED => on_dpi_changed(hwnd, wparam, lparam),
            WM_GETDPISCALEDSIZE => on_get_dpi_scaled_size(hwnd, wparam, lparam),
            WM_DROPFILES => on_drop_files(hwnd, wparam, lparam),
            WM_NCCREATE => on_nc_create(hwnd, wparam, lparam),
            WM_NCHITTEST => on_nc_hittest(hwnd, wparam, lparam),
            WM_CLOSE => on_close(hwnd, wparam, lparam),
            WM_DESTROY => on_destroy(hwnd),
            _ => {
                Context::send_event(
                    WindowHandle::new(hwnd),
                    Event::Other(event::Other {
                        msg,
                        wparam: wparam.0,
                        lparam: lparam.0,
                    }),
                );
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    });
    ret.unwrap_or_else(|e| {
        set_unwind(e);
        LRESULT(0)
    })
}
