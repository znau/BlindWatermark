import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {
  BadgeCheck,
  CheckCircle2,
  Database,
  File,
  Download,
  FileSearch,
  FolderOpen,
  Fingerprint,
  Globe2,
  History,
  KeyRound,
  Loader2,
  Play,
  UploadCloud,
  RefreshCcw,
  Settings,
  ShieldCheck,
  Sparkles,
  Info,
  XCircle,
} from "lucide-react";
import { FormEvent, ReactNode, useEffect, useMemo, useState } from "react";

type Tab = "embed" | "extract" | "history" | "settings";
type Locale = "en" | "zh-CN" | "zh-TW";
type Profile = "balanced" | "strong" | "fidelity";
type OutputFormat = "preserve" | "jpeg" | "png" | "webp";
type EvidenceFormat = "json" | "csv" | "pdf";
type PathPickerMode = "file" | "directory" | "save";

type BatchSummary = {
  batch_id?: string;
  batchId?: string;
  total: number;
  succeeded: number;
  failed: number;
  failures?: Array<{ path: string; error: string }>;
  results?: ExtractResult[];
};

type ExtractResult = {
  input_path?: string;
  inputPath?: string;
  watermark_id?: string | null;
  watermarkId?: string | null;
  confidence: number;
  ecc_status?: string;
  eccStatus?: string;
  detected_profile?: string | null;
  detectedProfile?: string | null;
  diagnostics: string[];
};

type EvidenceRecord = {
  id: number;
  batch_id?: string;
  batchId?: string;
  watermark_id?: string;
  watermarkId?: string;
  owner: string;
  output_path?: string;
  outputPath?: string;
  profile: string;
  psnr: number;
};

type AppInfo = {
  dbPath: string;
};

type Dictionary = typeof dictionaries.en;

