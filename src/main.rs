use std::fs;
use std::path::Path;
use std::process::Command;

use windows::Win32::Foundation::{LRESULT, WPARAM, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowsHookExW, CallNextHookEx, UnhookWindowsHookEx,
    WH_MOUSE_LL, WH_KEYBOARD_LL, KBDLLHOOKSTRUCT,
    WM_RBUTTONDOWN, WM_KEYDOWN, HHOOK
};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder},
    dpi::LogicalSize,
};
use wry::WebViewBuilder;

const VK_F12: u32 = 0x7B;

static mut HOOK_MOUSE: HHOOK = HHOOK(0);
static mut HOOK_KEYBOARD: HHOOK = HHOOK(0);

// Mouse hook procedure
unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 == WM_RBUTTONDOWN as usize {
        println!("Правая кнопка мыши заблокирована.");
        return LRESULT(1); // Block the right mouse button
    }
    CallNextHookEx(HOOK_MOUSE, code, wparam, lparam)
}

// Keyboard hook procedure
unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if wparam.0 == WM_KEYDOWN as usize && kb_struct.vkCode == VK_F12 {
            println!("Нажатие F12 заблокировано.");
            return LRESULT(1); // Block F12 key
        }
    }
    CallNextHookEx(HOOK_KEYBOARD, code, wparam, lparam)
}

// Function to remove WebView2 folder
fn remove_webview_folder() -> std::io::Result<()> {
    // Get the current path to the executable
    let exe_path = std::env::current_exe()?;

    // Create the path to the folder by adding ".WebView2"
    let folder_name = format!("{}.WebView2", exe_path.file_stem().unwrap().to_string_lossy());
    let folder_path = exe_path.parent().unwrap().join(folder_name); // Form the path to the folder in the same directory

    // Check if the folder exists
    if folder_path.exists() {
        // Try to remove the folder
        fs::remove_dir_all(&folder_path)?;
        println!("Папка {:?} успешно удалена.", folder_path);
    } else {
        println!("Папка {:?} не найдена.", folder_path);
    }

    Ok(())
}

// Function to run cleanup script
fn run_cleanup_script() {
    // Get the current executable path and unwrap the result
    let exe_path = std::env::current_exe().unwrap();

    // Create a string from the executable's file stem
    let exe_name = exe_path.file_stem().unwrap().to_string_lossy();

    // Create the folder name
    let folder_name = format!("{}.exe.WebView2", exe_name);
    let folder_path = exe_path.parent().unwrap().join(&folder_name); // Use &folder_name to avoid ownership issues

    let script_path = std::path::Path::new("remove_webview2.ps1");

    let output = Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass") // Disable the execution policy for the script
        .arg("-File")
        .arg(script_path)
        .arg(&*exe_name) // Dereference Cow here
        .arg(&*folder_path.to_string_lossy()) // Dereference Cow here
        .output()
        .expect("Не удалось выполнить скрипт");

    if !output.status.success() {
        eprintln!(
            "Ошибка выполнения скрипта: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();

    // Create a non-resizable window with a fixed size
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .with_resizable(false)
        .build(&event_loop)?;

    // Build a webview with initialization scripts
    let _webview = WebViewBuilder::new()
        .with_url("https://google.com")
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
        .with_devtools(false) // Disable developer tools
        .build(&window)?;

    unsafe {
        HOOK_MOUSE = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)?;
        HOOK_KEYBOARD = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)?;

        if HOOK_MOUSE.0 == 0 || HOOK_KEYBOARD.0 == 0 {
            return Err("Не удалось установить хуки.".into()); // Failed to set hooks
        }
    }

    // Run the event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                .. 
            } => {
                unsafe {
                    UnhookWindowsHookEx(HOOK_MOUSE);
                    UnhookWindowsHookEx(HOOK_KEYBOARD);
                }

                // Run the cleanup script
                run_cleanup_script();

                *control_flow = ControlFlow::Exit; // Exit the application
            },
            _ => (),
        }
    });
}
