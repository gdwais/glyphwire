use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

use super::*;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

#[cfg(test)]
mod tests;

const AUTO_SAVE_DELAY: Duration = Duration::from_millis(400);

pub(super) struct Document {
    pub path: PathBuf,
    pub text: String,
    pub show_raw: bool,
    dirty: bool,
    dirty_since: Option<Instant>,
    save_error: Option<String>,
    markdown_cache: CommonMarkCache,
    scroll_fraction: f32,
    editor_max_scroll: f32,
    preview_max_scroll: f32,
}

impl Document {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        Ok(Self {
            path,
            text,
            show_raw: true,
            dirty: false,
            dirty_since: None,
            save_error: None,
            markdown_cache: CommonMarkCache::default(),
            scroll_fraction: 0.0,
            editor_max_scroll: 0.0,
            preview_max_scroll: 0.0,
        })
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn save_now(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        // Never recreate a file deleted from another window or outside the app.
        let result = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .and_then(|mut file| file.write_all(self.text.as_bytes()));
        match result {
            Ok(()) => {
                self.dirty = false;
                self.dirty_since = None;
                self.save_error = None;
                true
            }
            Err(error) => {
                self.dirty_since = None;
                self.save_error = Some(format!("Could not save {}: {error}", self.path.display()));
                false
            }
        }
    }

    fn schedule_save(&mut self, context: &egui::Context) {
        self.dirty = true;
        self.dirty_since = Some(Instant::now());
        self.save_error = None;
        context.request_repaint_after(AUTO_SAVE_DELAY);
    }

    pub fn save_if_due(&mut self, context: &egui::Context) {
        if let Some(changed) = self.dirty_since {
            if changed.elapsed() >= AUTO_SAVE_DELAY {
                self.save_now();
            } else {
                context.request_repaint_after(AUTO_SAVE_DELAY.saturating_sub(changed.elapsed()));
            }
        }
    }

    pub fn status(&self) -> (&str, Color32) {
        if self.save_error.is_some() {
            ("SAVE FAILED", DANGER)
        } else if self.dirty {
            ("SAVING", AMBER)
        } else {
            ("SAVED", LIME)
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.save_error.as_deref()
    }

    fn sync_scroll(
        &mut self,
        actual_offset: f32,
        requested_offset: f32,
        content_height: f32,
        viewport_height: f32,
        editor: bool,
        context: &egui::Context,
    ) {
        let max_scroll = (content_height - viewport_height).max(0.0);
        let previous_max = if editor {
            &mut self.editor_max_scroll
        } else {
            &mut self.preview_max_scroll
        };
        let dimensions_changed = (*previous_max - max_scroll).abs() > 0.5;
        *previous_max = max_scroll;
        if max_scroll > 0.0 && (actual_offset - requested_offset).abs() > 0.5 {
            self.scroll_fraction = (actual_offset / max_scroll).clamp(0.0, 1.0);
            context.request_repaint();
        } else if dimensions_changed {
            context.request_repaint();
        }
    }

    pub fn show(&mut self, context: &egui::Context) {
        // File-specific IDs preserve cursor, selection, and scroll state across tab switches.
        let id = egui::Id::new(&self.path);
        if self.show_raw {
            egui::SidePanel::left("markdown_editor")
                .default_width(540.0)
                .min_width(260.0)
                .resizable(true)
                .frame(pane_frame(DEEP))
                .show(context, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Raw Markdown");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Hide raw").clicked() {
                                self.show_raw = false;
                            }
                        });
                    });
                    neon_rule(ui, CYAN);
                    let requested_scroll = self.scroll_fraction * self.editor_max_scroll;
                    let output = egui::ScrollArea::both()
                        .id_salt(id.with("editor_scroll"))
                        .vertical_scroll_offset(requested_scroll)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let editor = egui::TextEdit::multiline(&mut self.text)
                                .id(id.with("editor"))
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
                    self.sync_scroll(
                        output.state.offset.y,
                        requested_scroll,
                        output.content_size.y,
                        output.inner_rect.height(),
                        true,
                        context,
                    );
                });
        }

        egui::CentralPanel::default()
            .frame(pane_frame(PANEL))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Preview");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !self.show_raw && ui.button("Show raw").clicked() {
                            self.show_raw = true;
                        }
                    });
                });
                neon_rule(ui, MAGENTA);
                let directory = self.path.parent().unwrap_or_else(|| Path::new("/"));
                let base_uri = format!("file://{}/", directory.display());
                let requested_scroll = self.scroll_fraction * self.preview_max_scroll;
                let output = egui::ScrollArea::vertical()
                    .id_salt(id.with("preview_scroll"))
                    .vertical_scroll_offset(requested_scroll)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.push_id(id.with("preview"), |ui| {
                            let max_width = ui.available_width().max(1.0) as usize;
                            if CommonMarkViewer::new()
                                .max_image_width(Some(max_width))
                                .default_width(Some(max_width))
                                .default_implicit_uri_scheme(base_uri)
                                .show_mut(ui, &mut self.markdown_cache, &mut self.text)
                                .response
                                .changed()
                            {
                                self.schedule_save(context);
                            }
                        });
                    });
                self.sync_scroll(
                    output.state.offset.y,
                    requested_scroll,
                    output.content_size.y,
                    output.inner_rect.height(),
                    false,
                    context,
                );
            });
    }
}
