# Blind Watermark

**语言：** [English](../README.md) | 简体中文 | [繁體中文](README.zh-TW.md)

Blind Watermark 是一个跨平台、商业级的盲水印工具，用于图片产权保护、泄露追踪和取证证据导出。项目包含 Rust 核心引擎、CLI 和 Tauri 2 桌面应用。

## 功能

- DWT + DCT + SVD-QIM 盲水印嵌入与提取。
- 每张图或每次分发生成唯一指纹 ID。
- 支持文件和目录批量处理。
- SQLite 证据库记录 SHA-256、感知哈希、PSNR、算法版本和产权信息。
- 支持 JSON、CSV、PDF 证据包导出。
- 桌面端支持英文、简体中文、繁体中文。
- 跨平台架构，支持 macOS、Windows、Linux。
- 提供 GitHub Actions 发布打包流程。

## 架构

- `crates/watermark-core`：Rust 核心库，负责图像变换、payload 编码、批处理、哈希和证据存储。
- `crates/watermark-cli`：命令行工具，二进制名为 `watermark`。
- `src-tauri`：Tauri 2 桌面壳和 Rust 命令。
- `ui/src`：React + TypeScript 桌面界面。

## 环境要求

- Rust stable。
- Node.js 20+。
- npm。
- Tauri 平台依赖。

Ubuntu/Linux 需要安装 WebKitGTK 等依赖：

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

## 开发

安装依赖：

```bash
npm install
```

启动桌面应用：

```bash
npm run tauri dev
```

开发服务器默认使用 `127.0.0.1:1420`。如果端口被占用，`scripts/dev-server.mjs` 会自动选择后续可用端口，并同步更新 Tauri 的 `devUrl`。

构建前端：

```bash
npm run build
```

运行测试：

```bash
cargo test --workspace
```

构建 Rust workspace：

```bash
cargo build --workspace
```

## 推荐默认参数

如果不确定怎么选，建议从下面这组开始：

- `--profile balanced`：大多数产权保护批处理的首选。
- `--quality 92`：画质较好，同时不会让 JPEG/WebP 文件过大。
- 不填写 `--strength`：程序会使用当前策略内置的默认强度。
- `--output-format preserve`：普通流程最省心；如果用于归档取证，可选择 `png`。
- 使用稳定的密钥或密钥文件并妥善保存。提取时必须使用同一个密钥。

如果图片经常被二次压缩或缩放，测试后再考虑 `strong`。如果画质优先于抗攻击恢复能力，可选择 `fidelity`。

## CLI 用法

嵌入水印：

```bash
cargo run -p watermark -- embed \
  --input ./images \
  --output ./out \
  --owner "Acme Studio" \
  --key ./secret.key \
  --profile balanced \
  --output-format png
```

提取水印：

```bash
cargo run -p watermark -- extract \
  --input ./out \
  --key ./secret.key \
  --report ./extract-report.json
```

验证指定水印 ID：

```bash
cargo run -p watermark -- verify \
  --input ./out/image.png \
  --watermark-id <uuid> \
  --key ./secret.key
```

导出证据：

```bash
cargo run -p watermark -- evidence export \
  --batch-id <uuid> \
  --format json \
  --output ./evidence.json
```

默认 SQLite 数据库为当前目录下的 `watermark-evidence.sqlite`，可用 `--db <path>` 覆盖。

## 算法说明

第一版只向图片中嵌入紧凑的唯一水印 ID，而不是完整版权文本。完整产权和分发信息保存在 SQLite 中，这样可以提升鲁棒性并降低载荷。

默认流程：

1. 解码图片并转换为 YCbCr。
2. 在亮度 Y 通道嵌入。
3. 使用 Haar DWT 并选择中频子带。
4. 在密钥派生的伪随机块上执行 8x8 DCT。
5. 使用 SVD-QIM 嵌入重复 payload bit。
6. 使用同一密钥盲提取，并输出置信度。

盲水印不是绝对不可破。强裁剪、截图、激进压缩或生成式重绘都会影响可恢复性。取证结果应结合哈希、元数据和置信度综合判断。

## 桌面应用

桌面端支持：

- 批量嵌入任务。
- 批量提取任务。
- 文件/目录路径支持拖拽、系统选择器和手动输入。
- 最近证据记录。
- 证据包导出。
- 英文、简体中文、繁体中文。
- 托盘后台行为：关闭窗口后隐藏，可从托盘显示或退出。

## GitHub 发布打包

`.github/workflows/release.yml` 会构建：

- macOS x64。
- macOS arm64。
- Windows x64。
- Linux x64。

GitHub 托管 runner 对 Linux/Windows 32 位安装包没有通用、稳定的一键矩阵。如果必须支持 32 位包，建议使用 self-hosted runner 或补充目标平台交叉编译配置。

创建发布：

```bash
git tag v0.1.0
git push origin v0.1.0
```

## 许可证

MIT，见 [LICENSE](../LICENSE)。
