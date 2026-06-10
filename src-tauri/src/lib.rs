use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Emitter;

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
}

// Get the path to save chats
fn chats_dir() -> std::path::PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("local-chat");
    path.push("chats");
    fs::create_dir_all(&path).ok();
    path
}

#[tauri::command]
fn save_chat(chat: Chat) -> Result<(), String> {
    let path = chats_dir().join(format!("{}.json", chat.id));
    let json = serde_json::to_string(&chat).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_chats() -> Result<Vec<Chat>, String> {
    let dir = chats_dir();
    let mut chats = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(chat) = serde_json::from_str::<Chat>(&content) {
                    chats.push(chat);
                }
            }
        }
    }

    // Sort by title
    chats.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(chats)
}

#[tauri::command]
fn delete_chat(id: String) -> Result<(), String> {
    let path = chats_dir().join(format!("{}.json", id));
    fs::remove_file(path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat(window: tauri::Window, messages: Vec<Message>) -> Result<String, String> {
    let client = reqwest::Client::new();

    let system = serde_json::json!({
        "role": "system",
        "content": "/no_think\nYou are a helpful, friendly, and knowledgeable AI assistant. You answer questions clearly and concisely. You are honest when you don't know something."
    });

    let mut full_messages = vec![system];
    for msg in &messages {
        full_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    let response = client
        .post("http://localhost:8081/v1/chat/completions")
        .timeout(std::time::Duration::from_secs(120))
        .json(&serde_json::json!({
            "model": "qwen",
            "max_tokens": 2048,
            "temperature": 0.7,
            "top_p": 0.9,
            "stream": true,
            "messages": full_messages
        }))
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("LLM server error: {}", response.status()));
    }

    let mut stream = response.bytes_stream();
    let mut full_text = String::new();
    let mut past_thinking = false;

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(_) => break,
        };
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];
            if data == "[DONE]" {
                break;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                    full_text.push_str(delta);

                    if !past_thinking {
                        if full_text.contains("</think>") {
                            past_thinking = true;
                        } else {
                            continue;
                        }
                    }

                    let visible = if let Some(end) = full_text.find("</think>") {
                        full_text[end + 8..].trim().to_string()
                    } else {
                        full_text.trim().to_string()
                    };

                    if !visible.is_empty() {
                        let _ = window.emit("chat-chunk", &visible);
                    }
                }
            }
        }
    }

    let final_text = if let Some(end) = full_text.find("</think>") {
        full_text[end + 8..].trim().to_string()
    } else {
        full_text.trim().to_string()
    };

    Ok(final_text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            chat,
            save_chat,
            load_chats,
            delete_chat
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
