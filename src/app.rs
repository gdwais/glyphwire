use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, FontId, Layout, RichText, Stroke, TextStyle,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

const AUTO_SAVE_DELAY: Duration = Duration::from_millis(400);

const VOID: Color32 = Color32::from_rgb(3, 5, 12);
const DEEP: Color32 = Color32::from_rgb(6, 9, 19);
const PANEL: Color32 = Color32::from_rgb(8, 13, 27);
const SURFACE: Color32 = Color32::from_rgb(12, 20, 38);
const SURFACE_HOVER: Color32 = Color32::from_rgb(17, 31, 53);
const TEXT: Color32 = Color32::from_rgb(218, 235, 255);
const MUTED: Color32 = Color32::from_rgb(94, 112, 144);
const CYAN: Color32 = Color32::from_rgb(35, 242, 255);
const MAGENTA: Color32 = Color32::from_rgb(255, 45, 202);
const LIME: Color32 = Color32::from_rgb(140, 255, 124);
const AMBER: Color32 = Color32::from_rgb(255, 199, 82);
const DANGER: Color32 = Color32::from_rgb(255, 78, 112);
const BORDER: Color32 = Color32::from_rgb(22, 69, 88);

pub struct GlyphwireApp {
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

impl GlyphwireApp {
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

        configure_theme(&context.egui_ctx);

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

    fn status_text(&self) -> (&str, Color32) {
        match &self.save_status {
            SaveStatus::Idle => ("STANDBY", MUTED),
            SaveStatus::Pending => ("SYNCING", AMBER),
            SaveStatus::Saved => ("SYNCED", LIME),
            SaveStatus::Error(_) => ("FAULT", DANGER),
        }
    }

    fn status_detail(&self) -> Option<&str> {
        match &self.save_status {
            SaveStatus::Error(message) => Some(message),
            _ => None,
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
            .unwrap_or_else(|| "NO FILE LINKED".to_owned())
    }

    fn image_base_uri(&self) -> String {
        let directory = self
            .current_file
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or(&self.root);
        format!("file://{}/", directory.display())
    }

    fn document_stats(&self) -> (usize, usize, usize) {
        if self.current_file.is_none() {
            return (0, 0, 0);
        }
        let lines = self.document.lines().count().max(1);
        let words = self.document.split_whitespace().count();
        let characters = self.document.chars().count();
        (lines, words, characters)
    }
}

impl eframe::App for GlyphwireApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.save_if_due();

        let save_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);
        if context.input_mut(|input| input.consume_shortcut(&save_shortcut)) {
            self.save_now();
        }

        egui::TopBottomPanel::top("command_header")
            .exact_height(72.0)
            .frame(
                egui::Frame::new()
                    .fill(VOID)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .inner_margin(egui::Margin::symmetric(18, 10)),
            )
            .show(context, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("GLYPH")
                                    .family(FontFamily::Monospace)
                                    .size(22.0)
                                    .strong()
                                    .color(MAGENTA),
                            );
                            ui.label(
                                RichText::new("//WIRE")
                                    .family(FontFamily::Monospace)
                                    .size(22.0)
                                    .strong()
                                    .color(CYAN),
                            );
                        });
                        ui.label(
                            RichText::new(format!(
                                "LOCAL MARKDOWN INTERFACE  v{}",
                                env!("CARGO_PKG_VERSION")
                            ))
                            .monospace()
                            .size(9.0)
                            .color(MUTED),
                        );
                    });

                    ui.add_space(30.0);
                    ui.label(RichText::new("FILE ::").monospace().size(11.0).color(MUTED));
                    ui.label(
                        RichText::new(self.selected_label())
                            .monospace()
                            .size(12.0)
                            .color(TEXT),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (status, color) = self.status_text();
                        status_badge(ui, status, color);
                        ui.label(
                            RichText::new("AUTO-SAVE")
                                .monospace()
                                .size(10.0)
                                .color(MUTED),
                        );
                    });
                });
            });

        let (lines, words, characters) = self.document_stats();
        egui::TopBottomPanel::bottom("telemetry_footer")
            .exact_height(34.0)
            .frame(
                egui::Frame::new()
                    .fill(VOID)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .inner_margin(egui::Margin::symmetric(14, 7)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("READY // LOCAL")
                            .monospace()
                            .size(10.0)
                            .color(CYAN),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("LN {lines:04}  WD {words:04}  CH {characters:05}"))
                            .monospace()
                            .size(10.0)
                            .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("CMD+S  FORCE SYNC")
                                .monospace()
                                .size(10.0)
                                .color(MUTED),
                        );
                        if let Some(error) = self.status_detail() {
                            ui.label(RichText::new(error).monospace().size(10.0).color(DANGER));
                        }
                    });
                });
            });

        let root = self.root.clone();
        let selected = self.current_file.clone();
        let mut file_to_open = None;

        egui::SidePanel::left("file_browser")
            .default_width(255.0)
            .min_width(170.0)
            .max_width(480.0)
            .resizable(true)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(context, |ui| {
                panel_header(ui, "01", "DATA TREE", "ALL FILES");
                neon_rule(ui, MAGENTA);

                let root_name = root
                    .file_name()
                    .and_then(OsStr::to_str)
                    .unwrap_or_else(|| root.to_str().unwrap_or("/"));
                egui::Frame::new()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(3))
                    .inner_margin(egui::Margin::symmetric(9, 7))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("MOUNT POINT")
                                .monospace()
                                .size(9.0)
                                .color(MUTED),
                        );
                        ui.label(
                            RichText::new(root_name)
                                .monospace()
                                .size(12.0)
                                .strong()
                                .color(CYAN),
                        );
                    });
                ui.add_space(7.0);

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_directory(ui, &root, selected.as_deref(), &mut file_to_open);
                    });
            });

        egui::SidePanel::left("markdown_editor")
            .default_width(575.0)
            .min_width(300.0)
            .resizable(true)
            .frame(
                egui::Frame::new()
                    .fill(DEEP)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(context, |ui| {
                panel_header(ui, "02", "SOURCE BUFFER", "RAW MARKDOWN");
                neon_rule(ui, CYAN);

                if self.current_file.is_some() {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(4, 8, 17))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(20, 92, 108)))
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(egui::Margin::same(3))
                        .show(ui, |ui| {
                            egui::ScrollArea::both()
                                .id_salt("markdown_editor_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let editor = egui::TextEdit::multiline(&mut self.document)
                                        .code_editor()
                                        .text_color(TEXT)
                                        .frame(false)
                                        .lock_focus(true)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(36)
                                        .margin(egui::Margin::same(12));
                                    if ui.add_sized(ui.available_size(), editor).changed() {
                                        self.schedule_save(context);
                                    }
                                });
                        });
                } else {
                    empty_state(
                        ui,
                        "AWAITING FILE SELECTION",
                        "Choose an MD node from DATA TREE",
                    );
                }
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(context, |ui| {
                panel_header(ui, "03", "RENDER FEED", "LIVE PREVIEW");
                neon_rule(ui, MAGENTA);

                if self.current_file.is_some() {
                    let base_uri = self.image_base_uri();
                    egui::Frame::new()
                        .fill(Color32::from_rgb(10, 16, 29))
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
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
                        });
                } else {
                    empty_state(ui, "NO SIGNAL", "Rendered markdown will stream here");
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

fn configure_theme(context: &egui::Context) {
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
    visuals.hyperlink_color = MAGENTA;
    visuals.warn_fg_color = AMBER;
    visuals.error_fg_color = DANGER;
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(35, 242, 255, 45);
    visuals.selection.stroke = Stroke::new(1.5_f32, CYAN);
    visuals.indent_has_left_vline = true;
    visuals.collapsing_header_frame = false;
    visuals.button_frame = true;
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
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(8.0, 5.0);
    style.spacing.indent = 14.0;
    style.animation_time = 0.08;
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::new(17.0, FontFamily::Monospace));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::new(12.0, FontFamily::Monospace));
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(14.0, FontFamily::Monospace),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::new(10.0, FontFamily::Monospace));

    context.set_style(style);
}

