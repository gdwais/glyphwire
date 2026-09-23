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

pub(crate) fn spawn_window(path: &Path) -> std::io::Result<()> {
    Command::new(env::current_exe()?)
        .arg(GUI_FLAG)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn launch_app(path: PathBuf, command_name: &str) -> ExitCode {
    match spawn_window(&path) {
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

    match eframe::run_native(
        "Glyphwire",
        native_options(&title),
        Box::new(move |creation_context| Ok(Box::new(GlyphwireApp::new(creation_context, path)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{command_name}: could not open the application: {error}");
            ExitCode::FAILURE
        }
    }
}

fn native_options(title: &str) -> eframe::NativeOptions {
    use eframe::egui_wgpu::{SurfaceErrorAction, WgpuConfiguration, WgpuSetupCreateNew};

    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_app_id("com.glyphwire.editor")
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([960.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: WgpuConfiguration {
            wgpu_setup: WgpuSetupCreateNew {
                instance_descriptor: wgpu::InstanceDescriptor {
                    // Avoid NSOpenGLContext/CGLFlushDrawable, which crashed inside AppKit
                    // during a display reconfiguration. Do not allow an OpenGL fallback
                    // or a WGPU_BACKEND environment override to re-enable that path.
                    backends: wgpu::Backends::METAL,
                    ..Default::default()
                },
                ..Default::default()
            }
            .into(),
            on_surface_error: std::sync::Arc::new(|error| match error {
                // Display changes and resume can invalidate the presentation surface.
                wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                    SurfaceErrorAction::RecreateSurface
                }
                _ => SurfaceErrorAction::SkipFrame,
            }),
            ..Default::default()
        },
        ..Default::default()
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
    fn renderer_uses_metal_without_an_opengl_fallback() {
        let options = native_options("Glyphwire test");
        assert_eq!(options.renderer, eframe::Renderer::Wgpu);
        let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = options.wgpu_options.wgpu_setup else {
            panic!("Expected a new Metal instance");
        };
        assert_eq!(setup.instance_descriptor.backends, wgpu::Backends::METAL);
    }

    #[test]
    fn invalidated_surfaces_are_reconfigured_and_timeouts_skip_a_frame() {
        use eframe::egui_wgpu::SurfaceErrorAction;
        let options = native_options("Glyphwire test");
        let handle_error = options.wgpu_options.on_surface_error;
        for error in [wgpu::SurfaceError::Lost, wgpu::SurfaceError::Outdated] {
            assert!(matches!(
                handle_error(error),
                SurfaceErrorAction::RecreateSurface
            ));
        }
        assert!(matches!(
            handle_error(wgpu::SurfaceError::Timeout),
            SurfaceErrorAction::SkipFrame
        ));
    }

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