const dictionaries = {
  en: {
    appName: "Blind Watermark",
    appTagline: "DWT + DCT + SVD image provenance",
    nav: {
      embed: "Embed",
      extract: "Extract",
      history: "Evidence",
      settings: "Export",
    },
    tabs: {
      embedTitle: "Batch watermark embedding",
      embedSubtitle: "Create invisible ownership fingerprints for image collections.",
      extractTitle: "Extraction and verification",
      extractSubtitle: "Recover watermark IDs and confidence for forensic evidence.",
      historyTitle: "Evidence history",
      historySubtitle: "Search recent local SQLite records and reuse batch IDs.",
      settingsTitle: "Export and settings",
      settingsSubtitle: "Export evidence packages and inspect local runtime paths.",
    },
    status: {
      ready: "Ready",
      embedRunning: "Embedding batch is running",
      embedDone: "Embedding completed",
      extractRunning: "Extraction batch is running",
      extractDone: "Extraction completed",
      exportRunning: "Exporting evidence package",
      exportDone: "Evidence package exported",
      waiting: "Waiting for a job",
      none: "None",
      notDetected: "Not detected",
    },
    labels: {
      input: "Input file or folder",
      output: "Output folder",
      owner: "Owner",
      key: "Secret or key file path",
      profile: "Robustness profile",
      outputFormat: "Output format",
      quality: "Quality",
      strength: "Strength override",
      target: "Target file or folder",
      batchId: "Batch ID",
      evidenceFormat: "Evidence format",
      exportPath: "Export path",
      database: "Evidence database",
      algorithm: "Algorithm",
      currentBatch: "Current batch",
      total: "Total",
      succeeded: "Succeeded",
      failed: "Failed",
      watermarkId: "Watermark ID",
      subject: "Owner",
      psnr: "PSNR",
      outputFile: "Output",
      recentRecords: "Recent evidence records",
      browse: "Browse",
      chooseFile: "Choose file",
      chooseFolder: "Choose folder",
      dropHere: "Drop a file or folder here",
      recommended: "Recommended defaults",
    },
    help: {
      input: "Choose one image or a folder. Batch mode scans JPG, PNG, WebP, and TIFF files.",
      output: "Use an empty or dedicated folder so original images stay untouched.",
      owner: "Shown in local evidence records. It is not embedded as plaintext in the image.",
      key: "Use the same secret when extracting. A short passphrase works, a key file is better.",
      profile: "Recommended: Balanced. Strong improves recovery after compression but may affect image fidelity.",
      outputFormat: "Recommended: PNG for archival evidence, Preserve for normal batch workflows.",
      quality: "Recommended: 92. Higher keeps more visual quality; lower creates smaller JPEG/WebP files.",
      strength: "Recommended: leave blank. Only set this if you are tuning robustness after testing.",
      target: "Choose a suspicious image or a folder of images to detect embedded IDs.",
      exportPath: "Choose a JSON, CSV, or PDF destination for the evidence package.",
      evidenceFormat: "Recommended: JSON for complete data, PDF for human-readable sharing, CSV for spreadsheets.",
      defaultsOne: "Profile: Balanced",
      defaultsTwo: "Quality: 92",
      defaultsThree: "Strength override: blank",
      defaultsFour: "Output format: Preserve, or PNG for archival evidence",
    },
    actions: {
      startEmbed: "Start embedding",
      startExtract: "Start extraction",
      exportEvidence: "Export evidence",
      refresh: "Refresh",
    },
    profile: {
      balanced: "Balanced",
      strong: "Strong",
      fidelity: "Fidelity",
    },
    format: {
      preserve: "Preserve",
      jpeg: "JPG",
      png: "PNG",
      webp: "WebP",
    },
    insight: {
      title: "Commercial evidence chain",
      one: "Unique fingerprint ID is embedded into each image.",
      two: "Ownership metadata stays in SQLite instead of the image payload.",
      three: "Extraction returns confidence, ECC state, and diagnostics.",
    },
  },
  "zh-CN": {
    appName: "Blind Watermark",
    appTagline: "DWT + DCT + SVD 图片产权取证",
    nav: { embed: "嵌入", extract: "提取", history: "证据", settings: "导出" },
    tabs: {
      embedTitle: "批量水印嵌入",
      embedSubtitle: "为图片集合创建不可见产权指纹。",
      extractTitle: "提取与验证",
      extractSubtitle: "恢复水印 ID、置信度和取证诊断。",
      historyTitle: "证据历史",
      historySubtitle: "查看本地 SQLite 记录并复用批次 ID。",
      settingsTitle: "导出与设置",
      settingsSubtitle: "导出证据包并检查本地运行路径。",
    },
    status: {
      ready: "就绪",
      embedRunning: "嵌入任务运行中",
      embedDone: "嵌入任务完成",
      extractRunning: "提取任务运行中",
      extractDone: "提取任务完成",
      exportRunning: "证据包导出中",
      exportDone: "证据包已导出",
      waiting: "等待任务",
      none: "无",
      notDetected: "未检出",
    },
    labels: {
      input: "输入文件或目录",
      output: "输出目录",
      owner: "产权主体",
      key: "密钥或密钥文件路径",
      profile: "鲁棒策略",
      outputFormat: "输出格式",
      quality: "质量",
      strength: "强度覆盖",
      target: "待检文件或目录",
      batchId: "Batch ID",
      evidenceFormat: "证据格式",
      exportPath: "导出路径",
      database: "证据库",
      algorithm: "默认算法",
      currentBatch: "当前批次",
      total: "总数",
      succeeded: "成功",
      failed: "失败",
      watermarkId: "水印 ID",
      subject: "主体",
      psnr: "PSNR",
      outputFile: "输出",
      recentRecords: "最近证据记录",
      browse: "选择",
      chooseFile: "选择文件",
      chooseFolder: "选择目录",
      dropHere: "拖拽文件或目录到这里",
      recommended: "推荐默认参数",
    },
    help: {
      input: "选择单张图片或一个目录。批量模式会扫描 JPG、PNG、WebP、TIFF。",
      output: "建议使用空目录或专用目录，避免覆盖原图。",
      owner: "会写入本地证据记录，不会以明文塞进图片水印载荷。",
      key: "提取时必须使用同一个密钥。短口令可用，密钥文件更稳妥。",
      profile: "推荐：均衡。强鲁棒更抗压缩，但可能略微影响画质。",
      outputFormat: "推荐：普通批量用保持；归档取证可用 PNG。",
      quality: "推荐：92。数值越高画质越好；越低 JPEG/WebP 文件越小。",
      strength: "推荐：留空。只有在做鲁棒性调参测试时才填写。",
      target: "选择疑似盗用图片或图片目录，用于检测水印 ID。",
      exportPath: "选择 JSON、CSV 或 PDF 证据包的保存路径。",
      evidenceFormat: "推荐：JSON 保存完整数据，PDF 便于人工查看，CSV 适合表格分析。",
      defaultsOne: "鲁棒策略：均衡",
      defaultsTwo: "质量：92",
      defaultsThree: "强度覆盖：留空",
      defaultsFour: "输出格式：保持；归档取证可选 PNG",
    },
    actions: {
      startEmbed: "开始嵌入",
      startExtract: "开始提取",
      exportEvidence: "导出证据包",
      refresh: "刷新",
    },
    profile: { balanced: "均衡", strong: "强鲁棒", fidelity: "高保真" },
    format: { preserve: "保持", jpeg: "JPG", png: "PNG", webp: "WebP" },
    insight: {
      title: "商业取证链路",
      one: "每张图片嵌入唯一指纹 ID。",
      two: "完整产权信息保存在 SQLite，不塞进图片载荷。",
      three: "提取结果包含置信度、纠错状态和诊断信息。",
    },
  },
  "zh-TW": {
    appName: "Blind Watermark",
    appTagline: "DWT + DCT + SVD 圖片產權取證",
    nav: { embed: "嵌入", extract: "提取", history: "證據", settings: "匯出" },
    tabs: {
      embedTitle: "批次浮水印嵌入",
      embedSubtitle: "為圖片集合建立不可見產權指紋。",
      extractTitle: "提取與驗證",
      extractSubtitle: "恢復浮水印 ID、信賴度與取證診斷。",
      historyTitle: "證據歷史",
      historySubtitle: "檢視本機 SQLite 記錄並重用批次 ID。",
      settingsTitle: "匯出與設定",
      settingsSubtitle: "匯出證據包並檢查本機執行路徑。",
    },
    status: {
      ready: "就緒",
      embedRunning: "嵌入任務執行中",
      embedDone: "嵌入任務完成",
      extractRunning: "提取任務執行中",
      extractDone: "提取任務完成",
      exportRunning: "證據包匯出中",
      exportDone: "證據包已匯出",
      waiting: "等待任務",
      none: "無",
      notDetected: "未檢出",
    },
    labels: {
      input: "輸入檔案或資料夾",
      output: "輸出資料夾",
      owner: "產權主體",
      key: "密鑰或密鑰檔案路徑",
      profile: "魯棒策略",
      outputFormat: "輸出格式",
      quality: "品質",
      strength: "強度覆寫",
      target: "待檢檔案或資料夾",
      batchId: "Batch ID",
      evidenceFormat: "證據格式",
      exportPath: "匯出路徑",
      database: "證據庫",
      algorithm: "預設演算法",
      currentBatch: "目前批次",
      total: "總數",
      succeeded: "成功",
      failed: "失敗",
      watermarkId: "浮水印 ID",
      subject: "主體",
      psnr: "PSNR",
      outputFile: "輸出",
      recentRecords: "最近證據記錄",
      browse: "選擇",
      chooseFile: "選擇檔案",
      chooseFolder: "選擇資料夾",
      dropHere: "拖拽檔案或資料夾到這裡",
      recommended: "建議預設參數",
    },
    help: {
      input: "選擇單張圖片或一個資料夾。批次模式會掃描 JPG、PNG、WebP、TIFF。",
      output: "建議使用空資料夾或專用資料夾，避免覆蓋原圖。",
      owner: "會寫入本機證據記錄，不會以明文塞進圖片浮水印載荷。",
      key: "提取時必須使用同一個密鑰。短口令可用，密鑰檔案更穩妥。",
      profile: "建議：均衡。強魯棒更抗壓縮，但可能略微影響畫質。",
      outputFormat: "建議：普通批次用保留；歸檔取證可用 PNG。",
      quality: "建議：92。數值越高畫質越好；越低 JPEG/WebP 檔案越小。",
      strength: "建議：留空。只有在做魯棒性調參測試時才填寫。",
      target: "選擇疑似盜用圖片或圖片資料夾，用於檢測浮水印 ID。",
      exportPath: "選擇 JSON、CSV 或 PDF 證據包的儲存路徑。",
      evidenceFormat: "建議：JSON 保存完整資料，PDF 便於人工查看，CSV 適合表格分析。",
      defaultsOne: "魯棒策略：均衡",
      defaultsTwo: "品質：92",
      defaultsThree: "強度覆寫：留空",
      defaultsFour: "輸出格式：保留；歸檔取證可選 PNG",
    },
    actions: {
      startEmbed: "開始嵌入",
      startExtract: "開始提取",
      exportEvidence: "匯出證據包",
      refresh: "重新整理",
    },
    profile: { balanced: "均衡", strong: "強魯棒", fidelity: "高保真" },
    format: { preserve: "保留", jpeg: "JPG", png: "PNG", webp: "WebP" },
    insight: {
      title: "商業取證鏈路",
      one: "每張圖片嵌入唯一指紋 ID。",
      two: "完整產權資訊保存在 SQLite，不塞入圖片載荷。",
      three: "提取結果包含信賴度、糾錯狀態與診斷資訊。",
    },
  },
} satisfies Record<Locale, object>;

