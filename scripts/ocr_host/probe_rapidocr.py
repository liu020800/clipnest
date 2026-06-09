"""Smoke test: feed the same fixture image we used for the C# probe through
RapidOCR and dump the parsed text. We will compare this output against the
expected text rendered into the PNG.
"""
import json
import sys
from pathlib import Path

from rapidocr import RapidOCR

FIXTURE = Path("H:/code/copyliusq/scripts/ocr_host/fixture.png")
OUT_JSON = Path("H:/code/copyliusq/scripts/ocr_host/rapidocr_out.json")


def main() -> int:
    if not FIXTURE.exists():
        print(f"missing fixture: {FIXTURE}")
        return 2
    engine = RapidOCR()
    result = engine(str(FIXTURE))
    payload = {
        "elapsed": result.elapse if hasattr(result, "elapse") else None,
        "boxes": result.boxes.tolist() if getattr(result, "boxes", None) is not None else [],
        "txts": result.txts if getattr(result, "txts", None) is not None else [],
        "scores": result.scores if getattr(result, "scores", None) is not None else [],
    }
    OUT_JSON.write_text(json.dumps(payload, ensure_ascii=False, indent=2))
    print("== RapidOCR result ==")
    for t, s in zip(payload["txts"], payload["scores"]):
        print(f"  [{s:.3f}] {t}")
    print(f"saved -> {OUT_JSON}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
