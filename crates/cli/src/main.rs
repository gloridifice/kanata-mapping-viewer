use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// Convert kanata .kbd configuration files to HTML visualization
#[derive(Parser)]
#[command(name = "kanata-mapping-viewer")]
struct Cli {
    /// Path to the .kbd input file
    input: PathBuf,

    /// Output HTML file path (writes to stdout if not specified)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Target platform (win, linux, or macos)
    #[arg(long = "platform", default_value = "win")]
    platform: String,

    /// Enable developer mode (don't use embedded css, use external css)
    #[arg(short = 'd', long = "dev")]
    dev: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match kanata_mapping_viewer_core::render_file(&cli.input, &cli.platform, cli.dev) {
        Ok(html) => {
            if let Some(out) = cli.output {
                if let Err(e) = std::fs::write(&out, &html) {
                    eprintln!("error writing {out:?}: {e}");
                    return ExitCode::FAILURE;
                }
                println!("wrote {}", out.display());
            } else {
                let stdout = std::io::stdout();
                let mut lock = stdout.lock();
                if let Err(e) = lock.write_all(html.as_bytes()) {
                    eprintln!("error writing stdout: {e}");
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
