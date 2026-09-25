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

        /// Filter test cases by category (foundation, agent, safety, etc.)
        #[arg(long)]
        category: Option<String>,

        /// Filter test cases by difficulty (L1, L2, L3, L4, L5, or comma-separated e.g. L4,L5)
        #[arg(short = 'D', long = "difficulty")]
        difficulty: Option<String>,

        /// Filter only extreme/frontier test cases (L4 & L5 Olympiad, PhD, SWE-Hard)
        #[arg(long = "frontier", default_value_t = false)]
        frontier: bool,

        /// Filter test cases by evaluation type (exact_match, regex, json_schema, code_execution, llm_judge, agent_trajectory)
        #[arg(long = "eval-type")]
        eval_type: Option<String>,

        /// Filter test cases by tag
        #[arg(long)]
        tag: Option<String>,

        /// Limit the maximum number of test cases to run
        #[arg(short = 'l', long = "limit")]
        limit: Option<usize>,

        /// Randomly sample a ratio of test cases (0.0 to 1.0)
        #[arg(long = "sample-ratio")]
        sample_ratio: Option<f64>,

        /// Random seed for deterministic test case sampling
        #[arg(long = "seed")]
        seed: Option<u64>,

        /// Resume and only run test cases that failed or errored in a previous results JSON
        #[arg(long = "resume-failed")]
        resume_failed: Option<String>,

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
            difficulty,
            frontier,
            eval_type,
            tag,
            limit,
            sample_ratio,
            seed,
            resume_failed,
            models,
            concurrency,
            output_dir,
        } => {
            commands::run::execute_run(
                config,
                dataset.unwrap_or_default(),
                category,
                difficulty,
                frontier,
                eval_type,
                tag,
                limit,
                sample_ratio,
                seed,
                resume_failed,
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
