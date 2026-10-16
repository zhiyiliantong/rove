"""Fail closed when a packaged Tauri binary lacks the current frontend keys."""
import re
from pathlib import Path


def validate_frontend_assets(index: Path, binary: Path) -> None:
    if not index.is_file():
        raise ValueError("missing frontend dist/index.html; build the frontend first")
    keys = re.findall(r'"/(assets/[^"\s]+)"', index.read_text())
    if not binary.is_file() or not keys:
        raise ValueError("missing GUI binary or frontend entry assets")
    data = binary.read_bytes()
    # Tauri compresses asset contents, but not the embedded lookup keys.
    for key in keys:
        if key.encode() not in data:
            raise ValueError(
                f"GUI does not embed current frontend asset {key}; rebuild GUI "
                "after frontend, without concurrent frontend builds"
            )
