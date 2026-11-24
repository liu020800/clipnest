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

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

fn build_prompt(content: &str, title: &str) -> String {
    let text = format!("{} {}", title, content);
    let truncated = if text.chars().count() > 2000 {
        text.chars().take(2000).collect::<String>()
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
    prompt
}

fn parse_ai_json(raw: &str) -> Result<AiTagResult, String> {
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

    Ok(AiTagResult {
        tags,
        summary: parsed.summary,
        source: "ai".to_string(),
    })
}

fn normalize_openai_endpoint(endpoint: &str) -> Result<reqwest::Url, String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err("OpenAI-compatible endpoint is required".to_string());
    }

    let mut url = reqwest::Url::parse(endpoint)
        .map_err(|_| "OpenAI-compatible endpoint must be a valid URL".to_string())?;
    let host = url
        .host_str()
        .ok_or_else(|| "OpenAI-compatible endpoint must include a host".to_string())?;
    let host_lower = host.to_ascii_lowercase();

    if url.username() != "" || url.password().is_some() {
        return Err("OpenAI-compatible endpoint must not include credentials".to_string());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("OpenAI-compatible endpoint must not include a query or fragment".to_string());
    }
    if host_lower == "bps.openai.com" || host_lower.ends_with(".bps.openai.com") {
        return Err("BPS endpoints are not supported".to_string());
    }

    match url.scheme() {
        "https" => {}
        "http" if matches!(host_lower.as_str(), "localhost" | "127.0.0.1" | "::1") => {}
        "http" => {
            return Err("HTTP endpoints are allowed only for localhost".to_string());
        }
        _ => return Err("OpenAI-compatible endpoint must use HTTPS".to_string()),
    }

    let path = url.path().trim_end_matches('/');
    let normalized_path = if path.ends_with("/chat/completions") {
        path.to_string()
    } else if path.ends_with("/v1") {
        format!("{path}/chat/completions")
    } else if path.is_empty() {
        "/v1/chat/completions".to_string()
    } else {
        format!("{path}/v1/chat/completions")
    };
    url.set_path(&normalized_path);
    Ok(url)
}

pub async fn ollama_tag(
    content: &str,
    title: &str,
    endpoint: &str,
    model: &str,
) -> Result<AiTagResult, String> {
    let prompt = build_prompt(content, title);

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

    parse_ai_json(raw)
}

/// Call an explicitly configured OpenAI-compatible chat-completions endpoint.
///
/// The endpoint is supplied by the user and the bearer token is read only from
/// the named process environment variable; no OAuth, token replay, or
/// account-pool behavior is involved. BPS/Office endpoints are rejected.
pub async fn openai_compatible_tag(
    content: &str,
    title: &str,
    endpoint: &str,
    model: &str,
    api_key_env: &str,
) -> Result<AiTagResult, String> {
    let url = normalize_openai_endpoint(endpoint)?;
    let model = model.trim();
    if model.is_empty() {
        return Err("OpenAI-compatible model is required".to_string());
    }

    let env_name = api_key_env.trim();
    if env_name.is_empty()
        || env_name.len() > 128
        || !env_name.chars().enumerate().all(|(index, ch)| {
            (index == 0 && (ch == '_' || ch.is_ascii_alphabetic()))
                || (index > 0 && (ch == '_' || ch.is_ascii_alphanumeric()))
        })
    {
        return Err("OpenAI-compatible API key environment variable name is invalid".to_string());
    }
    let api_key = std::env::var(env_name)
        .map_err(|_| format!("{env_name} environment variable is not set"))?;
    if api_key.trim().is_empty() {
        return Err(format!("{env_name} environment variable is empty"));
    }

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "你只能输出符合要求的 JSON,不要输出 markdown 或其他解释。"
            },
            {
                "role": "user",
                "content": build_prompt(content, title)
            }
        ],
        "temperature": 0.2,
        "stream": false
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(url)
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI-compatible connection failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("OpenAI-compatible request failed (HTTP {status})"));
    }

    let data: ChatCompletionResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI-compatible response: {e}"))?;
    let raw = data
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or_else(|| "OpenAI-compatible response has no message content".to_string())?;

    parse_ai_json(raw)
}

#[cfg(test)]
mod tests {
    use super::{normalize_openai_endpoint, parse_ai_json};

    #[test]
    fn openai_endpoint_appends_chat_completions_path() {
        let url = normalize_openai_endpoint("https://api.openai.com/v1").unwrap();
        assert_eq!(url.as_str(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn openai_endpoint_allows_local_http_only() {
        assert!(normalize_openai_endpoint("http://localhost:1234/v1").is_ok());
        assert!(normalize_openai_endpoint("http://example.test/v1").is_err());
    }

    #[test]
    fn openai_endpoint_rejects_bps_host() {
        assert!(normalize_openai_endpoint("https://bps.openai.com/v1").is_err());
    }

    #[test]
    fn parse_ai_json_trims_and_limits_tags() {
        let result = parse_ai_json(
            r#"{"tags":[" docker ","","this-tag-is-way-too-long-for-the-ui"],"summary":"ok"}"#,
        )
        .unwrap();
        assert_eq!(result.tags, vec!["docker"]);
        assert_eq!(result.summary, "ok");
    }
}
