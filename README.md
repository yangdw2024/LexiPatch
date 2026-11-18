# 译铸 · LexiPatch

> Rust + Tauri 2 + SQLite 驱动的软件汉化与独立 `.exe` 补丁铸造平台

**译铸 LexiPatch** 面向开发者、开源汉化组与本地化团队，提供从「扫描真实软件目录 → 格式保留解析 → 高并发大模型翻译 → 占位符 QA 校验 → 一键铸造独立 `.exe` 汉化补丁」的完整本地化工作流。

---

## 核心特性

- **真实目录扫描与格式保留回写**：支持 `JSON` / `ARB` / `YAML` / `TOML` / `Java .properties` / `INI` / `Gettext .po` / `XML` / `RESX` / `Qt .ts`，回写时保留注释、键序、缩进与非字符串字段。
- **高并发大模型翻译引擎**：内置 DeepSeek、通义千问、OpenAI、本地 Ollama 及任意 OpenAI 兼容接口配置，支持术语表注入、翻译记忆精确预填、并发批次调度与实时进度反馈。
- **占位符 QA 自动校验**：自动检测 `%s`、`%d`、`{0}`、`{{var}}`、`${var}`、HTML 标签与转义符，防止译文破坏目标软件运行。
- **独立 `.exe` 汉化安装器**：将汉化文件与清单压缩嵌入纯 Rust Win32 自解压引导器（`patcher-stub.exe`），终端用户双击即可通过注册表/默认路径自动定位软件目录、自动备份原版至 `.transcore_backup/` 并支持随时一键还原。
- **跨生态 CLI 工具**：同时提供 [PyPI (`lexipatch`)](https://pypi.org/project/lexipatch/) 与 [npm (`lexipatch`)](https://www.npmjs.com/package/lexipatch) 命令行工具。

---

## 快速开始

### 1. 命令行工具（PyPI / npm）

```bash
# 通过 pip 安装
pip install lexipatch

# 或通过 npx 直接运行
npx lexipatch scan ./my-app
npx lexipatch extract ./locales/en.json
npx lexipatch qa "Hello %s" "你好 %s"
```

### 2. 桌面端构建（Rust + Tauri 2）

```bash
pnpm install
# 1. 编译独立 .exe 补丁引导器
cargo build --release -p transcore-patcher
copy target\release\transcore-patcher.exe src-tauri\resources\patcher-stub.exe

# 2. 启动桌面端开发或构建发布包
pnpm tauri dev
pnpm tauri build
```

---

## License

MIT
