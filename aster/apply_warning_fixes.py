from pathlib import Path
import re

ROOT = Path.cwd()
FILES = [ROOT / "src" / "screens" / "customer_master.rs", ROOT / "src" / "screens" / "supplier_master.rs"]
STROKE_WIDTH = re.compile(r"(Stroke::new\(\s*)(1\.(?:0|5|6|8))(?=\s*,)")

for path in FILES:
    if not path.exists():
        print(f"SKIP: {path.relative_to(ROOT)} not found")
        continue

    source = path.read_text(encoding="utf-8")
    source, count = STROKE_WIDTH.subn(lambda m: f"{m.group(1)}{m.group(2)}_f32", source)

    removed_shadow = False
    if path.name == "supplier_master.rs":
        shadow_line = re.compile(r"(?m)^\s*const SHADOW: Color32 = Color32::from_rgba_premultiplied\([^\n]*\);\s*\n")
        candidate, n = shadow_line.subn("", source, count=1)
        # Remove it only when no remaining code refers to SHADOW.
        if n and not re.search(r"\bSHADOW\b", candidate):
            source = candidate
            removed_shadow = True

    path.write_text(source, encoding="utf-8", newline="")
    print(f"UPDATED: {path.relative_to(ROOT)} — {count} Stroke width literal(s) fixed" + (", unused SHADOW removed" if removed_shadow else ""))
