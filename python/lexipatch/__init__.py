"""LexiPatch (译铸) — Software localization scanner, QA validator, and patcher."""

__version__ = "2.0.1"

from .cli import extract_file, patch_dir, restore_dir, scan_directory, validate_placeholders

__all__ = [
    "__version__",
    "scan_directory",
    "extract_file",
    "validate_placeholders",
    "patch_dir",
    "restore_dir",
]
