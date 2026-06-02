export type ClipboardContentType = "code" | "url" | "text" | "image";

export interface ClipboardInsight {
  label: string;
  value: string;
}

export interface ClipboardAnalysis {
  title: string;
  type: ClipboardContentType;
  summary: string;
  tags: string[];
  insights: ClipboardInsight[];
  related: string[];
}

export function analyzeClipboardContent(content: string): ClipboardAnalysis {
  const normalized = content.toLowerCase();

  if (normalized.includes("docker run")) {
    return {
      title: "Docker部署命令",
      type: "code",
      summary: "这是一个 Docker 容器启动命令，适合 NAS 环境部署服务。",
      tags: ["Docker", "NAS", "命令行", "教程"],
      insights: [
        { label: "端口", value: "8080 → 80" },
        { label: "数据", value: "/volume1/docker/clipnest:/data" },
        { label: "策略", value: "unless-stopped" },
      ],
      related: ["飞牛 NAS 教程", "反向代理配置", "Compose 模板"],
    };
  }

  if (normalized.includes("http://") || normalized.includes("https://")) {
    return {
      title: "网页资料收藏",
      type: "url",
      summary: "这是一条网页资料链接，适合先收藏到待阅读并在之后整理成知识片段。",
      tags: ["网址", "资料", "待阅读"],
      insights: [
        { label: "类型", value: "网页链接" },
        { label: "状态", value: "待阅读" },
        { label: "建议", value: "保存后生成摘要" },
      ],
      related: ["资料收集箱", "AI 阅读清单"],
    };
  }

  if (normalized.includes("clipboard-image://") || normalized.includes(".png") || normalized.includes(".jpg")) {
    return {
      title: "图片素材记录",
      type: "image",
      summary: "这是一份图片或截图素材，适合作为教程、标注或产品说明的参考。",
      tags: ["图片", "素材", "待整理"],
      insights: [
        { label: "类型", value: "图片" },
        { label: "用途", value: "教程素材" },
        { label: "建议", value: "补充说明文字" },
      ],
      related: ["截图标注灵感", "产品说明图"],
    };
  }

  return {
    title: "文本片段",
    type: "text",
    summary: "这是一段普通文本内容，可以保存为笔记并在之后通过搜索快速复用。",
    tags: ["笔记", "文本"],
    insights: [
      { label: "类型", value: "普通文本" },
      { label: "场景", value: "知识摘录" },
      { label: "建议", value: "补充标题后保存" },
    ],
    related: ["临时笔记", "待整理内容"],
  };
}
