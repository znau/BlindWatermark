# Blind Watermark

**語言：** [English](../README.md) | [简体中文](README.zh-CN.md) | 繁體中文

Blind Watermark 是一個跨平台、商業級的盲浮水印工具，用於圖片產權保護、洩漏追蹤與取證證據匯出。專案包含 Rust 核心引擎、CLI 和 Tauri 2 桌面應用。

## 功能

- DWT + DCT + SVD-QIM 盲浮水印嵌入與提取。
- 每張圖或每次分發建立唯一指紋 ID。
- 支援檔案和資料夾批次處理。
- SQLite 證據庫記錄 SHA-256、感知雜湊、PSNR、演算法版本和產權資訊。
- 支援 JSON、CSV、PDF 證據包匯出。
- 桌面端支援英文、簡體中文、繁體中文。
- 跨平台架構，目前發布打包聚焦 Windows 和 Linux。
- 提供 GitHub Actions 發布打包流程。

## 架構

- `crates/watermark-core`：Rust 核心庫，負責圖像變換、payload 編碼、批次處理、雜湊和證據儲存。
- `crates/watermark-cli`：命令列工具，二進位名稱為 `watermark`。
- `src-tauri`：Tauri 2 桌面殼和 Rust 命令。
- `ui/src`：React + TypeScript 桌面介面。

## 環境需求

- Rust stable。
- Node.js 20+。
- npm。
- Tauri 平台依賴。

Ubuntu/Linux 需要安裝 WebKitGTK 等依賴：

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

## 開發

安裝依賴：

```bash
npm install
```

啟動桌面應用：

```bash
npm run tauri dev
```

開發伺服器預設使用 `127.0.0.1:1420`。如果連接埠被佔用，`scripts/dev-server.mjs` 會自動選擇後續可用連接埠，並同步更新 Tauri 的 `devUrl`。

建置前端：

```bash
npm run build
```

執行測試：

```bash
cargo test --workspace
```

建置 Rust workspace：

```bash
cargo build --workspace
```

## 建議預設參數

如果不確定怎麼選，建議從下面這組開始：

- `--profile balanced`：大多數產權保護批次處理的首選。
- `--quality 92`：畫質較好，同時不會讓 JPEG/WebP 檔案過大。
- 不填寫 `--strength`：程式會使用目前策略內建的預設強度。
- `--output-format preserve`：普通流程最省心；如果用於歸檔取證，可選擇 `png`。
- 使用穩定的密鑰或密鑰檔案並妥善保存。提取時必須使用同一個密鑰。

如果圖片經常被二次壓縮或縮放，測試後再考慮 `strong`。如果畫質優先於抗攻擊恢復能力，可選擇 `fidelity`。

## CLI 用法

嵌入浮水印：

```bash
cargo run -p watermark -- embed \
  --input ./images \
  --output ./out \
  --owner "Acme Studio" \
  --key ./secret.key \
  --profile balanced \
  --output-format png
```

提取浮水印：

```bash
cargo run -p watermark -- extract \
  --input ./out \
  --key ./secret.key \
  --report ./extract-report.json
```

驗證指定浮水印 ID：

```bash
cargo run -p watermark -- verify \
  --input ./out/image.png \
  --watermark-id <uuid> \
  --key ./secret.key
```

匯出證據：

```bash
cargo run -p watermark -- evidence export \
  --batch-id <uuid> \
  --format json \
  --output ./evidence.json
```

預設 SQLite 資料庫為目前目錄下的 `watermark-evidence.sqlite`，可用 `--db <path>` 覆寫。

## 演算法說明

第一版只向圖片中嵌入精簡的唯一浮水印 ID，而不是完整版權文字。完整產權與分發資訊保存在 SQLite 中，這樣可以提升魯棒性並降低載荷。

預設流程：

1. 解碼圖片並轉換為 YCbCr。
2. 在亮度 Y 通道嵌入。
3. 使用 Haar DWT 並選擇中頻子帶。
4. 在密鑰派生的偽隨機區塊上執行 8x8 DCT。
5. 使用 SVD-QIM 嵌入重複 payload bit。
6. 使用同一密鑰盲提取，並輸出信賴度。

盲浮水印不是絕對不可破。強裁切、截圖、激進壓縮或生成式重繪都會影響可恢復性。取證結果應結合雜湊、元資料和信賴度綜合判斷。

## 桌面應用

桌面端支援：

- 批次嵌入任務。
- 批次提取任務。
- 檔案/資料夾路徑支援拖拽、系統選擇器和手動輸入。
- 最近證據記錄。
- 證據包匯出。
- 英文、簡體中文、繁體中文。
- 托盤背景行為：關閉視窗後隱藏，可從托盤顯示或退出。

## GitHub 發布打包

`.github/workflows/release.yml` 會建置：

- Windows x64，使用 NSIS 安裝包。
- Linux x64，輸出 DEB 和 AppImage。

macOS 建置已主動關閉。GitHub 託管 runner 對 Linux/Windows 32 位安裝包沒有通用、穩定的一鍵矩陣。如果必須支援 32 位包，建議使用 self-hosted runner 或補充目標平台交叉編譯設定。

建立發布：

```bash
git tag v0.1.3
git push origin v0.1.3
```

## 授權

MIT，見 [LICENSE](../LICENSE)。
