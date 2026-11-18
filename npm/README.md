# 译铸 LexiPatch (`lexipatch`)

Node.js CLI wrapper for **译铸 LexiPatch**, a software localization resource scanner, placeholder QA validator, and format-preserving patcher.

## Quick Start

```bash
npx lexipatch scan ./my-software-dir
npx lexipatch extract ./locales/en.json
npx lexipatch qa "Hello %s" "你好 %s"
```

Desktop GUI (Rust + Tauri 2 + SQLite + AI Batch Translation + Standalone `.exe` Patcher Builder):
https://github.com/yangdw2024/LexiPatch
