fn contains_word(text: &str, keyword: &str) -> bool {
    // CJK 关键词直接用 contains（无单词边界概念）
    let is_cjk = keyword.chars().any(|c| c >= '\u{4e00}' && c <= '\u{9fff}');
    if is_cjk {
        return text.contains(keyword);
    }
    // ASCII 关键词：检查单词边界
    let padded = format!(" {} ", text);
    let key_padded = format!(" {} ", keyword);
    padded.contains(&key_padded)
}

pub fn auto_tag(content: &str, title: &str) -> Option<String> {
    let text = format!("{} {}", title, content).to_lowercase();

    let rules: [(&str, &str); 45] = [
        ("docker", "docker"),
        ("linux", "linux"),
        ("bash", "bash"),
        ("shell", "shell"),
        ("python", "python"),
        ("rust", "rust"),
        ("javascript", "javascript"),
        ("typescript", "typescript"),
        ("react", "react"),
        ("vue", "vue"),
        ("git", "git"),
        ("github", "github"),
        ("sql", "sql"),
        ("api", "api"),
        ("rest", "rest"),
        ("graphql", "graphql"),
        ("css", "css"),
        ("html", "html"),
        ("node", "node"),
        ("npm", "npm"),
        ("cargo", "cargo"),
        ("tauri", "tauri"),
        ("windows", "windows"),
        ("wsl", "wsl"),
        ("prompt", "prompt"),
        ("ai", "ai"),
        ("llm", "llm"),
        ("ollama", "ollama"),
        ("dify", "dify"),
        ("mcp", "mcp"),
        ("nginx", "nginx"),
        ("redis", "redis"),
        ("postgresql", "postgresql"),
        ("postgres", "postgresql"),
        ("mysql", "mysql"),
        ("mongodb", "mongodb"),
        ("curl", "curl"),
        ("ssh", "ssh"),
        ("config", "config"),
        ("deploy", "deploy"),
        ("test", "test"),
        ("debug", "debug"),
        ("写作", "写作"),
        ("笔记", "笔记"),
        ("灵感", "灵感"),
    ];

    let mut tags: Vec<String> = Vec::new();
    for (keyword, tag) in rules {
        if contains_word(&text, keyword) {
            let tag_str = tag.to_string();
            if !tags.contains(&tag_str) {
                tags.push(tag_str);
            }
        }
    }

    tags.truncate(3);
    if tags.is_empty() {
        None
    } else {
        Some(tags.join(","))
    }
}
