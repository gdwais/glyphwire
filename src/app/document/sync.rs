use std::io::{Read, Seek, Write};

use super::*;

const DISK_CHECK_INTERVAL: Duration = Duration::from_secs(1);

impl Document {
    pub fn has_conflict(&self) -> bool {
        self.external_text.is_some()
    }

    pub fn has_sync_dialog(&self) -> bool {
        self.confirm_overwrite.is_some()
    }

    fn mark_synced(&mut self) {
        self.saved_text.clone_from(&self.text);
        self.dirty = false;
        self.dirty_since = None;
        self.save_error = None;
        self.disk_error = None;
        self.external_text = None;
        self.confirm_overwrite = None;
    }

    fn replace_from_disk(&mut self, text: String) {
        self.text = text;
        self.mark_synced();
        self.markdown_cache = CommonMarkCache::default();
        self.find.refresh(&self.text);
        self.reset_editor_state = true;
        // Keep raw-pane visibility and proportional scroll position on reload.
    }

    fn observe_disk(&mut self, text: String) {
        self.disk_error = None;
        if text == self.text {
            // Another writer saved the same contents; there is nothing to overwrite.
            self.mark_synced();
        } else if text != self.saved_text {
            if self.dirty && self.text != self.saved_text {
                self.external_text = Some(text);
                self.dirty_since = None;
                self.disk_error = Some(format!(
                    "{} changed on disk and has local edits. Autosave paused; select its tab to resolve.",
                    self.path.display()
                ));
            } else {
                self.replace_from_disk(text);
            }
        } else {
            // A missing/unreadable file was restored, or an external edit was reverted.
            self.external_text = None;
            self.confirm_overwrite = None;
            if self.dirty && self.dirty_since.is_none() && self.save_error.is_none() {
                self.dirty_since = Some(Instant::now());
            }
        }
    }

    fn read_disk(&mut self) -> Option<String> {
        self.disk_checked_at = Instant::now();
        match fs::read_to_string(&self.path) {
            Ok(text) => Some(text),
            Err(error) => {
                self.dirty_since = None;
                self.save_error = None;
                self.disk_error = Some(format!(
                    "Could not read {}: {error}. Keeping the current buffer; autosave paused.",
                    self.path.display()
                ));
                None
            }
        }
    }

    fn check_disk(&mut self) {
        if let Some(text) = self.read_disk() {
            self.observe_disk(text);
        }
    }

    pub fn save_if_due(&mut self, context: &egui::Context) {
        // Content polling also catches atomic file replacements and changes with the
        // same size/mtime. Schedule a repaint so this runs even while the UI is idle.
        if self.disk_checked_at.elapsed() >= DISK_CHECK_INTERVAL {
            self.check_disk();
        }
        context.request_repaint_after(
            DISK_CHECK_INTERVAL.saturating_sub(self.disk_checked_at.elapsed()),
        );
        if self.disk_error.is_some() || self.has_conflict() {
            return;
        }
        if let Some(changed) = self.dirty_since {
            if changed.elapsed() >= AUTO_SAVE_DELAY {
                self.save_now();
            } else {
                context.request_repaint_after(AUTO_SAVE_DELAY.saturating_sub(changed.elapsed()));
            }
        }
    }

    pub fn save_now(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        // Every save path (autosave, Cmd+S, tab/window close) checks fresh contents,
        // regardless of when the background poll last ran.
        self.check_disk();
        if self.disk_error.is_some() || self.has_conflict() {
            return false;
        }
        if !self.dirty {
            return true;
        }
        self.write_if_unchanged(&self.saved_text.clone())
    }

    fn write_if_unchanged(&mut self, expected: &str) -> bool {
        let result = (|| -> std::io::Result<bool> {
            // Do not create missing files or truncate before validating the contents.
            // Validate on the same handle we write, narrowing the check/write race.
            // Uncooperative writers can still race this operation; this is not an
            // inter-process transaction or lock.
            let mut file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.path)?;
            let mut current = String::new();
            file.read_to_string(&mut current)?;
            if current != expected {
                self.observe_disk(current);
                return Ok(!self.dirty);
            }
            file.rewind()?;
            file.write_all(self.text.as_bytes())?;
            file.set_len(self.text.len() as u64)?;
            file.flush()?;
            Ok(true)
        })();
        match result {
            Ok(true) => {
                self.mark_synced();
                true
            }
            Ok(false) => false,
            Err(error) => {
                self.dirty_since = None;
                self.save_error = Some(format!("Could not save {}: {error}", self.path.display()));
                false
            }
        }
    }

    fn use_disk(&mut self) {
        // Read again rather than accepting a potentially stale conflict snapshot.
        if let Some(text) = self.read_disk() {
            self.replace_from_disk(text);
        }
    }

    pub(super) fn show_disk_conflict(&mut self, context: &egui::Context) {
        if !self.has_conflict() {
            return;
        }
        egui::TopBottomPanel::top("disk_conflict")
            .frame(pane_frame(DEEP))
            .show(context, |ui| {
                ui.colored_label(AMBER, "This file changed outside Glyphwire. Your local edits are preserved; autosave is paused.");
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Use disk (discard edits)")
                        .on_hover_text("Reload the latest disk contents and discard your local changes").clicked() {
                        self.use_disk();
                    }
                    if ui.button("Keep my edits…")
                        .on_hover_text("Confirm overwriting the external changes with your local version").clicked() {
                        self.confirm_overwrite = self.external_text.clone();
                    }
                });
            });
    }

    pub(super) fn show_overwrite_confirmation(&mut self, context: &egui::Context) {
        if self.confirm_overwrite.is_none() {
            return;
        }
        let response = egui::Modal::new(egui::Id::new(&self.path).with("overwrite_disk"))
            .show(context, |ui| {
                ui.set_max_width(480.0);
                ui.heading("Overwrite external changes?");
                ui.label(self.path.display().to_string());
                ui.colored_label(DANGER, "This replaces the disk version with your current local edits. External changes will be lost.");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm_overwrite = None;
                    }
                    if ui.button("Overwrite disk").clicked() {
                        if let Some(expected) = self.confirm_overwrite.take() {
                            // If the disk changed again during confirmation, update the
                            // conflict instead of overwriting the newer, unseen version.
                            self.write_if_unchanged(&expected);
                        }
                    }
                });
            });
        if response.should_close() {
            self.confirm_overwrite = None;
        }
    }
}
