use serde::{Deserialize, Serialize};

/// AI 标签结果: 标签列表、摘要、来源(ai / rules)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiTagResult {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "ai".to_string()
}

#[derive(Debug, Deserialize, Default)]
struct AiResponse {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    summary: String,
}

pub async fn ollama_tag(
    content: &str,
    title: &str,
    endpoint: &str,
    model: &str,
) -> Result<AiTagResult, String> {
    let text = format!("{} {}", title, content);
    let truncated = if text.chars().count() > 2000 {
        let mut out = String::new();
        for (i, c) in text.chars().enumerate() {
            if i >= 2000 {
                break;
            }
            out.push(c);
        }
        out
    } else {
        text
    };

    // 重要: 切勿用 format!("...{truncated}"),用户剪贴板里可能含 `{` `}` 字面量,
    // 会触发 format! 解析失败而 panic。改为字符串拼接。
    let mut prompt = String::from(
        "你是一名助理,负责为剪贴板内容生成简短的中文标签和摘要。\n\
         要求:\n\
         1. tags 数组,3-5 个短词(中文/英文/技术名词皆可),不要超过 20 字符\n\
         2. summary 字符串,不超过 30 个字符\n\
         3. 只输出 JSON,不要任何其他文字、不要 markdown 代码块\n\
         JSON 格式: {\"tags\": [\"docker\", \"部署\"], \"summary\": \"...\"}\n\n\
         内容: ",
    );
    prompt.push_str(&truncated);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "format": "json"
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama connection failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let raw = data["response"].as_str().ok_or("No response from Ollama")?;

    let parsed: AiResponse = serde_json::from_str(raw).map_err(|e| {
        format!(
            "AI 返回非 JSON: {e}, 内容: {}",
            raw.chars().take(80).collect::<String>()
        )
    })?;

    let tags: Vec<String> = parsed
        .tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && t.chars().count() <= 20)
        .take(5)
        .collect();

    // 摘要目前不持久化,仅在内存中返回给前端
    Ok(AiTagResult {
        tags,
        summary: parsed.summary,
        source: "ai".to_string(),
    })
}
