use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, FontId, Layout, RichText, Stroke, TextStyle,
};

mod document;
mod logo;
use document::Document;

const VOID: Color32 = Color32::from_rgb(3, 5, 12);
const DEEP: Color32 = Color32::from_rgb(6, 9, 19);
const PANEL: Color32 = Color32::from_rgb(8, 13, 27);
const SURFACE: Color32 = Color32::from_rgb(12, 20, 38);
const SURFACE_HOVER: Color32 = Color32::from_rgb(17, 31, 53);
const TEXT: Color32 = Color32::from_rgb(218, 235, 255);
const MUTED: Color32 = Color32::from_rgb(155, 173, 198);
const CYAN: Color32 = Color32::from_rgb(35, 242, 255);
const MAGENTA: Color32 = Color32::from_rgb(255, 45, 202);
const LIME: Color32 = Color32::from_rgb(140, 255, 124);
const AMBER: Color32 = Color32::from_rgb(255, 199, 82);
const DANGER: Color32 = Color32::from_rgb(255, 78, 112);
const BORDER: Color32 = Color32::from_rgb(22, 69, 88);

pub struct GlyphwireApp {
    root: PathBuf,
    documents: Vec<Document>,
    active: Option<usize>,
    error: Option<String>,
    pending_delete: Option<PathBuf>,
    show_files: bool,
}

impl GlyphwireApp {
    pub fn new(context: &eframe::CreationContext<'_>, opened_path: PathBuf) -> Self {
        configure_theme(&context.egui_ctx);
        Self::open(opened_path)
    }

    fn open(opened_path: PathBuf) -> Self {
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
        let mut app = Self {
            root,
            documents: Vec::new(),
            active: None,
            error: None,
            pending_delete: None,
            show_files: true,
        };
        if let Some(path) = initial_file {
            app.load_file(path);
        }
        app
    }

    fn load_file(&mut self, path: PathBuf) {
        // Opening the same file again selects its existing buffer rather than creating
        // two competing autosaves (also handles symlink aliases).
        let path = path.canonicalize().unwrap_or(path);
        if let Some(index) = self
            .documents
            .iter()
            .position(|document| document.path == path)
        {
            self.active = Some(index);
            return;
        }
        match Document::open(path) {
            Ok(document) => {
                self.documents.push(document);
                self.active = Some(self.documents.len() - 1);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn close_tab(&mut self, index: usize) {
        if !self.documents[index].save_now() {
            self.active = Some(index);
            return;
        }
        self.remove_tab(index);
    }

    fn remove_tab(&mut self, index: usize) {
        self.documents.remove(index);
        self.active = match self.active {
            _ if self.documents.is_empty() => None,
            Some(active) if active > index => Some(active - 1),
            Some(active) if active == index => Some(index.min(self.documents.len() - 1)),
            active => active,
        };
    }

    fn save_all(&mut self) -> bool {
        let mut saved = true;
        for document in &mut self.documents {
            // Do not short-circuit: other tabs should still be saved after an error.
            saved = document.save_now() && saved;
        }
        saved
    }

    fn active_document(&self) -> Option<&Document> {
        self.active.and_then(|index| self.documents.get(index))
    }

    fn show_toolbar(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::top("command_header")
            .frame(pane_frame(VOID))
            .show(context, |ui| {
                ui.horizontal_wrapped(|ui| {
                    logo::show(ui);
                    ui.separator();
                    if ui
                        .button("New Window")
                        .on_hover_text("Open this folder in another window (Cmd/Ctrl+N)")
                        .clicked()
                    {
                        self.new_window();
                    }
                    let files_label = if self.show_files {
                        "Hide files"
                    } else {
                        "Show files"
                    };
                    if ui
                        .button(files_label)
                        .on_hover_text("Collapse or expand the file picker")
                        .clicked()
                    {
                        self.show_files = !self.show_files;
                    }
                    if let Some(index) = self.active {
                        let document = &mut self.documents[index];
                        let raw_label = if document.show_raw {
                            "Hide raw"
                        } else {
                            "Show raw"
                        };
                        if ui
                            .button(raw_label)
                            .on_hover_text("Toggle the raw editor (Cmd/Ctrl+Shift+E)")
                            .clicked()
                        {
                            document.show_raw = !document.show_raw;
                        }
                        if ui
                            .button("Copy Markdown")
                            .on_hover_text(
                                "Copy the entire raw Markdown, including unsaved changes",
                            )
                            .clicked()
                        {
                            context.copy_text(document.text.clone());
                        }
                        if ui
                            .button("Find")
                            .on_hover_text("Find in document (Cmd/Ctrl+F)")
                            .clicked()
                        {
                            document.open_find();
                        }
                        let (status, color) = document.status();
                        ui.label(RichText::new(status).color(color));
                    }
                });
            });

        if !self.documents.is_empty() {
            let mut close = None;
            egui::TopBottomPanel::top("document_tabs")
                .frame(pane_frame(DEEP))
                .show(context, |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("tabs_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (index, document) in self.documents.iter().enumerate() {
                                    ui.push_id(&document.path, |ui| {
                                        let name = document
                                            .path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy();
                                        let marker = if document.error().is_some() {
                                            " !"
                                        } else if document.is_dirty() {
                                            " •"
                                        } else {
                                            ""
                                        };
                                        if ui
                                            .selectable_label(
                                                self.active == Some(index),
                                                format!("{name}{marker}"),
                                            )
                                            .on_hover_text(document.path.display().to_string())
                                            .clicked()
                                        {
                                            self.active = Some(index);
                                        }
                                        if ui
                                            .button("×")
                                            .on_hover_text("Close tab (Cmd/Ctrl+W)")
                                            .clicked()
                                        {
                                            close = Some(index);
                                        }
                                        ui.separator();
                                    });
                                }
                            });
                        });
                });
            if let Some(index) = close {
                self.close_tab(index);
            }
        }
    }

    fn delete_file(&mut self, path: &Path) {
        // Unlinking a symlink must not close or discard edits to its target.
        let is_symlink = fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_symlink());
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        match fs::remove_file(path) {
            Ok(()) => {
                if !is_symlink {
                    if let Some(index) = self
                        .documents
                        .iter()
                        .position(|document| document.path == canonical_path)
                    {
                        // Deliberately do not save a buffer whose file was just deleted.
                        self.remove_tab(index);
                    }
                }
                self.error = None;
            }
            Err(error) => {
                self.error = Some(format!("Could not delete {}: {error}", path.display()))
            }
        }
    }

    fn show_delete_confirmation(&mut self, context: &egui::Context) {
        let Some(path) = self.pending_delete.clone() else {
            return;
        };
        let response = egui::Modal::new(egui::Id::new("confirm_delete")).show(context, |ui| {
            ui.set_max_width(480.0);
            ui.heading("Delete file?");
            ui.label(path.display().to_string());
            ui.colored_label(
                DANGER,
                "This permanently deletes the file. It cannot be undone.",
            );
            ui.label("If open here, its tab will close and any unsaved changes will be discarded.");
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    self.pending_delete = None;
                }
                if ui
                    .button(RichText::new("Delete file").color(DANGER))
                    .clicked()
                {
                    self.delete_file(&path);
                    self.pending_delete = None;
                }
            });
        });
        if response.should_close() {
            self.pending_delete = None;
        }
    }

    fn new_window(&mut self) {
        if let Err(error) = crate::spawn_window(&self.root) {
            self.error = Some(format!("Could not open a new window: {error}"));
        }
    }
}

