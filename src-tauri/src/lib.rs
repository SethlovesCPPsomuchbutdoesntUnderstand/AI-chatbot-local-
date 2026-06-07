use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[tauri::command]
async fn chat(messages: Vec<Message>) -> Result<String, String> {
    let client = reqwest::Client::new();

    let system = Message {
        role: "system".to_string(),
        content: "/no_think
You are a helpful, friendly, and knowledgeable AI assistant.
You answer questions clearly and concisely.
You are honest when you don't know something.
You keep responses conversational but informative.".to_string(),
    };

    // Build full message history with system prompt at the front
    let mut full_messages = vec![serde_json::json!({
        "role": system.role,
        "content": system.content
    })];

    for msg in &messages {
        full_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    let response = client
        .post("http://localhost:8081/v1/chat/completions")
        .timeout(std::time::Duration::from_secs(60))
        .json(&serde_json::json!({
            "model": "qwen",
            "max_tokens": 2048,
            "temperature": 0.7,
            "top_p": 0.9,
            "messages": full_messages
        }))
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("LLM server error: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let text = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    // Strip thinking tags
    let text = if let Some(end) = text.find("</think>") {
        &text[end + 8..]
    } else {
        text
    };

    Ok(text.trim().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![chat])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}