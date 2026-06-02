pub async fn ollama_tag(content: &str, title: &str) -> Result<String, String> {
    let text = format!("{} {}", title, content);
    let prompt = format!(
        "Generate 3-5 short comma-separated tags for this technical content. \
         Output ONLY the tags, nothing else. No explanation.\n\nContent: {}",
        text
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": "qwen2.5:3b",
        "prompt": prompt,
        "stream": false
    });

    let resp = client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama connection failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let tags = data["response"]
        .as_str()
        .ok_or("No response from Ollama")?;

    let cleaned: String = tags
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ',' || *c == ' ' || *c == '#')
        .collect();
    let cleaned = cleaned
        .split(',')
        .map(|t| t.trim().trim_start_matches('#'))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(",");

    Ok(cleaned)
}
