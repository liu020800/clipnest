# ClipNest Python OCR host: standalone executable.
# 接收图片路径, 输出 JSON 一行。
#
# 设计目标:
#   - 与原 wcocr.exe 协议兼容 (--json 输出 {"text": "..."})
#   - 使用 RapidOCR (PaddleOCR-based, 离线, 不依赖本地微信)
#   - 首次运行自动保证 rapidocr + onnxruntime 已安装 + 模型已下载
#   - 中文/英文都支持
#
# 退出码:
#   0  success
#   2  invalid usage
#   3  image not found
#   4  engine error
#   5  setup error (pip install failed, model download failed)
#
# 性能目标: 中文短图 < 3s (CPU, 已下载模型)
import json
import re
import sys
import time
from pathlib import Path


REQUIRED = {
    "rapidocr": "rapidocr>=3.8.0",
    "onnxruntime": "onnxruntime>=1.17.0",
    "PIL": "pillow>=10.0.0",
}


def ensure_dependencies() -> None:
    """通过 pip 自举, 跳过已安装的包。失败抛 SetupError。"""
    missing = []
    for mod, _spec in REQUIRED.items():
        try:
            __import__(mod)
        except ImportError:
            missing.append(_spec)
    if not missing:
        return
    cmd = [sys.executable, "-m", "pip", "install",
           "--disable-pip-version-check", "--quiet"] + missing
    import subprocess
    proc = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        sys.stderr.write(
            "setup failed installing "
            + ", ".join(m.split(">=")[0] for m in missing)
            + ": "
            + proc.stderr.decode(errors="ignore")[:300]
            + "\n"
        )
        sys.exit(5)


def recognize(image_path: str) -> dict:
    from rapidocr import RapidOCR
    prepared_path, cleanup_path = prepare_image(image_path)
    engine = RapidOCR(params={"Global.log_level": "error"})
    started = time.time()
    result = engine(prepared_path)
    elapsed_ms = int((time.time() - started) * 1000)
    if cleanup_path is not None:
        try:
            cleanup_path.unlink(missing_ok=True)
        except OSError:
            pass

    boxes = getattr(result, "boxes", None)
    txts = getattr(result, "txts", None) or []
    scores = getattr(result, "scores", None) or []

    if boxes is None or len(txts) == 0:
        return {"text": "", "elapsed_ms": elapsed_ms, "lines": []}

    lines: list[dict] = []
    for i, text in enumerate(txts):
        score = float(scores[i]) if i < len(scores) else 0.0
        if score < 0.55:
            continue
        box = boxes[i].tolist() if i < len(boxes) else []
        lines.append({"text": clean_ocr_text(repair_mojibake(text)), "score": score, "box": box})
    if not lines:
        return {"text": "", "elapsed_ms": elapsed_ms, "lines": []}

    def y_of(line):
        b = line["box"]
        if not b:
            return 0.0
        return sum(p[1] for p in b) / len(b)

    def x_of(line):
        b = line["box"]
        if not b:
            return 0.0
        return min(p[0] for p in b)

    lines.sort(key=lambda l: (y_of(l) // 20, x_of(l)))
    text_out: list[str] = []
    last_y: float | None = None
    for line in lines:
        y = y_of(line)
        if last_y is not None and abs(y - last_y) > 20:
            text_out.append("\n")
        elif text_out and not text_out[-1].endswith("\n"):
            text_out.append(" ")
        text_out.append(line["text"])
        last_y = y
    return {"text": "".join(text_out).strip(), "elapsed_ms": elapsed_ms, "lines": lines}


def repair_mojibake(text: str) -> str:
    candidates = [text]
    for source in ("latin1", "cp1252"):
        for target in ("utf-8", "gb18030"):
            try:
                candidates.append(text.encode(source).decode(target))
            except UnicodeError:
                pass
    return max(set(candidates), key=score_text)


def score_text(text: str) -> int:
    cjk = sum("\u4e00" <= ch <= "\u9fff" for ch in text)
    ascii_printable = sum(" " <= ch <= "~" for ch in text)
    controls = sum(ord(ch) < 32 for ch in text)
    mojibake = sum(ch in "ÃÂÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ�" for ch in text)
    return cjk * 12 + ascii_printable - controls * 8 - mojibake * 4


def clean_ocr_text(text: str) -> str:
    # Common screen OCR artifact: an isolated I/l/1 inserted between Chinese chars.
    text = re.sub(r"(?<=[\u4e00-\u9fff])\s*[Il1]\s*(?=[\u4e00-\u9fff])", "", text)
    return text


def prepare_image(image_path: str) -> tuple[str, Path | None]:
    """Improve small screen text before OCR without touching the source crop."""
    from PIL import Image, ImageEnhance, ImageFilter, ImageOps

    src = Path(image_path)
    img = Image.open(src).convert("RGB")
    w, h = img.size
    scale = 1
    if min(w, h) < 160:
        scale = 3
    elif min(w, h) < 320:
        scale = 2
    if scale > 1:
        img = img.resize((w * scale, h * scale), Image.Resampling.LANCZOS)
    img = ImageOps.grayscale(img)
    img = ImageEnhance.Contrast(img).enhance(1.8)
    img = ImageEnhance.Sharpness(img).enhance(1.6)
    img = img.filter(ImageFilter.UnsharpMask(radius=1, percent=140, threshold=3))

    out = src.with_name(f"{src.stem}.ocr.png")
    img.save(out)
    return str(out), out


def main(argv: list[str]) -> int:
    want_json = "--json" in argv
    args = [a for a in argv[1:] if a != "--json"]
    if not args:
        sys.stderr.write("usage: rcroc_ocr <image_path> [--json]\n")
        return 2
    image_path = args[0]
    if not Path(image_path).exists():
        sys.stderr.write(f"image not found: {image_path}\n")
        return 3

    ensure_dependencies()

    try:
        out = recognize(image_path)
    except Exception as exc:  # noqa: BLE001
        sys.stderr.write(f"ocr error: {exc}\n")
        return 4

    if want_json:
        payload = json.dumps(out, ensure_ascii=False)
        sys.stdout.buffer.write(payload.encode("utf-8") + b"\n")
    else:
        sys.stdout.buffer.write(out["text"].encode("utf-8") + b"\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
