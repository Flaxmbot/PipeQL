use clap::{Args, Parser as ClapParser, Subcommand};
use pipeql_core::PipeQLError;
use serde::Serialize;

#[derive(ClapParser)]
#[command(name = "pipeql")]
#[command(about = "PipeQL - pipelined & injection-safe polyglot query language compiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a PipeQL query into target-dialect SQL with extracted parameters.
    Compile(CompileArgs),
}

#[derive(Args)]
struct CompileArgs {
    /// The PipeQL query source (use quotes for multi-line input).
    query: String,
    /// Target dialect: postgres (default), sqlite, duckdb, mysql.
    #[arg(long, short, default_value = "postgres")]
    dialect: String,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
    /// Show extracted parameter names on stderr (default when not --json).
    #[arg(long)]
    no_params: bool,
}

#[derive(Serialize)]
struct JsonOutput {
    sql: String,
    params: Vec<String>,
    dialect: String,
    statement_type: String,
    is_mutation: bool,
    parameter_count: usize,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Compile(args) => run_compile(args),
    }
}

fn run_compile(args: CompileArgs) {
    match pipeql_core::api::compile(&args.query, &args.dialect) {
        Ok(compiled) => {
            if args.json {
                let out = JsonOutput {
                    sql: compiled.sql,
                    params: compiled.params.clone(),
                    dialect: args.dialect,
                    statement_type: compiled.statement_type.as_str().to_string(),
                    is_mutation: compiled.is_mutation,
                    parameter_count: compiled.params.len(),
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&out).expect("serializing output must not fail")
                );
            } else {
                println!("{}", compiled.sql);
                if !args.no_params && !compiled.params.is_empty() {
                    eprintln!("Parameters: {:?}", compiled.params);
                }
            }
        }
        Err(err) => {
            render_error(&err);
            std::process::exit(1);
        }
    }
}

fn render_error(err: &PipeQLError) {
    match err {
        PipeQLError::Parse(errs) => {
            for e in errs {
                eprintln!("{}", e.message);
                if let Some(s) = &e.suggestion {
                    eprintln!("  hint: {s}");
                }
            }
        }
        PipeQLError::Analysis(errs) => {
            for e in errs {
                eprintln!("{}", e.message);
                if let Some(s) = &e.suggestion {
                    eprintln!("  hint: {s}");
                }
            }
        }
        PipeQLError::Codegen(e) => eprintln!("{e}"),
    }
}
