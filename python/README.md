# 译铸 LexiPatch (`lexipatch`)

**译铸 LexiPatch** 是一款面向开发者与本地化团队的软件汉化与独立补丁铸造工具。

本 PyPI 包提供 `lexipatch` 命令行工具与 Python 库，支持：
- **资源目录扫描 (`lexipatch scan <dir>`)**：自动识别 JSON / ARB / YAML / TOML / Properties / INI / PO / XML / RESX / Qt `.ts` 文件。
- **词条提取 (`lexipatch extract <file>`)**：提取键值对与待翻译文本。
- **占位符 QA 校验 (`lexipatch qa <source> <target>`)**：校验 `%s`、`{0}`、`{{var}}`、`${var}`、HTML 标签与转义符。
- **原位备份打补丁与还原 (`lexipatch patch` / `lexipatch restore`)**：自动备份原文件至 `.lexipatch_backup/` 并支持一键还原。

桌面端完整版（Rust + Tauri 2 + SQLite + 高并发大模型翻译 + 一键导出独立 `.exe` 安装器）请访问 GitHub Releases：
https://github.com/yangdw2024/LexiPatch
