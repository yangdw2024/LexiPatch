import argparse
import json
import os
import re
import shutil
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Dict, List, Optional

SUPPORTED_EXTS = {
    ".json", ".arb", ".yaml", ".yml", ".toml",
    ".properties", ".ini", ".cfg", ".conf",
    ".po", ".pot", ".xml", ".resx", ".ts",
}

SKIP_DIRS = {
    "node_modules", ".git", ".transcore_backup", ".lexipatch_backup",
    "target", "__pycache__", ".venv", "dist", "build",
}

PLACEHOLDER_RE = re.compile(
    r"(\{\{[^{}]+\}\})"
    r"|(\$\{[^{}]+\})"
    r"|(\{[0-9a-zA-Z_.:-]+\})"
    r"|(%(?:\d+\$)?[+#0\- ]*(?:\d+)?(?:\.\d+)?[hlLzjt]*[diuoxXfFeEgGaAcspn%])"
    r"|(</?[a-zA-Z][a-zA-Z0-9-]*(?:\s+[^>]*)?/?>)"
    r"|(\\[ntr])"
)


def extract_placeholders(text: str) -> List[str]:
    return [m.group(0) for m in PLACEHOLDER_RE.finditer(text)]


def validate_placeholders(source: str, target: str) -> Optional[str]:
    t = target.strip()
    if not t:
        return None
    src_c = Counter(extract_placeholders(source))
    tgt_c = Counter(extract_placeholders(t))
    missing = [k for k, v in src_c.items() if tgt_c.get(k, 0) < v]
    if missing:
        return f"缺少占位符或标签: {', '.join(missing)}"
    extra = [k for k, v in tgt_c.items() if v > src_c.get(k, 0)]
    if extra:
        return f"多余占位符或标签: {', '.join(extra)}"
    return None


def scan_directory(root_dir: str) -> List[Dict[str, Any]]:
    root = Path(root_dir)
    if not root.is_dir():
        raise FileNotFoundError(f"Directory not found: {root_dir}")
    results: List[Dict[str, Any]] = []
    for cur, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d.lower() not in SKIP_DIRS]
        for fname in files:
            ext = Path(fname).suffix.lower()
            if ext not in SUPPORTED_EXTS:
                continue
            fpath = Path(cur) / fname
            rel = fpath.relative_to(root).as_posix()
            results.append({
                "relPath": rel,
                "absPath": str(fpath),
                "format": ext.lstrip("."),
                "sizeBytes": fpath.stat().st_size,
            })
    results.sort(key=lambda x: x["relPath"])
    return results


def extract_file(file_path: str) -> List[Dict[str, str]]:
    p = Path(file_path)
    ext = p.suffix.lower()
    raw = p.read_text(encoding="utf-8")
    units: List[Dict[str, str]] = []
    if ext in {".json", ".arb"}:
        data = json.loads(raw)

        def walk(node: Any, prefix: str) -> None:
            if isinstance(node, str) and prefix and node.strip():
                if ext == ".arb" and prefix.startswith("@"):
                    return
                units.append({"key": prefix, "source": node})
            elif isinstance(node, list):
                for i, v in enumerate(node):
                    walk(v, f"{prefix}[{i}]")
            elif isinstance(node, dict):
                for k, v in node.items():
                    if ext == ".arb" and str(k).startswith("@"):
                        continue
                    nk = f"{prefix}.{k}" if prefix else str(k)
                    walk(v, nk)

        walk(data, "")
    elif ext in {".ini", ".cfg", ".conf", ".properties"}:
        section = ""
        for line in raw.splitlines():
            s = line.strip()
            if not s or s.startswith("#") or s.startswith(";") or s.startswith("!"):
                continue
            if ext != ".properties" and s.startswith("[") and s.endswith("]"):
                section = s[1:-1].strip()
                continue
            if "=" in line:
                k, v = line.split("=", 1)
                key = f"{section}.{k.strip()}" if section else k.strip()
                if key and v.strip():
                    units.append({"key": key, "source": v.strip()})
    return units


def patch_dir(target_dir: str, files_map: Dict[str, str], patch_id: str = "default") -> int:
    root = Path(target_dir)
    if not root.is_dir():
        raise FileNotFoundError(f"Target directory not found: {target_dir}")
    bdir = root / ".lexipatch_backup" / patch_id
    bdir.mkdir(parents=True, exist_ok=True)
    record_path = bdir / "backup_manifest.json"
    entries: Dict[str, bool] = {}
    if record_path.exists():
        entries = json.loads(record_path.read_text(encoding="utf-8")).get("entries", {})

    count = 0
    for rel, content in files_map.items():
        dest = root / rel
        if rel not in entries:
            if dest.exists():
                bak = bdir / rel
                bak.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(dest, bak)
                entries[rel] = True
            else:
                entries[rel] = False
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(content, encoding="utf-8")
        count += 1

    record_path.write_text(json.dumps({"patch_id": patch_id, "entries": entries}, indent=2), encoding="utf-8")
    return count


def restore_dir(target_dir: str, patch_id: str = "default") -> int:
    root = Path(target_dir)
    bdir = root / ".lexipatch_backup" / patch_id
    record_path = bdir / "backup_manifest.json"
    if not record_path.exists():
        raise FileNotFoundError("Backup manifest not found")
    entries: Dict[str, bool] = json.loads(record_path.read_text(encoding="utf-8")).get("entries", {})
    restored = 0
    for rel, existed in entries.items():
        dest = root / rel
        if existed:
            bak = bdir / rel
            if bak.exists():
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(bak, dest)
                restored += 1
        elif dest.exists():
            dest.unlink()
            restored += 1
    shutil.rmtree(bdir, ignore_errors=True)
    return restored


def main() -> int:
    parser = argparse.ArgumentParser(prog="lexipatch", description="译铸 LexiPatch CLI v2.0.0")
    sub = parser.add_subparsers(dest="cmd")

    p_scan = sub.add_parser("scan", help="扫描目录内可汉化资源文件")
    p_scan.add_argument("dir", help="目标软件目录")

    p_ext = sub.add_parser("extract", help="提取单个资源文件中的词条")
    p_ext.add_argument("file", help="资源文件路径")

    p_qa = sub.add_parser("qa", help="校验原文与译文的占位符一致性")
    p_qa.add_argument("source", help="原文")
    p_qa.add_argument("target", help="译文")

    p_restore = sub.add_parser("restore", help="还原 .lexipatch_backup 备份")
    p_restore.add_argument("dir", help="目标软件目录")
    p_restore.add_argument("--id", default="default", help="补丁 ID")

    args = parser.parse_args()
    if args.cmd == "scan":
        print(json.dumps(scan_directory(args.dir), ensure_ascii=False, indent=2))
        return 0
    if args.cmd == "extract":
        print(json.dumps(extract_file(args.file), ensure_ascii=False, indent=2))
        return 0
    if args.cmd == "qa":
        warn = validate_placeholders(args.source, args.target)
        print(json.dumps({"ok": warn is None, "warning": warn}, ensure_ascii=False))
        return 0 if warn is None else 2
    if args.cmd == "restore":
        n = restore_dir(args.dir, args.id)
        print(f"Restored {n} files.")
        return 0
    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
