mod app;

use std::{env, path::PathBuf, process::ExitCode};

use app::OctomarkApp;
use eframe::egui;

const HELP: &str = "Octomark — a local Markdown editor and live preview\n\nUSAGE:\n    octomark <PATH>\n\nARGS:\n    <PATH>    A directory or Markdown file\n\nEXAMPLES:\n    octomark .\n    octomark README.md";

fn main() -> ExitCode {
    let path = match parse_path(env::args_os().skip(1).collect()) {
        Ok(Some(path)) => path,
        Ok(None) => return ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("octomark: {message}\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    };

    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name} — Octomark"))
        .unwrap_or_else(|| "Octomark".to_owned());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_app_id("com.octomark.editor")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([900.0, 560.0]),
        ..Default::default()
    };

    match eframe::run_native(
        "Octomark",
        native_options,
        Box::new(move |creation_context| Ok(Box::new(OctomarkApp::new(creation_context, path)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("octomark: could not open the application: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_path(arguments: Vec<std::ffi::OsString>) -> Result<Option<PathBuf>, String> {
    if arguments.len() != 1 {
        if arguments.is_empty() {
            return Err("missing a directory or Markdown file".to_owned());
        }
        return Err("expected exactly one path".to_owned());
    }

    if arguments[0] == "-h" || arguments[0] == "--help" {
        println!("{HELP}");
        return Ok(None);
    }
    if arguments[0] == "-V" || arguments[0] == "--version" {
        println!("octomark {}", env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let path = PathBuf::from(&arguments[0]);
    let path = path
        .canonicalize()
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;

    if path.is_file() && !app::is_markdown(&path) {
        return Err(format!("{} is not a Markdown file", path.display()));
    }
    if !path.is_file() && !path.is_dir() {
        return Err(format!("{} is not a file or directory", path.display()));
    }

    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn help_needs_no_path() {
        assert!(parse_path(vec![OsString::from("--help")])
            .unwrap()
            .is_none());
    }

    #[test]
    fn rejects_too_many_paths() {
        assert!(parse_path(vec![OsString::from("a"), OsString::from("b")]).is_err());
    }
}