function App() {
  const [locale, setLocale] = useState<Locale>(() => (localStorage.getItem("bw-locale") as Locale) || "en");
  const t = dictionaries[locale] as Dictionary;
  const [tab, setTab] = useState<Tab>("embed");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [records, setRecords] = useState<EvidenceRecord[]>([]);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string>(t.status.ready);
  const [lastSummary, setLastSummary] = useState<BatchSummary | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);

  const [embedForm, setEmbedForm] = useState({
    input: "",
    output: "",
    owner: "",
    key: "",
    profile: "balanced" as Profile,
    outputFormat: "preserve" as OutputFormat,
    quality: 92,
    strength: "",
  });
  const [extractForm, setExtractForm] = useState({ input: "", key: "" });
  const [exportForm, setExportForm] = useState({ batchId: "", format: "json" as EvidenceFormat, output: "" });

  useEffect(() => {
    localStorage.setItem("bw-locale", locale);
  }, [locale]);

  useEffect(() => {
    void loadAppInfo();
    void loadRecords();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          return;
        }
        if (event.payload.type === "leave") {
          setDropTarget(null);
          return;
        }
        if (event.payload.type === "drop") {
          const [path] = event.payload.paths;
          if (path && dropTarget) {
            assignPath(dropTarget, path);
          }
          setDropTarget(null);
        }
      })
      .then((dispose) => {
        unlisten = dispose;
      });
    return () => unlisten?.();
  }, [dropTarget]);

  const lastBatchId = useMemo(() => batchIdOf(lastSummary), [lastSummary]);

  async function loadAppInfo() {
    const info = await invoke<AppInfo>("app_info");
    setAppInfo(info);
  }

  async function loadRecords() {
    const next = await invoke<EvidenceRecord[]>("recent_records", { limit: 50 });
    setRecords(next);
  }

  function assignPath(target: string, path: string) {
    if (target === "embed.input") setEmbedForm((current) => ({ ...current, input: path }));
    if (target === "embed.output") setEmbedForm((current) => ({ ...current, output: path }));
    if (target === "embed.key") setEmbedForm((current) => ({ ...current, key: path }));
    if (target === "extract.input") setExtractForm((current) => ({ ...current, input: path }));
    if (target === "extract.key") setExtractForm((current) => ({ ...current, key: path }));
    if (target === "export.output") setExportForm((current) => ({ ...current, output: path }));
  }

  async function pickPath(target: string, mode: PathPickerMode, title: string) {
    const selected =
      mode === "save"
        ? await save({ title })
        : await open({
            title,
            directory: mode === "directory",
            multiple: false,
          });
    if (typeof selected === "string") {
      assignPath(target, selected);
    }
  }

  async function submitEmbed(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setStatus(t.status.embedRunning);
    try {
      const summary = await invoke<BatchSummary>("embed_batch", {
        request: {
          input: embedForm.input,
          output: embedForm.output,
          owner: embedForm.owner,
          key: embedForm.key,
          profile: embedForm.profile,
          outputFormat: embedForm.outputFormat,
          quality: embedForm.quality,
          strength: embedForm.strength ? Number(embedForm.strength) : null,
        },
      });
      setLastSummary(summary);
      setExportForm((current) => ({ ...current, batchId: batchIdOf(summary) ?? current.batchId }));
      setStatus(t.status.embedDone);
      await loadRecords();
    } catch (error) {
      setStatus(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function submitExtract(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setStatus(t.status.extractRunning);
    try {
      const summary = await invoke<BatchSummary>("extract_batch", { request: extractForm });
      setLastSummary(summary);
      setStatus(t.status.extractDone);
    } catch (error) {
      setStatus(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function submitExport(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setStatus(t.status.exportRunning);
    try {
      await invoke("export_evidence", {
        request: {
          batchId: exportForm.batchId || lastBatchId || "",
          format: exportForm.format,
          output: exportForm.output,
        },
      });
      setStatus(t.status.exportDone);
    } catch (error) {
      setStatus(String(error));
    } finally {
      setBusy(false);
    }
  }

  const profileOptions = [
    { value: "balanced" as Profile, label: t.profile.balanced },
    { value: "strong" as Profile, label: t.profile.strong },
    { value: "fidelity" as Profile, label: t.profile.fidelity },
  ];
  const outputFormatOptions = [
    { value: "preserve" as OutputFormat, label: t.format.preserve },
    { value: "jpeg" as OutputFormat, label: t.format.jpeg },
    { value: "png" as OutputFormat, label: t.format.png },
    { value: "webp" as OutputFormat, label: t.format.webp },
  ];

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <img src="/app-icon.png" alt="" />
          <div>
            <span>{t.appName}</span>
            <small>{t.appTagline}</small>
          </div>
        </div>
        <nav className="nav">
          <NavButton active={tab === "embed"} icon={<Play />} label={t.nav.embed} onClick={() => setTab("embed")} />
          <NavButton active={tab === "extract"} icon={<FileSearch />} label={t.nav.extract} onClick={() => setTab("extract")} />
          <NavButton active={tab === "history"} icon={<History />} label={t.nav.history} onClick={() => setTab("history")} />
          <NavButton active={tab === "settings"} icon={<Settings />} label={t.nav.settings} onClick={() => setTab("settings")} />
        </nav>
        <div className="sidebar-card">
          <Sparkles aria-hidden />
          <strong>{t.insight.title}</strong>
          <span>{t.insight.one}</span>
          <span>{t.insight.two}</span>
          <span>{t.insight.three}</span>
        </div>
        <div className="runtime">
          <span className={busy ? "pulse" : "dot"} />
          <span>{status}</span>
        </div>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">DWT + DCT + SVD-QIM</p>
            <h1>{titleFor(tab, t)}</h1>
            <p>{subtitleFor(tab, t)}</p>
          </div>
          <div className="top-actions">
            <label className="locale-switch">
              <Globe2 aria-hidden />
              <select value={locale} onChange={(event) => setLocale(event.target.value as Locale)}>
                <option value="en">English</option>
                <option value="zh-CN">简体中文</option>
                <option value="zh-TW">繁體中文</option>
              </select>
            </label>
            <button className="icon-button" onClick={() => void loadRecords()} title={t.actions.refresh}>
              <RefreshCcw aria-hidden />
            </button>
          </div>
        </header>

        <section className="status-strip">
          <Metric icon={<Fingerprint />} label={t.labels.total} value={lastSummary?.total ?? 0} />
          <Metric icon={<BadgeCheck />} label={t.labels.succeeded} value={lastSummary?.succeeded ?? 0} />
          <Metric icon={<XCircle />} label={t.labels.failed} value={lastSummary?.failed ?? 0} />
          <Metric icon={<Database />} label={t.labels.currentBatch} value={shortId(lastBatchId) || t.status.none} compact />
        </section>

        {tab === "embed" && (
          <div className="grid two">
            <form className="panel form" onSubmit={submitEmbed}>
              <PanelTitle icon={<ShieldCheck />} title={t.tabs.embedTitle} />
              <PathField
                label={t.labels.input}
                value={embedForm.input}
                onChange={(input) => setEmbedForm({ ...embedForm, input })}
                onPick={() => void pickPath("embed.input", "file", t.labels.input)}
                onPickDirectory={() => void pickPath("embed.input", "directory", t.labels.input)}
                onDropTarget={(active) => setDropTarget(active ? "embed.input" : null)}
                help={t.help.input}
                t={t}
              />
              <PathField
                label={t.labels.output}
                value={embedForm.output}
                onChange={(output) => setEmbedForm({ ...embedForm, output })}
                onPick={() => void pickPath("embed.output", "directory", t.labels.output)}
                onDropTarget={(active) => setDropTarget(active ? "embed.output" : null)}
                directoryOnly
                help={t.help.output}
                t={t}
              />
              <Field label={t.labels.owner} value={embedForm.owner} onChange={(owner) => setEmbedForm({ ...embedForm, owner })} help={t.help.owner} />
              <PathField
                label={t.labels.key}
                value={embedForm.key}
                onChange={(key) => setEmbedForm({ ...embedForm, key })}
                onPick={() => void pickPath("embed.key", "file", t.labels.key)}
                onDropTarget={(active) => setDropTarget(active ? "embed.key" : null)}
                help={t.help.key}
                t={t}
              />
              <Segmented label={t.labels.profile} value={embedForm.profile} options={profileOptions} onChange={(profile) => setEmbedForm({ ...embedForm, profile })} help={t.help.profile} />
              <Segmented label={t.labels.outputFormat} value={embedForm.outputFormat} options={outputFormatOptions} onChange={(outputFormat) => setEmbedForm({ ...embedForm, outputFormat })} help={t.help.outputFormat} />
              <div className="inline-fields">
                <NumberField label={t.labels.quality} value={embedForm.quality} min={50} max={100} onChange={(quality) => setEmbedForm({ ...embedForm, quality })} help={t.help.quality} />
                <Field label={t.labels.strength} value={embedForm.strength} onChange={(strength) => setEmbedForm({ ...embedForm, strength })} help={t.help.strength} />
              </div>
              <button className="primary" disabled={busy}>
                {busy ? <Loader2 className="spin" aria-hidden /> : <Play aria-hidden />}
                {t.actions.startEmbed}
              </button>
            </form>
            <div className="side-stack">
              <DefaultsPanel t={t} />
              <SummaryPanel summary={lastSummary} t={t} />
            </div>
          </div>
        )}

        {tab === "extract" && (
          <div className="grid two">
            <form className="panel form" onSubmit={submitExtract}>
              <PanelTitle icon={<FileSearch />} title={t.tabs.extractTitle} />
              <PathField
                label={t.labels.target}
                value={extractForm.input}
                onChange={(input) => setExtractForm({ ...extractForm, input })}
                onPick={() => void pickPath("extract.input", "file", t.labels.target)}
                onPickDirectory={() => void pickPath("extract.input", "directory", t.labels.target)}
                onDropTarget={(active) => setDropTarget(active ? "extract.input" : null)}
                help={t.help.target}
                t={t}
              />
              <PathField
                label={t.labels.key}
                value={extractForm.key}
                onChange={(key) => setExtractForm({ ...extractForm, key })}
                onPick={() => void pickPath("extract.key", "file", t.labels.key)}
                onDropTarget={(active) => setDropTarget(active ? "extract.key" : null)}
                help={t.help.key}
                t={t}
              />
              <button className="primary" disabled={busy}>
                {busy ? <Loader2 className="spin" aria-hidden /> : <FileSearch aria-hidden />}
                {t.actions.startExtract}
              </button>
            </form>
            <SummaryPanel summary={lastSummary} t={t} />
          </div>
        )}

        {tab === "history" && (
          <div className="panel">
            <div className="panel-toolbar">
              <PanelTitle icon={<History />} title={t.labels.recentRecords} />
              <button className="secondary" onClick={() => void loadRecords()}>
                <RefreshCcw aria-hidden />
                {t.actions.refresh}
              </button>
            </div>
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>ID</th>
                    <th>{t.labels.watermarkId}</th>
                    <th>{t.labels.subject}</th>
                    <th>Profile</th>
                    <th>{t.labels.psnr}</th>
                    <th>{t.labels.outputFile}</th>
                  </tr>
                </thead>
                <tbody>
                  {records.map((record) => (
                    <tr key={record.id} onClick={() => setExportForm({ ...exportForm, batchId: batchIdRecord(record) ?? "" })}>
                      <td>{record.id}</td>
                      <td className="mono">{shortId(watermarkIdRecord(record))}</td>
                      <td>{record.owner}</td>
                      <td>{record.profile}</td>
                      <td>{Number(record.psnr).toFixed(2)}</td>
                      <td className="path">{outputPathRecord(record)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {tab === "settings" && (
          <div className="grid two">
            <form className="panel form" onSubmit={submitExport}>
              <PanelTitle icon={<Download />} title={t.tabs.settingsTitle} />
              <Field label={t.labels.batchId} value={exportForm.batchId || lastBatchId || ""} onChange={(batchId) => setExportForm({ ...exportForm, batchId })} />
              <Segmented
                label={t.labels.evidenceFormat}
                value={exportForm.format}
                options={[
                  { value: "json", label: "JSON" },
                  { value: "csv", label: "CSV" },
                  { value: "pdf", label: "PDF" },
                ]}
                onChange={(format) => setExportForm({ ...exportForm, format })}
                help={t.help.evidenceFormat}
              />
              <PathField
                label={t.labels.exportPath}
                value={exportForm.output}
                onChange={(output) => setExportForm({ ...exportForm, output })}
                onPick={() => void pickPath("export.output", "save", t.labels.exportPath)}
                onDropTarget={(active) => setDropTarget(active ? "export.output" : null)}
                help={t.help.exportPath}
                t={t}
              />
              <button className="primary" disabled={busy}>
                {busy ? <Loader2 className="spin" aria-hidden /> : <Download aria-hidden />}
                {t.actions.exportEvidence}
              </button>
            </form>
            <div className="panel kv">
              <InfoItem icon={<Database />} label={t.labels.database} value={appInfo?.dbPath ?? ""} />
              <InfoItem icon={<Fingerprint />} label={t.labels.algorithm} value="dwt-dct-svd-qim-v1" />
              <InfoItem icon={<KeyRound />} label={t.labels.currentBatch} value={lastBatchId ?? t.status.none} />
            </div>
          </div>
        )}
      </section>
    </main>
  );
}

function NavButton({ active, icon, label, onClick }: { active: boolean; icon: ReactNode; label: string; onClick: () => void }) {
  return (
    <button className={active ? "nav-button active" : "nav-button"} onClick={onClick}>
      {icon}
      <span>{label}</span>
    </button>
  );
}

function PanelTitle({ icon, title }: { icon: ReactNode; title: string }) {
  return (
    <div className="panel-title">
      {icon}
      <strong>{title}</strong>
    </div>
  );
}

function Field({ label, value, onChange, help }: { label: string; value: string; onChange: (value: string) => void; help?: string }) {
  return (
    <label className="field">
      <span>{label}</span>
      <input value={value} onChange={(event) => onChange(event.target.value)} />
      {help && <HelpText>{help}</HelpText>}
    </label>
  );
}

function PathField({
  label,
  value,
  onChange,
  onPick,
  onPickDirectory,
  onDropTarget,
  directoryOnly,
  help,
  t,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onPick: () => void;
  onPickDirectory?: () => void;
  onDropTarget: (active: boolean) => void;
  directoryOnly?: boolean;
  help?: string;
  t: Dictionary;
}) {
  const [dragActive, setDragActive] = useState(false);
  return (
    <label
      className={dragActive ? "field path-field drag-active" : "field path-field"}
      onDragEnter={() => {
        setDragActive(true);
        onDropTarget(true);
      }}
      onDragOver={(event) => {
        event.preventDefault();
        onDropTarget(true);
      }}
      onDragLeave={() => {
        setDragActive(false);
        onDropTarget(false);
      }}
      onDrop={() => {
        setDragActive(false);
        onDropTarget(false);
      }}
    >
      <span>{label}</span>
      <div className="path-input-row">
        <input value={value} onChange={(event) => onChange(event.target.value)} />
        {!directoryOnly && onPickDirectory && (
          <button type="button" className="path-action" onClick={onPick} title={t.labels.chooseFile}>
            <File aria-hidden />
          </button>
        )}
        <button type="button" className="path-action" onClick={directoryOnly ? onPick : (onPickDirectory ?? onPick)} title={directoryOnly ? t.labels.chooseFolder : t.labels.chooseFolder}>
          <FolderOpen aria-hidden />
        </button>
      </div>
      <small>
        <UploadCloud aria-hidden />
        {t.labels.dropHere}
      </small>
      {help && <HelpText>{help}</HelpText>}
    </label>
  );
}

function NumberField({ label, value, min, max, onChange, help }: { label: string; value: number; min: number; max: number; onChange: (value: number) => void; help?: string }) {
  return (
    <label className="field">
      <span>{label}</span>
      <input type="number" min={min} max={max} value={value} onChange={(event) => onChange(Number(event.target.value))} />
      {help && <HelpText>{help}</HelpText>}
    </label>
  );
}

function Segmented<T extends string>({ label, value, options, onChange, help }: { label: string; value: T; options: Array<{ value: T; label: string }>; onChange: (value: T) => void; help?: string }) {
  return (
    <div className="segmented-field">
      <span>{label}</span>
      <div className="segmented">
        {options.map((option) => (
          <button key={option.value} type="button" className={value === option.value ? "selected" : ""} onClick={() => onChange(option.value)}>
            {option.label}
          </button>
        ))}
      </div>
      {help && <HelpText>{help}</HelpText>}
    </div>
  );
}

function HelpText({ children }: { children: ReactNode }) {
  return (
    <p className="help-text">
      <Info aria-hidden />
      <span>{children}</span>
    </p>
  );
}

function DefaultsPanel({ t }: { t: Dictionary }) {
  return (
    <div className="panel defaults-panel">
      <PanelTitle icon={<Info />} title={t.labels.recommended} />
      <ul>
        <li>{t.help.defaultsOne}</li>
        <li>{t.help.defaultsTwo}</li>
        <li>{t.help.defaultsThree}</li>
        <li>{t.help.defaultsFour}</li>
      </ul>
    </div>
  );
}

function SummaryPanel({ summary, t }: { summary: BatchSummary | null; t: Dictionary }) {
  const results = summary?.results ?? [];
  return (
    <div className="panel summary">
      <PanelTitle icon={<CheckCircle2 />} title={t.labels.currentBatch} />
      <div className="summary-status">
        {(summary?.failed ?? 0) > 0 ? <XCircle aria-hidden /> : <CheckCircle2 aria-hidden />}
        <span>{batchIdOf(summary) ?? t.status.waiting}</span>
      </div>
      {results.length > 0 && (
        <div className="result-list">
          {results.slice(0, 10).map((result, index) => (
            <div className="result-item" key={`${inputPathResult(result)}-${index}`}>
              <span className="mono">{shortId(watermarkIdResult(result)) || t.status.notDetected}</span>
              <strong>{Math.round(result.confidence * 100)}%</strong>
              <span>{inputPathResult(result)}</span>
            </div>
          ))}
        </div>
      )}
      {(summary?.failures ?? []).length > 0 && (
        <div className="failures">
          {summary?.failures?.slice(0, 6).map((failure, index) => (
            <div key={index}>
              <strong>{String(failure.path)}</strong>
              <span>{failure.error}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Metric({ icon, label, value, compact }: { icon: ReactNode; label: string; value: number | string; compact?: boolean }) {
  return (
    <div className={compact ? "metric compact" : "metric"}>
      <div>{icon}</div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function InfoItem({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div>
      {icon}
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function titleFor(tab: Tab, t: Dictionary) {
  return {
    embed: t.tabs.embedTitle,
    extract: t.tabs.extractTitle,
    history: t.tabs.historyTitle,
    settings: t.tabs.settingsTitle,
  }[tab];
}

function subtitleFor(tab: Tab, t: Dictionary) {
  return {
    embed: t.tabs.embedSubtitle,
    extract: t.tabs.extractSubtitle,
    history: t.tabs.historySubtitle,
    settings: t.tabs.settingsSubtitle,
  }[tab];
}

function batchIdOf(summary: BatchSummary | null) {
  return summary?.batch_id ?? summary?.batchId ?? null;
}

function batchIdRecord(record: EvidenceRecord) {
  return record.batch_id ?? record.batchId;
}

function watermarkIdRecord(record: EvidenceRecord) {
  return record.watermark_id ?? record.watermarkId;
}

function outputPathRecord(record: EvidenceRecord) {
  return record.output_path ?? record.outputPath ?? "";
}

function watermarkIdResult(result: ExtractResult) {
  return result.watermark_id ?? result.watermarkId ?? null;
}

function inputPathResult(result: ExtractResult) {
  return result.input_path ?? result.inputPath ?? "";
}

function shortId(value?: string | null) {
  if (!value) return "";
  return `${value.slice(0, 8)}...${value.slice(-6)}`;
}

export default App;