impl eframe::App for GlyphwireApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        for document in &mut self.documents {
            document.save_if_due(context);
        }

        let shortcuts_enabled = self.pending_delete.is_none();
        let shortcut = |modifiers, key| {
            shortcuts_enabled
                && context.input_mut(|input| {
                    input.consume_shortcut(&egui::KeyboardShortcut::new(modifiers, key))
                })
        };
        if shortcuts_enabled {
            if let Some(index) = self.active {
                self.documents[index].handle_find_shortcuts(context);
            }
        }
        if shortcut(egui::Modifiers::COMMAND, egui::Key::S) {
            self.save_all();
        }
        if shortcut(egui::Modifiers::COMMAND, egui::Key::N) {
            self.new_window();
        }
        if shortcut(egui::Modifiers::COMMAND, egui::Key::W) {
            if let Some(index) = self.active {
                self.close_tab(index);
            }
        }
        if shortcut(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::E,
        ) {
            if let Some(index) = self.active {
                self.documents[index].show_raw = !self.documents[index].show_raw;
            }
        }
        if context.input(|input| input.viewport().close_requested()) && !self.save_all() {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        self.show_toolbar(context);

        let errors: Vec<_> = self
            .error
            .iter()
            .map(String::as_str)
            .chain(self.documents.iter().filter_map(Document::error))
            .map(str::to_owned)
            .collect();
        if !errors.is_empty() {
            egui::TopBottomPanel::bottom("errors")
                .frame(pane_frame(VOID))
                .show(context, |ui| {
                    for error in errors {
                        ui.colored_label(DANGER, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Retry save").clicked() {
                            self.save_all();
                        }
                        if self.error.is_some() && ui.button("Dismiss").clicked() {
                            self.error = None;
                        }
                    });
                });
        }

        egui::TopBottomPanel::bottom("telemetry_footer")
            .frame(pane_frame(VOID))
            .show(context, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(document) = self.active_document() {
                        ui.label(
                            RichText::new(
                                document
                                    .path
                                    .strip_prefix(&self.root)
                                    .unwrap_or(&document.path)
                                    .display()
                                    .to_string(),
                            )
                            .color(CYAN),
                        );
                        ui.separator();
                        ui.label(format!(
                            "{} lines · {} words · {} characters",
                            document.text.lines().count().max(1),
                            document.text.split_whitespace().count(),
                            document.text.chars().count()
                        ));
                        ui.separator();
                        ui.label(if document.show_raw {
                            "Linked scrolling"
                        } else {
                            "Preview only"
                        });
                    } else {
                        ui.label("Choose a Markdown file to open a tab");
                    }
                });
            });

        self.show_file_browser(context);
        if let Some(index) = self.active {
            self.documents[index].show(context);
        } else {
            egui::CentralPanel::default().frame(pane_frame(PANEL)).show(context, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a Markdown file from the file browser. Each file opens in its own tab.");
                });
            });
        }
        self.show_delete_confirmation(context);
    }

    fn on_exit(&mut self) {
        self.save_all();
    }
}

