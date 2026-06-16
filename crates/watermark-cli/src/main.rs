use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use watermark_core::{
    batch_embed, batch_extract, export_batch_records, verify_file, AlgorithmProfile, EmbedOptions,
    EvidenceExportFormat, EvidenceStore, OutputFormat,
};

const DEFAULT_DB: &str = "watermark-evidence.sqlite";

#[derive(Debug, Parser)]
#[command(name = "watermark")]
#[command(about = "Blind image watermarking toolkit")]
struct Cli {
    #[arg(long, global = true, default_value = DEFAULT_DB)]
    db: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Embed(EmbedCommand),
    Extract(ExtractCommand),
    Verify(VerifyCommand),
    Evidence(EvidenceCommand),
}

#[derive(Debug, Args)]
struct EmbedCommand {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    owner: String,
    #[arg(long, default_value = "balanced")]
    profile: ProfileArg,
    #[arg(long)]
    key: String,
    #[arg(long)]
    strength: Option<f32>,
    #[arg(long, default_value = "preserve")]
    output_format: OutputFormatArg,
    #[arg(long, default_value_t = 92)]
    quality: u8,
}

#[derive(Debug, Args)]
struct ExtractCommand {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    key: String,
    #[arg(long)]
    report: PathBuf,
}

#[derive(Debug, Args)]
struct VerifyCommand {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    watermark_id: Uuid,
    #[arg(long)]
    key: String,
}

#[derive(Debug, Args)]
struct EvidenceCommand {
    #[command(subcommand)]
    command: EvidenceSubcommand,
}

#[derive(Debug, Subcommand)]
enum EvidenceSubcommand {
    Export(EvidenceExportCommand),
    Recent(EvidenceRecentCommand),
}

#[derive(Debug, Args)]
struct EvidenceExportCommand {
    #[arg(long)]
    batch_id: Uuid,
    #[arg(long)]
    format: EvidenceFormatArg,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct EvidenceRecentCommand {
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Clone, Debug, ValueEnum)]
enum ProfileArg {
    Fidelity,
    Balanced,
    Strong,
}

impl From<ProfileArg> for AlgorithmProfile {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Fidelity => Self::Fidelity,
            ProfileArg::Balanced => Self::Balanced,
            ProfileArg::Strong => Self::Strong,
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormatArg {
    Preserve,
    Jpeg,
    Png,
    Webp,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(value: OutputFormatArg) -> Self {
        match value {
            OutputFormatArg::Preserve => Self::Preserve,
            OutputFormatArg::Jpeg => Self::Jpeg,
            OutputFormatArg::Png => Self::Png,
            OutputFormatArg::Webp => Self::Webp,
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
enum EvidenceFormatArg {
    Json,
    Csv,
    Pdf,
}

impl From<EvidenceFormatArg> for EvidenceExportFormat {
    fn from(value: EvidenceFormatArg) -> Self {
        match value {
            EvidenceFormatArg::Json => Self::Json,
            EvidenceFormatArg::Csv => Self::Csv,
            EvidenceFormatArg::Pdf => Self::Pdf,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Embed(command) => run_embed(cli.db, command),
        Commands::Extract(command) => run_extract(cli.db, command),
        Commands::Verify(command) => run_verify(command),
        Commands::Evidence(command) => run_evidence(cli.db, command),
    }
}

fn run_embed(db: PathBuf, command: EmbedCommand) -> Result<()> {
    let store = EvidenceStore::open(db)?;
    let key = load_key(&command.key)?;
    let options = EmbedOptions {
        profile: command.profile.into(),
        strength: command.strength,
        output_format: command.output_format.into(),
        quality: command.quality,
    };
    let summary = batch_embed(
        command.input,
        command.output,
        &command.owner,
        &key,
        &options,
        &store,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn run_extract(db: PathBuf, command: ExtractCommand) -> Result<()> {
    let store = EvidenceStore::open(db)?;
    let key = load_key(&command.key)?;
    let summary = batch_extract(command.input, &key, &store)?;
    if let Some(parent) = command.report.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&command.report, serde_json::to_vec_pretty(&summary)?)
        .with_context(|| format!("write {}", command.report.display()))?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn run_verify(command: VerifyCommand) -> Result<()> {
    let key = load_key(&command.key)?;
    let result = verify_file(command.input, command.watermark_id, &key)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn run_evidence(db: PathBuf, command: EvidenceCommand) -> Result<()> {
    let store = EvidenceStore::open(db)?;
    match command.command {
        EvidenceSubcommand::Export(command) => {
            export_batch_records(
                &store,
                command.batch_id,
                command.format.into(),
                command.output,
            )?;
        }
        EvidenceSubcommand::Recent(command) => {
            let records = store.recent_records(command.limit)?;
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
    }
    Ok(())
}

fn load_key(key_ref: &str) -> Result<Vec<u8>> {
    let path = Path::new(key_ref);
    if path.exists() {
        return fs::read(path).with_context(|| format!("read key file {}", path.display()));
    }
    Ok(key_ref.as_bytes().to_vec())
}
