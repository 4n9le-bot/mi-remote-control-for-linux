#!/usr/bin/env python3
"""Generate the committed Linux Logical Key registry from a pinned UAPI header."""
import argparse, hashlib, re
from pathlib import Path

KEY = re.compile(r"^#define\s+(KEY_[A-Z0-9_]+)\s+((?:0x)?[0-9a-fA-F]+)\b")
EXCLUDED = {"KEY_RESERVED", "KEY_UNKNOWN", "KEY_MAX"}

def parse(path: Path):
    text = path.read_text(encoding="utf-8")
    seen = set(); rows = []
    for line in text.splitlines():
        match = KEY.match(line.strip())
        if not match: continue
        symbol, raw = match.groups(); code = int(raw, 0)
        if code in seen: continue
        seen.add(code)
        if symbol in EXCLUDED: continue
        label = symbol.removeprefix("KEY_").replace("_", " ").title()
        rows.append((symbol, code, label))
    return text, rows

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("header", type=Path)
    ap.add_argument("output", type=Path)
    ap.add_argument("--linux-tag", required=True)
    ap.add_argument("--catalog-version", type=int, required=True)
    args = ap.parse_args()
    text, rows = parse(args.header)
    digest = hashlib.sha256(text.encode()).hexdigest()
    lines = ["use super::LogicalKey;", "", f'pub const REGISTRY_LINUX_TAG: &str = "{args.linux_tag}";', f'pub const REGISTRY_SOURCE_SHA256: &str = "{digest}";', 'pub const REGISTRY_LICENSE: &str = "GPL-2.0-only WITH Linux-syscall-note";', f'pub const REGISTRY_CATALOG_VERSION: u32 = {args.catalog_version};', "", "pub static LOGICAL_KEYS: &[LogicalKey] = &[" ]
    lines += [f'    LogicalKey {{ symbol: "{s}", code: {c}, label: "{l}" }},' for s,c,l in rows]
    lines += ["];", ""]
    args.output.write_text("\n".join(lines), encoding="utf-8")

if __name__ == "__main__": main()