impl GlyphwireApp {
    fn show_file_browser(&mut self, context: &egui::Context) {
        if !self.show_files {
            return;
        }
        let selected = self.active_document().map(|document| document.path.clone());
        let mut file_to_open = None;
        egui::SidePanel::left("file_browser")
            .default_width(245.0)
            .min_width(170.0)
            .max_width(480.0)
            .resizable(true)
            .frame(pane_frame(PANEL))
            .show(context, |ui| {
                ui.heading("Files");
                neon_rule(ui, MAGENTA);
                ui.label(
                    RichText::new(self.root.file_name().and_then(OsStr::to_str).unwrap_or("/"))
                        .strong()
                        .color(CYAN),
                )
                .on_hover_text(self.root.display().to_string());
                ui.add_space(7.0);
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_directory(
                            ui,
                            &self.root,
                            selected.as_deref(),
                            &mut file_to_open,
                            &mut self.pending_delete,
                        );
                    });
            });

        if let Some(path) = file_to_open {
            self.load_file(path);
        }
    }
}

fn pane_frame(fill: Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .inner_margin(egui::Margin::same(12))
}

fn configure_theme(context: &egui::Context) {
    // Bundle open-source fonts for consistent readability without system-font dependencies.
    let mut fonts = egui::FontDefinitions::default();
    for (name, family, bytes) in [
        (
            "Source Sans 3",
            FontFamily::Proportional,
            include_bytes!("../assets/fonts/SourceSans3-Regular.ttf").as_slice(),
        ),
        (
            "JetBrains Mono",
            FontFamily::Monospace,
            include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), egui::FontData::from_static(bytes).into());
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, name.to_owned());
    }
    context.set_fonts(fonts);

    let mut style = (*context.style()).clone();
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(MUTED);
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.extreme_bg_color = VOID;
    visuals.text_edit_bg_color = Some(VOID);
    visuals.code_bg_color = SURFACE;
    visuals.faint_bg_color = SURFACE;
    visuals.hyperlink_color = CYAN;
    visuals.warn_fg_color = AMBER;
    visuals.error_fg_color = DANGER;
    visuals.selection.bg_fill = Color32::from_rgb(48, 50, 56);
    visuals.selection.stroke = Stroke::new(1.0_f32, TEXT);
    visuals.indent_has_left_vline = true;
    visuals.collapsing_header_frame = false;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.weak_bg_fill = SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.weak_bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, CYAN);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, CYAN);
    visuals.widgets.active.bg_fill = Color32::from_rgb(17, 43, 55);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(17, 43, 55);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, MAGENTA);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, CYAN);
    visuals.widgets.open = visuals.widgets.active;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(3);
    }
    visuals.text_cursor.stroke = Stroke::new(2.0_f32, MAGENTA);
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 9.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    style.spacing.indent = 18.0;
    style.animation_time = 0.08;
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(16.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(16.0, FontFamily::Monospace),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(18.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(14.0, FontFamily::Proportional),
    );
    context.set_style(style);
}

fn neon_rule(ui: &mut egui::Ui, color: Color32) {
    let (response, painter) =
        ui.allocate_painter(egui::vec2(ui.available_width(), 8.0), egui::Sense::hover());
    painter.line_segment(
        [response.rect.left_center(), response.rect.right_center()],
        Stroke::new(1.0_f32, color.gamma_multiply(0.65)),
    );
    painter.circle_filled(response.rect.left_center(), 2.0, color);
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
    file_to_delete: &mut Option<PathBuf>,
) {
    let entries = match directory_entries(directory) {
        Ok(entries) => entries,
        Err(error) => {
            ui.colored_label(DANGER, format!("Could not read folder: {error}"));
            return;
        }
    };
    for entry in entries {
        if entry.is_directory && !entry.is_symlink {
            egui::CollapsingHeader::new(RichText::new(&entry.name).strong().color(CYAN))
                .id_salt(&entry.path)
                .show(ui, |ui| {
                    render_directory(ui, &entry.path, selected, file_to_open, file_to_delete);
                });
            continue;
        }
        let markdown = is_markdown(&entry.path);
        let is_selected = selected == Some(entry.path.as_path());
        let color = if markdown { TEXT } else { MUTED };
        let response = ui
            .add(
                egui::Button::selectable(
                    is_selected,
                    RichText::new(&entry.name).size(16.0).color(color),
                )
                .frame(is_selected),
            )
            .on_hover_text(entry.path.display().to_string());
        response.context_menu(|ui| {
            if ui
                .button(RichText::new("Delete file…").color(DANGER))
                .clicked()
            {
                *file_to_delete = Some(entry.path.clone());
                ui.close();
            }
        });
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
