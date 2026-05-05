# AutoSubs App

这个目录是桌面应用本体，包含前端、Tauri 桥接和 Rust 后端。

如果你只是想使用当前仓库，优先看根目录 `README.md`。这里保留同样的精简说明，方便直接在 `AutoSubs-App` 目录下开发和构建。

## 安装教程

### 1. 安装前端依赖

```powershell
cd AutoSubs-App
npm install
```

### 2. 安装 Windows 构建依赖

```powershell
winget install Rustlang.Rustup
winget install Kitware.CMake
python -m pip install libclang
```

设置 `LIBCLANG_PATH`：

```powershell
$env:LIBCLANG_PATH = "$env:APPDATA\Python\Python314\site-packages\clang\native"
```

如果你要启用当前仓库的 Windows feature 构建，还需要 Vulkan SDK，并保证 `VULKAN_SDK` 已设置。

### 3. 创建 Python 3.12 虚拟环境

```powershell
py -3.12 -m venv .venv
```

### 4. 安装 `qwen-asr`

```powershell
.\.venv\Scripts\python.exe -m pip install --upgrade pip
.\.venv\Scripts\python.exe -m pip install -U qwen-asr
```

如果你要让 Qwen 使用 GPU，安装 CUDA 版 PyTorch：

```powershell
.\.venv\Scripts\python.exe -m pip uninstall -y torch torchvision torchaudio
.\.venv\Scripts\python.exe -m pip install --upgrade --force-reinstall torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu128
.\.venv\Scripts\python.exe -m pip install --upgrade --force-reinstall "huggingface_hub==0.36.2"
```

### 5. 下载 Qwen 模型

当前应用默认使用这个缓存目录：

```text
C:\Users\33287\AppData\Local\com.autosubs\models
```

安装 Hugging Face CLI：

```powershell
.\.venv\Scripts\python.exe -m pip install -U "huggingface_hub[cli]"
```

下载模型：

```powershell
$env:HF_HOME = "C:\Users\33287\AppData\Local\com.autosubs\models"
$env:HF_HUB_CACHE = "C:\Users\33287\AppData\Local\com.autosubs\models"
$env:TRANSFORMERS_CACHE = "C:\Users\33287\AppData\Local\com.autosubs\models"

.\.venv\Scripts\huggingface-cli.exe download Qwen/Qwen3-ASR-1.7B
.\.venv\Scripts\huggingface-cli.exe download Qwen/Qwen3-ForcedAligner-0.6B
```

### 6. 启动开发版

```powershell
npm run dev:win
```

### 7. 构建当前系统应用

有签名构建：

```powershell
npm run build:win
```

未签名本地构建：

```powershell
$env:AUTOSUBS_SKIP_SIGN = "1"
npm run build:win
```

可执行文件通常在：

```text
src-tauri\target\release\autosubs.exe
```

## Qwen 使用教程

### 1. 选择模型

在模型选择器里选择：

```text
Qwen3-ASR
```

建议语言：

- 中文：`zh`
- 英文：`en`
- 不确定：`auto`

### 2. 术语 / 上下文怎么填

当前应用已经把前端的 `custom_prompt` 接到了 Qwen 官方支持的 `context` 参数。

推荐填写：

- 术语
- 专有名词
- 品牌名
- 缩写
- 易错词

示例：

```text
DaVinci Resolve AutoSubs Fairlight Fusion Render Queue
```

或：

```text
劳熊 狂徒萨满 狂暴重击 非站立状态 开荒
```

不建议写成长篇提示词，例如“请润色”“请改写成书面语”。

### 3. 文本密度

`Text Density` 是字幕排版后处理，不是模型推理能力。

它会影响：

- 每条字幕长度
- 每行字符数
- 更偏短句还是更偏完整句

所以它对 `Whisper` 和 `Qwen3-ASR` 都会生效。

### 4. 当前已验证

当前应用已经验证通过：

- Qwen sidecar 转录可用
- ForcedAligner 时间戳可用
- Rust 集成链路可用
- Tauri 命令入口 smoke test 可用

## 致谢

本目录对应的应用工程基于原始 AutoSubs 继续开发。

特别感谢原作者 **Tom Moroney** 和原始项目：

- `https://github.com/tmoroney/auto-subs`

原项目在本地字幕工作流、DaVinci Resolve 集成和桌面应用结构上提供了坚实基础，当前 Qwen 版本是在这个基础上继续完成的。
