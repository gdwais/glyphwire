pub mod app;

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use app::GlyphwireApp;
use eframe::egui;

const GUI_FLAG: &str = "--glyphwire-gui-process";

pub fn run() -> ExitCode {
    let command_name = env::args_os()
        .next()
        .as_deref()
        .and_then(|executable| Path::new(executable).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("glyphwire")
        .to_owned();
    let mut arguments: Vec<_> = env::args_os().skip(1).collect();
    let is_gui_process = arguments
        .first()
        .is_some_and(|argument| argument == GUI_FLAG);
    if is_gui_process {
        arguments.remove(0);
    }

    let path = match parse_path(arguments, &command_name) {
        Ok(Some(path)) => path,
        Ok(None) => return ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{command_name}: {message}\n\n{}", help(&command_name));
            return ExitCode::FAILURE;
        }
    };

    if is_gui_process {
        run_app(path, &command_name)
    } else {
        launch_app(path, &command_name)
    }
}

fn help(command_name: &str) -> String {
    format!(
        "Glyphwire — a local Markdown editor and live preview\n\nUSAGE:\n    {command_name} <PATH>\n\nARGS:\n    <PATH>    A directory or Markdown file\n\nEXAMPLES:\n    {command_name} .\n    {command_name} README.md\n\nThe command returns immediately after launching the app."
    )
}

fn launch_app(path: PathBuf, command_name: &str) -> ExitCode {
    let executable = match env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("{command_name}: could not locate the application binary: {error}");
            return ExitCode::FAILURE;
        }
    };

    match Command::new(executable)
        .arg(GUI_FLAG)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{command_name}: could not launch the application: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_app(path: PathBuf, command_name: &str) -> ExitCode {
    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name} // Glyphwire"))
        .unwrap_or_else(|| "Glyphwire".to_owned());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_app_id("com.glyphwire.editor")
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    match eframe::run_native(
        "Glyphwire",
        native_options,
        Box::new(move |creation_context| Ok(Box::new(GlyphwireApp::new(creation_context, path)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{command_name}: could not open the application: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_path(
    arguments: Vec<std::ffi::OsString>,
    command_name: &str,
) -> Result<Option<PathBuf>, String> {
    if arguments.len() != 1 {
        if arguments.is_empty() {
            return Err("missing a directory or Markdown file".to_owned());
        }
        return Err("expected exactly one path".to_owned());
    }

    if arguments[0] == "-h" || arguments[0] == "--help" {
        println!("{}", help(command_name));
        return Ok(None);
    }
    if arguments[0] == "-V" || arguments[0] == "--version" {
        println!("{command_name} {}", env!("CARGO_PKG_VERSION"));
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
        assert!(parse_path(vec![OsString::from("--help")], "glyphwire")
            .unwrap()
            .is_none());
    }

    #[test]
    fn rejects_too_many_paths() {
        assert!(parse_path(vec![OsString::from("a"), OsString::from("b")], "glyphwire").is_err());
    }
}
