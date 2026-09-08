use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use eframe::egui::{self, Color32, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

const AUTO_SAVE_DELAY: Duration = Duration::from_millis(400);

pub struct OctomarkApp {
    root: PathBuf,
    current_file: Option<PathBuf>,
    document: String,
    dirty: bool,
    dirty_since: Option<Instant>,
    save_status: SaveStatus,
    markdown_cache: CommonMarkCache,
}

#[derive(Default)]
enum SaveStatus {
    #[default]
    Idle,
    Pending,
    Saved,
    Error(String),
}

impl OctomarkApp {
    pub fn new(context: &eframe::CreationContext<'_>, opened_path: PathBuf) -> Self {
        let (root, initial_file) = if opened_path.is_file() {
            (
                opened_path
                    .parent()
                    .unwrap_or_else(|| Path::new("/"))
                    .to_path_buf(),
                Some(opened_path),
            )
        } else {
            (opened_path, None)
        };

        context.egui_ctx.set_visuals(egui::Visuals::dark());
        context.egui_ctx.style_mut(|style| {
            style.url_in_tooltip = true;
            style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        });

        let mut app = Self {
            root,
            current_file: None,
            document: String::new(),
            dirty: false,
            dirty_since: None,
            save_status: SaveStatus::Idle,
            markdown_cache: CommonMarkCache::default(),
        };

        if let Some(path) = initial_file {
            app.load_file(path);
        }
        app
    }

    fn load_file(&mut self, path: PathBuf) {
        if self.current_file.as_ref() == Some(&path) {
            return;
        }
        if !self.save_now() {
            return;
        }

        match fs::read_to_string(&path) {
            Ok(document) => {
                self.current_file = Some(path);
                self.document = document;
                self.dirty = false;
                self.dirty_since = None;
                self.save_status = SaveStatus::Saved;
                self.markdown_cache = CommonMarkCache::default();
            }
            Err(error) => {
                self.save_status =
                    SaveStatus::Error(format!("Could not read {}: {error}", path.display()));
            }
        }
    }

    fn save_now(&mut self) -> bool {
        if !self.dirty {
            return true;
        }

        let Some(path) = self.current_file.as_ref() else {
            return true;
        };

        match fs::write(path, self.document.as_bytes()) {
            Ok(()) => {
                self.dirty = false;
                self.dirty_since = None;
                self.save_status = SaveStatus::Saved;
                true
            }
            Err(error) => {
                self.dirty_since = None;
                self.save_status = SaveStatus::Error(format!("Could not save: {error}"));
                false
            }
        }
    }

    fn schedule_save(&mut self, context: &egui::Context) {
        self.dirty = true;
        self.dirty_since = Some(Instant::now());
        self.save_status = SaveStatus::Pending;
        context.request_repaint_after(AUTO_SAVE_DELAY);
    }

    fn save_if_due(&mut self) {
        if self
            .dirty_since
            .is_some_and(|changed| changed.elapsed() >= AUTO_SAVE_DELAY)
        {
            self.save_now();
        }
    }

    fn status_text(&self) -> (String, Color32) {
        match &self.save_status {
            SaveStatus::Idle => ("Choose a Markdown file".to_owned(), Color32::GRAY),
            SaveStatus::Pending => ("Saving…".to_owned(), Color32::YELLOW),
            SaveStatus::Saved => ("Saved".to_owned(), Color32::from_rgb(110, 200, 140)),
            SaveStatus::Error(message) => (message.clone(), Color32::from_rgb(240, 100, 100)),
        }
    }

    fn selected_label(&self) -> String {
        self.current_file
            .as_ref()
            .map(|path| {
                path.strip_prefix(&self.root)
                    .unwrap_or(path)
                    .display()
                    .to_string()
            })
            .unwrap_or_else(|| "No file selected".to_owned())
    }

    fn image_base_uri(&self) -> String {
        let directory = self
            .current_file
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or(&self.root);
        format!("file://{}/", directory.display())
    }
}

impl eframe::App for OctomarkApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.save_if_due();

        let save_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);
        if context.input_mut(|input| input.consume_shortcut(&save_shortcut)) {
            self.save_now();
        }

        egui::TopBottomPanel::top("title_bar").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.strong(self.selected_label());
                ui.add_space(8.0);
                let (status, color) = self.status_text();
                ui.label(RichText::new(status).color(color).small());
            });
        });

        let root = self.root.clone();
        let selected = self.current_file.clone();
        let mut file_to_open = None;

        egui::SidePanel::left("file_browser")
            .default_width(240.0)
            .min_width(150.0)
            .max_width(480.0)
            .resizable(true)
            .show(context, |ui| {
                ui.heading("Files");
                ui.label(
                    RichText::new(
                        root.file_name()
                            .and_then(OsStr::to_str)
                            .unwrap_or_else(|| root.to_str().unwrap_or("/")),
                    )
                    .color(Color32::GRAY)
                    .small(),
                );
                ui.separator();
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_directory(ui, &root, selected.as_deref(), &mut file_to_open);
                    });
            });

        egui::SidePanel::left("markdown_editor")
            .default_width(560.0)
            .min_width(280.0)
            .resizable(true)
            .show(context, |ui| {
                ui.heading("Markdown");
                ui.separator();
                if self.current_file.is_some() {
                    egui::ScrollArea::both()
                        .id_salt("markdown_editor_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let editor = egui::TextEdit::multiline(&mut self.document)
                                .code_editor()
                                .lock_focus(true)
                                .desired_width(f32::INFINITY)
                                .desired_rows(32)
                                .margin(egui::Margin::same(12));
                            if ui.add_sized(ui.available_size(), editor).changed() {
                                self.schedule_save(context);
                            }
                        });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Select a Markdown file from the sidebar.").weak());
                    });
                }
            });

        egui::CentralPanel::default().show(context, |ui| {
            ui.heading("Preview");
            ui.separator();
            if self.current_file.is_some() {
                let base_uri = self.image_base_uri();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let max_width = ui.available_width().max(1.0) as usize;
                        CommonMarkViewer::new()
                            .max_image_width(Some(max_width))
                            .default_width(Some(max_width))
                            .default_implicit_uri_scheme(base_uri)
                            .show(ui, &mut self.markdown_cache, &self.document);
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("Select a Markdown file to preview it.").weak());
                });
            }
        });

        if let Some(path) = file_to_open {
            self.load_file(path);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_now();
    }
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown")
        })
}

