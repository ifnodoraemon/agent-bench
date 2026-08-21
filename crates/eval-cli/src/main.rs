mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agent-bench")]
#[command(about = "A high-performance LLM & Agent Benchmark evaluation engine in Rust", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new benchmark configuration file interactively
    Init {
        /// Output configuration file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run benchmark evaluations on specified models
    Run {
        /// Path to configuration file (TOML)
        #[arg(short = 'c', long = "config")]
        config: Option<String>,

        /// Path to dataset files or directories (can specify multiple)
        #[arg(short, long, num_args = 1..)]
        dataset: Option<Vec<String>>,

        /// Filter test cases by category (foundation, agent, safety)
        #[arg(long)]
        category: Option<String>,

        /// Filter test cases by tag
        #[arg(long)]
        tag: Option<String>,

        /// Override models to run (comma-separated or multiple flags)
        #[arg(short = 'm', long = "models", num_args = 1..)]
        models: Option<Vec<String>>,

        /// Max concurrency (override config)
        #[arg(short = 'j', long = "concurrency")]
        concurrency: Option<usize>,

        /// Output directory for results (JSON, Markdown, HTML)
        #[arg(short, long)]
        output_dir: Option<String>,
    },

    /// Validate dataset JSON/JSONL format and schema integrity
    Validate {
        /// Paths to dataset files to validate
        #[arg(required = true, num_args = 1..)]
        files: Vec<String>,
    },

    /// Compare multiple benchmark run result JSONs and compute Elo ratings
    Compare {
        /// Paths to result JSON files to compare
        #[arg(required = true, num_args = 1..)]
        result_files: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { output } => {
            commands::init::execute_init(output)?;
        }
        Commands::Run {
            config,
            dataset,
            category,
            tag,
            models,
            concurrency,
            output_dir,
        } => {
            commands::run::execute_run(
                config,
                dataset.unwrap_or_default(),
                category,
                tag,
                models,
                concurrency,
                output_dir,
            )
            .await?;
        }
        Commands::Validate { files } => {
            commands::validate::execute_validate(files)?;
        }
        Commands::Compare { result_files } => {
            commands::compare::execute_compare(result_files)?;
        }
    }

    Ok(())
}
