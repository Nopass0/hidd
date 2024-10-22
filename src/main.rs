use std::fs;
use std::process::Command;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder},
    dpi::LogicalSize,
};
use wry::WebViewBuilder;

#[cfg(target_os = "windows")]
mod windows_specific {
    use windows::Win32::Foundation::{LRESULT, WPARAM, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowsHookExW, CallNextHookEx, UnhookWindowsHookEx,
        WH_MOUSE_LL, WH_KEYBOARD_LL, KBDLLHOOKSTRUCT,
        WM_RBUTTONDOWN, WM_KEYDOWN, HHOOK
    };

    const VK_F12: u32 = 0x7B;

    static mut HOOK_MOUSE: HHOOK = HHOOK(0);
    static mut HOOK_KEYBOARD: HHOOK = HHOOK(0);

    pub unsafe fn setup_hooks() -> Result<(), Box<dyn std::error::Error>> {
        HOOK_MOUSE = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)?;
        HOOK_KEYBOARD = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)?;

        if HOOK_MOUSE.0 == 0 || HOOK_KEYBOARD.0 == 0 {
            return Err("Failed to set hooks.".into());
        }
        Ok(())
    }

    pub unsafe fn cleanup_hooks() {
        UnhookWindowsHookEx(HOOK_MOUSE);
        UnhookWindowsHookEx(HOOK_KEYBOARD);
    }

    unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && wparam.0 == WM_RBUTTONDOWN as usize {
            println!("Right mouse button blocked.");
            return LRESULT(1);
        }
        CallNextHookEx(HOOK_MOUSE, code, wparam, lparam)
    }

    unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let kb_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if wparam.0 == WM_KEYDOWN as usize && kb_struct.vkCode == VK_F12 {
                println!("F12 key blocked.");
                return LRESULT(1);
            }
        }
        CallNextHookEx(HOOK_KEYBOARD, code, wparam, lparam)
    }
}

#[cfg(target_os = "macos")]
mod macos_specific {
    use objc::{class, msg_send, sel, sel_impl};
    use objc_foundation::NSObject;
    use cocoa::base::id;
    use cocoa::appkit::{NSEvent, NSEventMask};

    pub fn setup_event_monitor() -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let notification_center: id = msg_send![workspace, notificationCenter];
            
            // Monitor global events
            let event_mask = NSEventMask::NSKeyDownMask as usize | 
                           NSEventMask::NSRightMouseDownMask as usize;
            
            let _monitor: id = msg_send![class!(NSEvent),
                addGlobalMonitorForEventsMatchingMask:event_mask 
                handler:^(event: id) {
                    let event_type = NSEvent::eventType(event);
                    if event_type == NSEventType::NSRightMouseDown {
                        println!("Right mouse button blocked on macOS");
                        return;
                    }
                    
                    if event_type == NSEventType::NSKeyDown {
                        let characters: id = msg_send![event, charactersIgnoringModifiers];
                        let key_code: u16 = msg_send![event, keyCode];
                        if key_code == 0x7B { // F12 key
                            println!("F12 key blocked on macOS");
                            return;
                        }
                    }
                }
            ];
        }
        Ok(())
    }
}

fn cleanup_webview() -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let exe_path = std::env::current_exe()?;
        let folder_name = format!("{}.WebView2", exe_path.file_stem().unwrap().to_string_lossy());
        let folder_path = exe_path.parent().unwrap().join(folder_name);

        if folder_path.exists() {
            fs::remove_dir_all(&folder_path)?;
            println!("WebView2 folder removed successfully.");
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS WebKit cache cleanup if needed
        let home = std::env::var("HOME").unwrap_or_default();
        let cache_path = format!("{}/Library/Caches/com.yourapp.webview", home);
        if std::path::Path::new(&cache_path).exists() {
            fs::remove_dir_all(cache_path)?;
            println!("WebKit cache removed successfully.");
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .with_resizable(false)
        .build(&event_loop)?;

    #[cfg(target_os = "windows")]
    unsafe {
        windows_specific::setup_hooks()?;
    }

    #[cfg(target_os = "macos")]
    macos_specific::setup_event_monitor()?;

    let _webview = WebViewBuilder::new()
        .with_url("https://ya.ru")
        .with_initialization_script(
            "document.addEventListener('contextmenu', event => event.preventDefault());
             window.addEventListener('keydown', function(e) {
                 if (e.keyCode === 123 || // F12
                     (e.ctrlKey && e.shiftKey && e.keyCode === 73) || // Ctrl+Shift+I
                     (e.ctrlKey && e.shiftKey && e.keyCode === 67) || // Ctrl+Shift+C
                     (e.ctrlKey && e.shiftKey && e.keyCode === 74) || // Ctrl+Shift+J
                     (e.ctrlKey && e.keyCode === 85) || // Ctrl+U
                     (e.ctrlKey && e.keyCode === 83)) { // Ctrl+S
                     e.preventDefault();
                     return false;
                 }
             }, true);
             // Hide URL
             if (window.location.href !== 'about:blank') {
                 history.pushState(null, '', 'about:blank');
             }
             // Block console access
             Object.defineProperty(window, 'console', {
                 value: Object.freeze({}),
                 configurable: false,
                 writable: false
             });
             // Block debugger access
             setInterval(function() {
                 debugger;
             }, 100);
             // Prevent text selection
             document.addEventListener('selectstart', function(e) { 
                 e.preventDefault();
             });
             // Prevent drag and drop
             document.addEventListener('dragstart', function(e) { 
                 e.preventDefault();
             });
             // Prevent copying
             document.addEventListener('copy', function(e) { 
                 e.preventDefault();
             });
             // Block saving the page
             document.addEventListener('keydown', function(e) {
                 if (e.ctrlKey && (e.key === 's' || e.key === 'S')) {
                     e.preventDefault();
                 }
             });"
        )
        .with_devtools(false)
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Application started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                #[cfg(target_os = "windows")]
                unsafe {
                    windows_specific::cleanup_hooks();
                }

                if let Err(e) = cleanup_webview() {
                    eprintln!("Error cleaning up WebView: {}", e);
                }

                *control_flow = ControlFlow::Exit;
            },
            _ => (),
        }
    });
}