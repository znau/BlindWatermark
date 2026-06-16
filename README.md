# Blind Watermark

**Language:** English | [简体中文](docs/README.zh-CN.md) | [繁體中文](docs/README.zh-TW.md)

Blind Watermark is a cross-platform, commercial-grade blind image watermarking toolkit for ownership protection, leakage tracing, and forensic evidence export. It combines a Rust watermarking engine, a CLI, and a Tauri 2 desktop application.

## Features

- DWT + DCT + SVD-QIM blind watermark embedding and extraction.
- Unique fingerprint ID per image or distribution batch.
- Batch processing for files and folders.
- SQLite evidence database with SHA-256, perceptual hash, PSNR, algorithm version, and ownership metadata.
- Evidence export as JSON, CSV, and PDF.
- Tauri desktop UI with English, Simplified Chinese, and Traditional Chinese.
- Cross-platform architecture for macOS, Windows, and Linux.
- GitHub Actions workflow for release builds.

## Architecture

- `crates/watermark-core`: Rust core library for image transforms, payload encoding, batch jobs, hashes, and evidence storage.
- `crates/watermark-cli`: CLI binary named `watermark`.
- `src-tauri`: Tauri 2 desktop shell and Rust commands.
- `ui/src`: React + TypeScript desktop UI.

## Requirements

- Rust stable toolchain.
- Node.js 20+.
- npm.
- Platform-specific Tauri system dependencies.

Linux builds require WebKitGTK and related libraries. On Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

## Development

Install dependencies:

```bash
npm install
```

Run the desktop app:

```bash
npm run tauri dev
```

The dev server starts at `127.0.0.1:1420` by default. If the port is occupied, `scripts/dev-server.mjs` automatically selects the next available port and updates Tauri's `devUrl`.

Build frontend assets:

```bash
npm run build
```

Run Rust tests:

```bash
cargo test --workspace
```

Build all Rust crates:

```bash
cargo build --workspace
```

## Recommended Defaults

If you are not sure what to choose, start with:

- `--profile balanced`: best first choice for most ownership-protection batches.
- `--quality 92`: good visual quality without producing unnecessarily large JPEG/WebP files.
- Leave `--strength` unset: the app will use the profile's tested default strength.
- `--output-format preserve`: keeps the original workflow simple. Use `png` when you want archival evidence outputs.
- Use a stable secret or key file and keep it safe. Extraction requires the same key.

Use `strong` only after testing if your images are often recompressed or resized. Use `fidelity` when visual quality is more important than recovery after attacks.

## CLI Usage

Embed watermarks:

```bash
cargo run -p watermark -- embed \
  --input ./images \
  --output ./out \
  --owner "Acme Studio" \
  --key ./secret.key \
  --profile balanced \
  --output-format png
```

Extract watermarks:

```bash
cargo run -p watermark -- extract \
  --input ./out \
  --key ./secret.key \
  --report ./extract-report.json
```

Verify a known watermark ID:

```bash
cargo run -p watermark -- verify \
  --input ./out/image.png \
  --watermark-id <uuid> \
  --key ./secret.key
```

Export evidence:

```bash
cargo run -p watermark -- evidence export \
  --batch-id <uuid> \
  --format json \
  --output ./evidence.json
```

The default local SQLite database is `watermark-evidence.sqlite` in the current directory. Use `--db <path>` to override it.

## Algorithm Notes

The first release embeds a compact unique watermark ID rather than long plaintext. Full ownership and distribution metadata are stored in SQLite. This improves robustness and keeps the image payload small.

Default pipeline:

1. Decode image and convert to YCbCr.
2. Embed into the luminance channel.
3. Apply Haar DWT and select mid-frequency subbands.
4. Apply 8x8 DCT on keyed pseudo-random blocks.
5. Use SVD-QIM to embed repeated payload bits.
6. Decode blindly using the same key and report confidence.

Blind watermarks are not magic. Heavy cropping, screenshots, aggressive recompression, or generative re-rendering can damage recoverability. Evidence confidence should be interpreted with the surrounding file hashes and metadata.

## Desktop App

The desktop app supports:

- Batch embed jobs.
- Batch extraction jobs.
- File/folder path input by drag and drop, native picker, or manual typing.
- Recent evidence records.
- Evidence export.
- English, Simplified Chinese, Traditional Chinese.
- Tray behavior: closing the window hides it; use the tray menu to show or quit.

## GitHub Release Builds

The workflow in `.github/workflows/release.yml` builds installers for:

- macOS x64.
- macOS arm64.
- Windows x64.
- Linux x64.

GitHub-hosted runners do not provide a practical universal matrix for every Linux/Windows 32-bit installer target. If 32-bit packages are required, use self-hosted runners or add target-specific cross-compilation setup.

Create a release by pushing a tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## License

MIT. See [LICENSE](LICENSE).
