//! Windows system tray Shell (NotifyIcon via tray-icon + tao).

use crate::windows_clipboard::WindowsClipboard;
use clip_ffi::{
    default_relay_ws_url, generate_ephemeral_id, generate_link_key, lifetime_may_auto_start,
    link_key_from_base32, link_key_to_base32, LifetimeSnapshotFfi, Session,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use windows_shell::{
    ArmedStateStore, ArmedStateStoring, ClipboardSyncController, FileCredentialStore,
    LinkKeyStoring, LocalNicknameStoring, NicknameStore, ShellCredentials,
};

#[derive(Debug)]
enum UserEvent {
    Menu(tray_icon::menu::MenuId),
    Tick,
}

pub fn run() -> Result<(), String> {
    let mut credentials = FileCredentialStore::new(FileCredentialStore::default_path());
    let mut armed_store =
        ArmedStateStore::open(ArmedStateStore::default_path()).map_err(|e| e.to_string())?;
    let mut nickname_store =
        NicknameStore::open(NicknameStore::default_path()).map_err(|e| e.to_string())?;

    // Opening the Shell clears Quit opt-out (ADR-0006).
    armed_store.clear_quit_opt_out();

    let clipboard = WindowsClipboard::open()?;
    let controller = Arc::new(Mutex::new(ClipboardSyncController::new(clipboard)));
    let running = Arc::new(AtomicBool::new(true));
    let pending_join: Arc<Mutex<Option<ShellCredentials>>> = Arc::new(Mutex::new(None));

    if let Ok(Some(creds)) = credentials.load() {
        let snapshot = LifetimeSnapshotFfi {
            durable_armed: armed_store.is_armed(),
            elevated_capture_granted: true,
            has_link_key: true,
            quit_opted_out: armed_store.quit_opted_out(),
            requires_elevated_capture: false,
        };
        if lifetime_may_auto_start(snapshot) {
            if let Err(err) = join_session(&controller, &creds, armed_store.is_armed()) {
                eprintln!("sync-clip: restore join soft-fail (Sync Idle): {err}");
                if let Ok(mut pending) = pending_join.lock() {
                    *pending = Some(creds);
                }
            }
        }
    }

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let tick_proxy = proxy.clone();
    let tick_running = Arc::clone(&running);
    thread::spawn(move || {
        while tick_running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(300));
            let _ = tick_proxy.send_event(UserEvent::Tick);
        }
    });

    let menu = Menu::new();
    let status_item = MenuItem::new("Sync Clip", false, None);
    let generate_item = MenuItem::new("Generate Link Key", true, None);
    let join_item = MenuItem::new("Join with clipboard Link Key", true, None);
    let arm_item = MenuItem::new("Toggle Armed / Paused", true, None);
    let nick_item = MenuItem::new("Set Local Nickname from clipboard", true, None);
    let quit_item = MenuItem::new("Quit Sync Clip", true, None);
    menu.append(&status_item).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|e| e.to_string())?;
    menu.append(&generate_item).map_err(|e| e.to_string())?;
    menu.append(&join_item).map_err(|e| e.to_string())?;
    menu.append(&arm_item).map_err(|e| e.to_string())?;
    menu.append(&nick_item).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    let icon = tray_icon_rgba();
    let mut tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Sync Clip")
        .with_icon(icon)
        .build()
        .map_err(|e| e.to_string())?;

    let menu_channel = MenuEvent::receiver();
    let generate_id = generate_item.id().clone();
    let join_id = join_item.id().clone();
    let arm_id = arm_item.id().clone();
    let nick_id = nick_item.id().clone();
    let quit_id = quit_item.id().clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        while let Ok(MenuEvent { id }) = menu_channel.try_recv() {
            let _ = proxy.send_event(UserEvent::Menu(id));
        }

        match event {
            Event::UserEvent(UserEvent::Tick) => {
                // Sync Idle retry: soft-failed join keeps Armed intent and retries.
                if let Ok(mut pending) = pending_join.lock() {
                    if let Some(creds) = pending.clone() {
                        let has_session = controller
                            .lock()
                            .map(|c| c.has_session())
                            .unwrap_or(false);
                        if !has_session {
                            if join_session(&controller, &creds, armed_store.is_armed()).is_ok() {
                                *pending = None;
                            }
                        }
                    }
                }
                if let Ok(mut c) = controller.lock() {
                    c.poll_local_clipboard();
                    c.poll_remote_applied();
                    let tip = status_tooltip(
                        nickname_store.load().as_deref(),
                        c.has_session(),
                        c.is_armed(),
                        c.is_sync_idle() || pending_join.lock().ok().and_then(|p| p.clone()).is_some(),
                    );
                    let _ = tray.set_tooltip(Some(&tip));
                    status_item.set_text(&tip);
                }
            }
            Event::UserEvent(UserEvent::Menu(id)) => {
                if id == quit_id {
                    armed_store.set_quit_opted_out(true);
                    if let Ok(mut c) = controller.lock() {
                        c.detach();
                    }
                    running.store(false, Ordering::SeqCst);
                    *control_flow = ControlFlow::Exit;
                } else if id == generate_id {
                    let key = generate_link_key();
                    let encoded = link_key_to_base32(key.clone());
                    let ephemeral = generate_ephemeral_id();
                    let creds = ShellCredentials {
                        ephemeral_id: ephemeral,
                        link_key: key,
                        relay_ws_url: default_relay_ws_url(),
                    };
                    if let Err(err) = credentials.save(&creds) {
                        eprintln!("sync-clip: save credentials: {err}");
                    } else {
                        set_clipboard_text(&encoded);
                        eprintln!("sync-clip: Link Key generated and copied to clipboard");
                        match join_session(&controller, &creds, armed_store.is_armed()) {
                            Ok(()) => {
                                if let Ok(mut pending) = pending_join.lock() {
                                    *pending = None;
                                }
                            }
                            Err(err) => {
                                eprintln!("sync-clip: join soft-fail (Sync Idle): {err}");
                                if let Ok(mut pending) = pending_join.lock() {
                                    *pending = Some(creds);
                                }
                            }
                        }
                    }
                } else if id == join_id {
                    let text = get_clipboard_text().unwrap_or_default();
                    match link_key_from_base32(text) {
                        Ok(key) => {
                            let ephemeral = credentials
                                .load()
                                .ok()
                                .flatten()
                                .map(|c| c.ephemeral_id)
                                .unwrap_or_else(generate_ephemeral_id);
                            let relay = credentials
                                .load()
                                .ok()
                                .flatten()
                                .map(|c| c.relay_ws_url)
                                .unwrap_or_else(default_relay_ws_url);
                            let creds = ShellCredentials {
                                ephemeral_id: ephemeral,
                                link_key: key,
                                relay_ws_url: relay,
                            };
                            if let Err(err) = credentials.save(&creds) {
                                eprintln!("sync-clip: save: {err}");
                            } else {
                                match join_session(&controller, &creds, armed_store.is_armed()) {
                                    Ok(()) => {
                                        if let Ok(mut pending) = pending_join.lock() {
                                            *pending = None;
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("sync-clip: join soft-fail (Sync Idle): {err}");
                                        if let Ok(mut pending) = pending_join.lock() {
                                            *pending = Some(creds);
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => eprintln!("sync-clip: clipboard is not a Link Key: {err}"),
                    }
                } else if id == arm_id {
                    let next = !armed_store.is_armed();
                    armed_store.set_armed(next);
                    if let Ok(c) = controller.lock() {
                        c.set_armed(next);
                    }
                } else if id == nick_id {
                    if let Some(text) = get_clipboard_text() {
                        nickname_store.save(&text);
                    }
                }
            }
            _ => {}
        }
    });
}

fn join_session(
    controller: &Arc<Mutex<ClipboardSyncController<WindowsClipboard>>>,
    creds: &ShellCredentials,
    armed: bool,
) -> Result<(), String> {
    let session = Session::new(
        creds.link_key.clone(),
        creds.relay_ws_url.clone(),
        creds.ephemeral_id.clone(),
    )
    .map_err(|e| e.to_string())?;
    session.set_armed(armed);
    let mut c = controller.lock().map_err(|e| e.to_string())?;
    c.detach();
    c.attach(Box::new(session));
    Ok(())
}

fn status_tooltip(
    nickname: Option<&str>,
    has_session: bool,
    armed: bool,
    sync_idle: bool,
) -> String {
    let mut tip = match nickname {
        Some(n) => format!("Sync Clip · {n}"),
        None => "Sync Clip".into(),
    };
    if !has_session {
        tip.push_str(" — no Sync Group");
    } else if sync_idle {
        tip.push_str(" — Sync Idle");
    } else if armed {
        tip.push_str(" — Armed");
    } else {
        tip.push_str(" — Paused");
    }
    tip
}

fn tray_icon_rgba() -> Icon {
    // Simple 16x16 teal square (matches Shell signal color #0F7A5A).
    let size = 16u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..(size * size) {
        rgba.extend_from_slice(&[0x0f, 0x7a, 0x5a, 0xff]);
    }
    Icon::from_rgba(rgba, size, size).expect("icon")
}

fn set_clipboard_text(text: &str) {
    if let Ok(mut clip) = arboard::Clipboard::new() {
        let _ = clip.set_text(text.to_string());
    }
}

fn get_clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}