#[derive(Debug)]
struct DirectoryEntry {
    path: PathBuf,
    name: String,
    is_directory: bool,
    is_symlink: bool,
}

fn directory_entries(path: &Path) -> Result<Vec<DirectoryEntry>, std::io::Error> {
    let mut entries = Vec::new();
    for item in fs::read_dir(path)? {
        let item = item?;
        let file_type = item.file_type()?;
        entries.push(DirectoryEntry {
            path: item.path(),
            name: item.file_name().to_string_lossy().into_owned(),
            is_directory: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        });
    }

    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

fn render_directory(
    ui: &mut egui::Ui,
    directory: &Path,
    selected: Option<&Path>,
    file_to_open: &mut Option<PathBuf>,
) {
    let entries = match directory_entries(directory) {
        Ok(entries) => entries,
        Err(error) => {
            ui.label(RichText::new(format!("Cannot read directory: {error}")).color(Color32::RED));
            return;
        }
    };

    for entry in entries {
        if entry.is_directory && !entry.is_symlink {
            egui::CollapsingHeader::new(RichText::new(&entry.name).strong())
                .id_salt(&entry.path)
                .show(ui, |ui| {
                    render_directory(ui, &entry.path, selected, file_to_open);
                });
            continue;
        }

        let markdown = is_markdown(&entry.path);
        let is_selected = selected == Some(entry.path.as_path());
        let response = ui
            .add_enabled(
                markdown,
                egui::Button::selectable(is_selected, RichText::new(&entry.name).monospace())
                    .frame(false),
            )
            .on_hover_text(entry.path.display().to_string());

        if markdown && response.clicked() {
            *file_to_open = Some(entry.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_markdown_extensions_case_insensitively() {
        assert!(is_markdown(Path::new("README.md")));
        assert!(is_markdown(Path::new("notes.MARKDOWN")));
        assert!(!is_markdown(Path::new("notes.txt")));
        assert!(!is_markdown(Path::new("md")));
    }
}
