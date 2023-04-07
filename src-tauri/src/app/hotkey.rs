use clipboard::{ClipboardContext, ClipboardProvider};
use once_cell::sync::Lazy;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Manager};
use winapi::shared::minwindef::{LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
  CallNextHookEx, GetAsyncKeyState, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION,
  KBDLLHOOKSTRUCT, VK_CONTROL, WH_KEYBOARD_LL, WM_KEYDOWN,
};
#[derive(Clone)]
pub struct MyHotkey {
  h_hook: Arc<Mutex<HHOOK>>,
}

unsafe impl Send for MyHotkey {}
unsafe impl Sync for MyHotkey {}

pub struct PressInfo {
  ctrl_c_pressed: Lazy<Arc<AtomicBool>>,
  last_press_time: Lazy<Arc<Mutex<Instant>>>,
  pub hot_key: Lazy<Arc<Mutex<MyHotkey>>>,
  // ctrl_c_pressed_clone: Arc<AtomicBool>,
  // last_press_time_clone: Arc<Mutex<Instant>>,
}

static mut WD: Mutex<Option<AppHandle>> = Mutex::new(None);

fn handle_copy() {
  unsafe {
    if PRESS.ctrl_c_pressed.load(Ordering::SeqCst) {
      let mut last_ctrl_c_time_locked = PRESS.last_press_time.lock().unwrap();
      let elapsed = last_ctrl_c_time_locked.elapsed();
      if elapsed.as_secs() < 1 {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let mut content = ctx.get_contents().unwrap();
        let app = WD.lock().unwrap();
        let w = app.as_ref().unwrap().app_handle().get_window("core");

        if !content.is_empty() && content.trim() != "" {
          content = content.replace("'", "\\'");
          let script = format!(
            "
                t = document.querySelector(\"textarea\");
                t.value='{}';
              size=document.getElementsByClassName('btn').length;
              if(size===0||size===5) {{
                t.nextSibling.disabled=false;
                t.nextSibling.click();
              }}",
            content
          );
          w.unwrap().eval(&script).unwrap();
        }
      } else {
        *last_ctrl_c_time_locked = Instant::now();
      }
    } else {
      PRESS.ctrl_c_pressed.store(true, Ordering::SeqCst);
      let mut last_press_time_locked = PRESS.last_press_time.lock().unwrap();
      *last_press_time_locked = Instant::now();
    }
  }
}

unsafe extern "system" fn keyboard_hook_callback(
  n_code: i32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  if n_code == HC_ACTION && w_param as u32 == WM_KEYDOWN {
    let kbhook: *const KBDLLHOOKSTRUCT = l_param as *const _;
    let kbhook = *kbhook;

    if kbhook.vkCode as i32 == 'C' as i32 && GetAsyncKeyState(VK_CONTROL) < 0 {
      handle_copy();
      return 0;
    }
  }
  CallNextHookEx(null_mut(), n_code, w_param, l_param)
}

impl MyHotkey {
  pub fn new() -> Self {
    MyHotkey {
      h_hook: Arc::new(Mutex::new(null_mut())),
    }
  }
  pub fn reg_hotkey(mut self, handle: AppHandle) {
    unsafe {
      WD = Mutex::new(Some(handle));
      let h_mod = GetModuleHandleW(null_mut());
      self.h_hook = Arc::new(Mutex::new(SetWindowsHookExW(
        WH_KEYBOARD_LL,
        Some(keyboard_hook_callback),
        h_mod,
        0,
      )));
    }
  }
  pub fn unreg_hotkey(&self) {
    let h = self.h_hook.lock().unwrap();
    unsafe {
      UnhookWindowsHookEx(h.as_mut().unwrap());
    };
  }
}

// impl Drop for MyHotkey {
//   fn drop(&mut self) {
//     self.unreg_hotkey();
//   }
// }
pub static mut PRESS: PressInfo = PressInfo {
  ctrl_c_pressed: Lazy::new(|| Arc::new(AtomicBool::new(false))),
  last_press_time: Lazy::new(|| Arc::new(Mutex::new(Instant::now()))),
  hot_key: Lazy::new(|| Arc::new(Mutex::new(MyHotkey::new()))),
};
