use crate::{app::window, conf::AppConf, utils};
use clipboard::{ClipboardContext, ClipboardProvider};
use log::{error, info};
use rdev::{listen, EventType, Key, KeyboardState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{utils::config::WindowUrl, App, GlobalShortcutManager, Manager, WindowBuilder};
use wry::application::accelerator::Accelerator;

pub fn init(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
  info!("stepup");
  let app_conf = AppConf::read();
  let url = app_conf.main_origin.to_string();
  let theme = AppConf::theme_mode();
  let handle = app.app_handle();

  tauri::async_runtime::spawn(async move {
    info!("stepup_tray");
    window::tray_window(&handle);
  });

  if let Some(v) = app_conf.clone().global_shortcut {
    info!("global_shortcut: `{}`", v);
    match v.parse::<Accelerator>() {
      Ok(_) => {
        info!("global_shortcut_register");
        let handle = app.app_handle();
        let mut shortcut = app.global_shortcut_manager();
        shortcut
          .register(&v, move || {
            if let Some(w) = handle.get_window("core") {
              if w.is_visible().unwrap() {
                w.hide().unwrap();
              } else {
                w.show().unwrap();
                w.set_focus().unwrap();
              }
            }
          })
          .unwrap_or_else(|err| {
            error!("global_shortcut_register_error: {}", err);
          });
      }
      Err(err) => {
        error!("global_shortcut_parse_error: {}", err);
      }
    }
  } else {
    info!("global_shortcut_unregister");
  };
  //register_shortcut(app);
  // reg_hotkey(app);
  let app_conf2 = app_conf.clone();
  if app_conf.hide_dock_icon {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
  } else {
    let app = app.handle();
    tauri::async_runtime::spawn(async move {
      let link = if app_conf2.main_dashboard {
        "index.html"
      } else {
        &url
      };
      info!("main_window: {}", link);
      let mut main_win = WindowBuilder::new(&app, "core", WindowUrl::App(link.into()))
        .title("ChatGPT")
        .resizable(true)
        .fullscreen(false)
        .inner_size(app_conf2.main_width, app_conf2.main_height)
        .theme(Some(theme))
        .always_on_top(app_conf2.stay_on_top)
        .initialization_script(&utils::user_script())
        .initialization_script(include_str!("../scripts/core.js"))
        .user_agent(&app_conf2.ua_window);

      #[cfg(target_os = "macos")]
      {
        main_win = main_win
          .title_bar_style(app_conf2.clone().titlebar())
          .hidden_title(true);
      }

      if url == "https://chat.openai.com" && !app_conf2.main_dashboard {
        main_win = main_win
          .initialization_script(include_str!("../vendors/floating-ui-core.js"))
          .initialization_script(include_str!("../vendors/floating-ui-dom.js"))
          .initialization_script(include_str!("../vendors/html2canvas.js"))
          .initialization_script(include_str!("../vendors/jspdf.js"))
          .initialization_script(include_str!("../vendors/turndown.js"))
          .initialization_script(include_str!("../vendors/turndown-plugin-gfm.js"))
          .initialization_script(include_str!("../scripts/popup.core.js"))
          .initialization_script(include_str!("../scripts/export.js"))
          .initialization_script(include_str!("../scripts/markdown.export.js"))
          .initialization_script(include_str!("../scripts/cmd.js"))
          .initialization_script(include_str!("../scripts/chat.js"))
      }

      main_win.build().unwrap();
    });
  }

  fn register_shortcut(app: &mut App) {
    let ctrl_c_pressed = Arc::new(AtomicBool::new(false));
    let last_press_time = Arc::new(Mutex::new(Instant::now()));
    let ctrl_c_pressed_clone = ctrl_c_pressed.clone();
    let last_press_time_clone = last_press_time.clone();

    let mut shortcut = app.global_shortcut_manager();
    let handle = app.app_handle();
    shortcut
      .register("Ctrl+C", move || {
        if ctrl_c_pressed_clone.load(Ordering::SeqCst) {
          let mut last_ctrl_c_time_locked = last_press_time_clone.lock().unwrap();
          let elapsed = last_ctrl_c_time_locked.elapsed();
          if elapsed.as_secs() < 1 {
            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
            let mut content = ctx.get_contents().unwrap();
            if !content.is_empty() && content.trim() != "" {
              if let Some(w) = handle.get_window("core") {
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
                info!("script {}", script);
                w.eval(&script).unwrap();
              } else {
                error!("窗口获取失败");
              }
            }
          } else {
            *last_ctrl_c_time_locked = Instant::now();
          }
        } else {
          ctrl_c_pressed_clone.store(true, Ordering::SeqCst);
          let mut last_press_time_locked = last_press_time_clone.lock().unwrap();
          *last_press_time_locked = Instant::now();
        }
      })
      .unwrap_or_else(|err| error!("ctrl+c注册失败:{}", err));
  }

  fn reg_hotkey(app: &mut App) {
    let mut listener = hotkey::Listener::new();
    let ctrl_c_pressed = Arc::new(AtomicBool::new(false));
    let last_press_time = Arc::new(Mutex::new(Instant::now()));
    let ctrl_c_pressed_clone = ctrl_c_pressed.clone();
    let last_press_time_clone = last_press_time.clone();
    let handle = app.app_handle();
    listener
      .register_hotkey(hotkey::modifiers::CONTROL, 'C' as u32, move || {
        if ctrl_c_pressed_clone.load(Ordering::SeqCst) {
          let mut last_ctrl_c_time_locked = last_press_time_clone.lock().unwrap();
          let elapsed = last_ctrl_c_time_locked.elapsed();
          if elapsed.as_secs() < 1 {
            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
            let mut content = ctx.get_contents().unwrap();
            if !content.is_empty() && content.trim() != "" {
              if let Some(w) = handle.get_window("core") {
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
                info!("script {}", script);
                w.eval(&script).unwrap();
              } else {
                error!("窗口获取失败");
              }
            }
          } else {
            *last_ctrl_c_time_locked = Instant::now();
          }
        } else {
          ctrl_c_pressed_clone.store(true, Ordering::SeqCst);
          let mut last_press_time_locked = last_press_time_clone.lock().unwrap();
          *last_press_time_locked = Instant::now();
        }
      })
      .unwrap();
    listener.listen();
  }

  // fn reg_hotkey2(app: &mut App) {
  //   let mut keyboard = KeyboardState::new();
  //   let mut listener = hotkey::Listener::new();
  //   let ctrl_c_pressed = Arc::new(AtomicBool::new(false));
  //   let last_press_time = Arc::new(Mutex::new(Instant::now()));
  //   let ctrl_c_pressed_clone = ctrl_c_pressed.clone();
  //   let last_press_time_clone = last_press_time.clone();
  //   let handle = app.app_handle();
  //   // 监听键盘事件
  //   listen(move |event| {
  //     if let EventType::KeyPress(key_event) = event.event_type {
  //       // 更新键盘状态
  //       KeyboardState::add(&key_event);
  //       // 检查是否按下了Ctrl+C
  //       if keyboard.matches_sequence(&[Key::ControlLeft, Key::KeyC]) {
  //         if ctrl_c_pressed_clone.load(Ordering::SeqCst) {
  //           let mut last_ctrl_c_time_locked = last_press_time_clone.lock().unwrap();
  //           let elapsed = last_ctrl_c_time_locked.elapsed();
  //           if elapsed.as_secs() < 1 {
  //             let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
  //             let mut content = ctx.get_contents().unwrap();
  //             if !content.is_empty() && content.trim() != "" {
  //               if let Some(w) = handle.get_window("core") {
  //                 content = content.replace("'", "\\'");
  //                 let script = format!(
  //                   "
  //                       t = document.querySelector(\"textarea\");
  //                       t.value='{}';
  //                     size=document.getElementsByClassName('btn').length;
  //                     if(size===0||size===5) {{
  //                       t.nextSibling.disabled=false;
  //                       t.nextSibling.click();
  //                     }}",
  //                   content
  //                 );
  //                 info!("script {}", script);
  //                 w.eval(&script).unwrap();
  //               } else {
  //                 error!("窗口获取失败");
  //               }
  //             }
  //           } else {
  //             *last_ctrl_c_time_locked = Instant::now();
  //           }
  //         } else {
  //           ctrl_c_pressed_clone.store(true, Ordering::SeqCst);
  //           let mut last_press_time_locked = last_press_time_clone.lock().unwrap();
  //           *last_press_time_locked = Instant::now();
  //         }
  //       }
  //     } else if let EventType::KeyRelease(key_event) = event.event_type {
  //       // 更新键盘状态
  //       keyboard.remove_key(&key_event);
  //     }
  //   })
  //   .unwrap();
  // }
  // auto_update
  let auto_update = app_conf.get_auto_update();
  if auto_update != "disable" {
    info!("run_check_update");
    let app = app.handle();
    utils::run_check_update(app, auto_update == "silent", None);
  }

  Ok(())
}
