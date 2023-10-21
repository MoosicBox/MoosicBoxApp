// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    env,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use debounce::EventDebouncer;
use log::{debug, trace};
use moosicbox_core::sqlite::models::Album;
use serde_json::json;
use tauri::{AppHandle, Manager};
use tauri_plugin_aptabase::EventTracker;
use tauri_plugin_log::LogTarget;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn show_main_window(window: tauri::Window) {
    window.get_window("main").unwrap().show().unwrap(); // replace "main" by the name of your window
}

#[tauri::command]
async fn get_albums() -> Vec<Album> {
    //moosicbox_core::sqlite::menu::get_albums()
    vec![Album {
        id: 121,
        title: "test alb".into(),
        artist: "test".into(),
        artist_id: 123,
        ..Default::default()
    }]
}

#[tauri::command]
async fn api_proxy(url: String) -> serde_json::Value {
    println!("Fetching url from proxy: {url}");
    reqwest::get(url).await.unwrap().json().await.unwrap()
}

static DISABLED_EVENTS: [&str; 2] = ["app_main_events_cleared", "app_window_event"];

fn track_event(handler: &AppHandle, name: &str, props: Option<serde_json::Value>) {
    static DEBOUNCER_COUNTS: OnceLock<Mutex<HashMap<String, u16>>> = OnceLock::new();
    static DEBOUNCER: OnceLock<EventDebouncer<String>> = OnceLock::new();

    DEBOUNCER.get_or_init(|| {
        let debounce_duration = Duration::from_millis(10);
        EventDebouncer::new(debounce_duration, move |data: String| {
            let counts = DEBOUNCER_COUNTS.get().unwrap();
            let count = *counts.lock().unwrap().get(&data).unwrap_or(&0);
            if count > 1 {
                trace!("{data} ({count} times)");
            } else {
                trace!("{}", data);
            }
            counts.lock().unwrap().remove(&data);
        })
    });
    DEBOUNCER_COUNTS.get_or_init(|| Mutex::new(HashMap::new()));

    if DISABLED_EVENTS.iter().any(|n| *n == name) {
        let message = format!("Not tracking disabled event {name}: {props:?}").to_string();

        DEBOUNCER.get().unwrap().put(message.clone());

        let counts = DEBOUNCER_COUNTS.get().unwrap();
        let count = *counts.lock().unwrap().get(&message).unwrap_or(&0);
        counts.lock().unwrap().insert(message.clone(), count + 1);
        return;
    }

    debug!("Tracking event {name}: {props:?}");
    handler.track_event(name, props);
}

fn main() {
    let aptabase_app_key = std::env!("APTABASE_APP_KEY");

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    LogTarget::Stdout,
                    // LogTarget::Webview,
                    LogTarget::LogDir,
                ])
                .build(),
        )
        .plugin(tauri_plugin_aptabase::Builder::new(aptabase_app_key).build())
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            get_albums,
            api_proxy
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|handler, event| match event {
            tauri::RunEvent::Exit { .. } => {
                track_event(handler, "app_exited", None);
                handler.flush_events_blocking();
            }
            tauri::RunEvent::ExitRequested { api, .. } => track_event(
                handler,
                "app_exit_requested",
                Some(json!({"api": format!("{api:?}")})),
            ),
            tauri::RunEvent::WindowEvent { label, event, .. } => track_event(
                handler,
                "app_window_event",
                Some(json!({"label": label, "event": format!("{event:?}")})),
            ),
            tauri::RunEvent::Ready => track_event(handler, "app_ready", None),
            tauri::RunEvent::Resumed => track_event(handler, "app_resumed", None),
            tauri::RunEvent::MainEventsCleared => {
                track_event(handler, "app_main_events_cleared", None)
            }
            _ => {}
        });
}