fn status_badge(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.11))
        .stroke(Stroke::new(1.0_f32, color))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(9, 4))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("+ {label}"))
                    .monospace()
                    .size(10.0)
                    .strong()
                    .color(color),
            );
        });
}

fn panel_header(ui: &mut egui::Ui, index: &str, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(MAGENTA.gamma_multiply(0.16))
            .stroke(Stroke::new(1.0_f32, MAGENTA))
            .corner_radius(CornerRadius::same(2))
            .inner_margin(egui::Margin::symmetric(5, 2))
            .show(ui, |ui| {
                ui.label(RichText::new(index).monospace().size(10.0).color(MAGENTA));
            });
        ui.label(
            RichText::new(title)
                .monospace()
                .size(13.0)
                .strong()
                .color(TEXT),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(detail).monospace().size(9.0).color(MUTED));
        });
    });
}

fn neon_rule(ui: &mut egui::Ui, color: Color32) {
    let (response, painter) =
        ui.allocate_painter(egui::vec2(ui.available_width(), 8.0), egui::Sense::hover());
    let y = response.rect.center().y;
    painter.line_segment(
        [
            response.rect.left_center(),
            egui::pos2(response.rect.right(), y),
        ],
        Stroke::new(1.0_f32, color.gamma_multiply(0.65)),
    );
    painter.circle_filled(response.rect.left_center(), 2.0, color);
}

fn empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("< / >").monospace().size(30.0).color(MAGENTA));
            ui.add_space(8.0);
            ui.label(
                RichText::new(title)
                    .monospace()
                    .size(13.0)
                    .strong()
                    .color(CYAN),
            );
            ui.label(RichText::new(detail).monospace().size(10.0).color(MUTED));
        });
    });
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
            ui.label(
                RichText::new(format!("ERR // {error}"))
                    .monospace()
                    .color(DANGER),
            );
            return;
        }
    };

    for entry in entries {
        if entry.is_directory && !entry.is_symlink {
            egui::CollapsingHeader::new(
                RichText::new(format!("// {}", entry.name))
                    .monospace()
                    .size(11.0)
                    .strong()
                    .color(CYAN.gamma_multiply(0.82)),
            )
            .id_salt(&entry.path)
            .show(ui, |ui| {
                render_directory(ui, &entry.path, selected, file_to_open);
            });
            continue;
        }

        let markdown = is_markdown(&entry.path);
        let is_selected = selected == Some(entry.path.as_path());
        let marker = if markdown { "MD" } else { "--" };
        let color = if markdown { TEXT } else { MUTED };
        let response = ui
            .add_enabled(
                markdown,
                egui::Button::selectable(
                    is_selected,
                    RichText::new(format!("{marker}  {}", entry.name))
                        .monospace()
                        .size(11.0)
                        .color(color),
                )
                .frame(is_selected),
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
