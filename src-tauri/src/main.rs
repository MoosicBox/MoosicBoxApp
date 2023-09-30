// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use moosicbox_core::slim::menu::Album;
use tauri::Manager;

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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            get_albums,
            api_proxy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
