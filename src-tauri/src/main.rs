#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mpris::{PlaybackStatus, PlayerFinder};
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
struct PlayerState {
    track_title: String,
    artist: String,
    album_art: String,
    is_playing: bool,
    volume: f64,
    position_ms: u64,
    duration_ms: u64,
}

fn find_spotify_player(finder: &PlayerFinder) -> Option<mpris::Player> {
    finder.find_all().ok().and_then(|players| {
        players
            .into_iter()
            .find(|p| p.identity().to_lowercase().contains("spotify"))
    })
}

fn monitor_player(app: AppHandle) {
    std::thread::spawn(move || {
        let finder = match PlayerFinder::new() {
            Ok(f) => f,
            Err(_) => return,
        };

        loop {
            std::thread::sleep(Duration::from_millis(500));

            let player_opt = find_spotify_player(&finder);

            let Some(player) = player_opt else {
                let _ = app.emit(
                    "mpris-status",
                    PlayerState {
                        track_title: "No Active Player".to_string(),
                        artist: "Start spotifyd".to_string(),
                        album_art: "".to_string(),
                        is_playing: false,
                        volume: 0.5,
                        position_ms: 0,
                        duration_ms: 1,
                    },
                );
                continue;
            };

            let metadata = player.get_metadata().ok();
            let playback_status = player
                .get_playback_status()
                .unwrap_or(PlaybackStatus::Stopped);
            let volume = player.get_volume().unwrap_or(0.5);
            let position = player.get_position().unwrap_or(Duration::from_secs(0));

            let title = metadata
                .as_ref()
                .and_then(|m| m.title())
                .unwrap_or("Unknown Title")
                .to_string();

            let artist = metadata
                .as_ref()
                .and_then(|m| m.artists())
                .map(|artists| artists.join(", "))
                .unwrap_or_else(|| "Unknown Artist".to_string());

            let album_art = metadata
                .as_ref()
                .and_then(|m| m.art_url())
                .unwrap_or("")
                .to_string();

            let duration_ms = metadata
                .as_ref()
                .and_then(|m| m.length())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(1);

            let is_playing = matches!(playback_status, PlaybackStatus::Playing);

            let state = PlayerState {
                track_title: title,
                artist,
                album_art,
                is_playing,
                volume,
                position_ms: position.as_millis() as u64,
                duration_ms,
            };

            let _ = app.emit("mpris-status", state);
        }
    });
}

#[tauri::command]
fn toggle_play_pause() -> Result<(), String> {
    let finder = PlayerFinder::new().map_err(|e| e.to_string())?;
    if let Some(player) = find_spotify_player(&finder) {
        player.play_pause().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn next_track() -> Result<(), String> {
    let finder = PlayerFinder::new().map_err(|e| e.to_string())?;
    if let Some(player) = find_spotify_player(&finder) {
        player.next().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn previous_track() -> Result<(), String> {
    let finder = PlayerFinder::new().map_err(|e| e.to_string())?;
    if let Some(player) = find_spotify_player(&finder) {
        player.previous().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn set_volume(volume: f64) -> Result<(), String> {
    let finder = PlayerFinder::new().map_err(|e| e.to_string())?;
    if let Some(player) = find_spotify_player(&finder) {
        player.set_volume(volume).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            monitor_player(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            toggle_play_pause,
            next_track,
            previous_track,
            set_volume
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
