use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut platform = String::from("win");

    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).map(PathBuf::from);
            }
            "--platform" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    platform = p.clone();
                }
            }
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            _ => {
                if input.is_none() {
                    input = Some(PathBuf::from(a));
                } else {
                    eprintln!("error: unexpected argument '{a}'");
                    return ExitCode::FAILURE;
                }
            }
        }
        i += 1;
    }

    let Some(input) = input else {
        print_help();
        return ExitCode::FAILURE;
    };

    match kanata_viewer_core::render_file(&input, &platform) {
        Ok(html) => {
            if let Some(out) = output {
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

fn print_help() {
    eprintln!(
        "kanata-viewer <input.kbd> [-o output.html] [--platform win|linux|macos]"
    );
}
