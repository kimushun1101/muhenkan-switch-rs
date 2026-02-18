//! Lightweight Win32 toast notification for Explorer file operations.
//!
//! Shows an immediate "processing" message, then updates with the result
//! and auto-dismisses after 1.5 seconds.

#[cfg(target_os = "windows")]
mod imp {
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};

    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
        GetStockObject, InvalidateRect, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT,
        DT_CENTER, DT_SINGLELINE, DT_VCENTER, PAINTSTRUCT, TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    const POLL_TIMER_ID: usize = 1;
    const DISMISS_TIMER_ID: usize = 2;
    const TOAST_WIDTH: i32 = 350;
    const TOAST_HEIGHT: i32 = 36;

    struct ToastData {
        rx: mpsc::Receiver<String>,
        message: Vec<u16>,
    }

    pub struct Toast {
        tx: mpsc::Sender<String>,
        handle: JoinHandle<()>,
    }

    impl Toast {
        /// Show the toast window immediately with an initial message.
        pub fn show(initial_message: &str) -> Self {
            let (tx, rx) = mpsc::channel::<String>();
            let init_msg: Vec<u16> = initial_message.encode_utf16().chain(std::iter::once(0)).collect();

            let handle = thread::spawn(move || {
                run_toast_ui(rx, init_msg);
            });

            Toast { tx, handle }
        }

        /// Update the toast message with the final result, then wait for dismiss.
        pub fn finish(self, message: &str) {
            let _ = self.tx.send(message.to_string());
            let _ = self.handle.join();
        }
    }

    fn run_toast_ui(rx: mpsc::Receiver<String>, initial_message: Vec<u16>) {
        unsafe {
            let class_name = w!("MuhenkanToast");

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wndproc),
                hInstance: GetModuleHandleW(None).unwrap_or_default().into(),
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };

            RegisterClassExW(&wc);

            // Position near cursor
            let mut pt = windows::Win32::Foundation::POINT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
            let x = pt.x + 16;
            let y = pt.y - TOAST_HEIGHT - 8;

            let data = Box::new(ToastData {
                rx,
                message: initial_message,
            });
            let data_ptr = Box::into_raw(data);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name,
                w!(""),
                WS_POPUP,
                x,
                y,
                TOAST_WIDTH,
                TOAST_HEIGHT,
                None,
                None,
                None,
                Some(data_ptr as *const std::ffi::c_void),
            )
            .unwrap_or_default();

            if hwnd.0.is_null() {
                // Clean up if window creation failed
                let _ = Box::from_raw(data_ptr);
                return;
            }

            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnregisterClassW(class_name, None);
        }
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => {
                let cs = &*(lparam.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
                let _ = SetTimer(Some(hwnd), POLL_TIMER_ID, 100, None);
                LRESULT(0)
            }

            WM_TIMER => {
                let timer_id = wparam.0;
                if timer_id == POLL_TIMER_ID {
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToastData;
                    if !ptr.is_null() {
                        if let Ok(new_msg) = (*ptr).rx.try_recv() {
                            (*ptr).message = new_msg
                                .encode_utf16()
                                .chain(std::iter::once(0))
                                .collect();
                            let _ = InvalidateRect(Some(hwnd), None, true);
                            let _ = KillTimer(Some(hwnd), POLL_TIMER_ID);
                            let _ = SetTimer(Some(hwnd), DISMISS_TIMER_ID, 1500, None);
                        }
                    }
                } else if timer_id == DISMISS_TIMER_ID {
                    let _ = KillTimer(Some(hwnd), DISMISS_TIMER_ID);
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }

            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);

                // Dark background
                let bg_brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00333333));
                let _ = FillRect(hdc, &rc, bg_brush);

                // White text with default GUI font
                let font = GetStockObject(DEFAULT_GUI_FONT);
                let old_font = SelectObject(hdc, font);
                SetTextColor(hdc, windows::Win32::Foundation::COLORREF(0x00FFFFFF));
                SetBkMode(hdc, TRANSPARENT);

                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToastData;
                if !ptr.is_null() {
                    let msg_slice = &(*ptr).message;
                    // Find null terminator for text length
                    let text_len = msg_slice.iter().position(|&c| c == 0).unwrap_or(msg_slice.len());
                    let text = PCWSTR::from_raw(msg_slice.as_ptr());
                    let _ = DrawTextW(
                        hdc,
                        &mut Vec::from(&msg_slice[..text_len]),
                        &mut rc,
                        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
                    );
                    let _ = text; // keep text alive
                }

                SelectObject(hdc, old_font);
                let _ = DeleteObject(bg_brush.into());
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }

            WM_DESTROY => {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToastData;
                if !ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    let _ = Box::from_raw(ptr);
                }
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::process::Command;

    pub struct Toast;

    impl Toast {
        pub fn show(initial_message: &str) -> Self {
            let _ = Command::new("notify-send")
                .args([
                    "--app-name=muhenkan-switch",
                    "muhenkan-switch",
                    initial_message,
                ])
                .spawn();
            Toast
        }

        pub fn finish(self, message: &str) {
            let _ = Command::new("notify-send")
                .args(["--app-name=muhenkan-switch", "muhenkan-switch", message])
                .spawn();
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    pub struct Toast;

    impl Toast {
        pub fn show(_initial_message: &str) -> Self {
            Toast
        }

        pub fn finish(self, _message: &str) {}
    }
}

pub use imp::Toast;

#[cfg(test)]
mod tests {
    use super::Toast;

    #[test]
    fn toast_show_and_finish_does_not_panic() {
        // notify-send がなくてもパニックしないことを確認
        let toast = Toast::show("test message");
        toast.finish("done");
    }

    #[test]
    fn toast_show_with_empty_message() {
        let toast = Toast::show("");
        toast.finish("");
    }

    #[test]
    fn toast_show_with_japanese_message() {
        let toast = Toast::show("処理中...");
        toast.finish("完了しました");
    }
}
