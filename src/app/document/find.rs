use std::ops::Range;

use super::*;

#[derive(Default)]
pub(super) struct Find {
    pub open: bool,
    query: String,
    pub matches: Vec<Range<usize>>,
    pub current: usize,
    focus: bool,
    pub scroll_to_match: bool,
}

impl Find {
    pub fn refresh(&mut self, text: &str) {
        self.matches = if self.query.is_empty() {
            Vec::new()
        } else {
            regex::RegexBuilder::new(&regex::escape(&self.query))
                .case_insensitive(true)
                .build()
                .map(|pattern| pattern.find_iter(text).map(|found| found.range()).collect())
                .unwrap_or_default()
        };
        self.current = self.current.min(self.matches.len().saturating_sub(1));
    }

    fn advance(&mut self, backwards: bool) {
        if !self.matches.is_empty() {
            let count = self.matches.len();
            self.current = (self.current + if backwards { count - 1 } else { 1 }) % count;
            self.scroll_to_match = true;
        }
    }

    pub fn layout(&self, text: &str, font_id: FontId, width: f32) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = width;
        let format = egui::TextFormat {
            font_id,
            color: TEXT,
            ..Default::default()
        };
        let mut end = 0;
        if self.open {
            for (index, range) in self.matches.iter().enumerate() {
                // TextEdit can invoke the layouter with newly edited text before refresh.
                let Some(found) = text.get(range.clone()).filter(|_| range.start >= end) else {
                    continue;
                };
                job.append(&text[end..range.start], 0.0, format.clone());
                let mut highlight = format.clone();
                highlight.background = if index == self.current {
                    Color32::from_rgb(87, 69, 35)
                } else {
                    Color32::from_rgb(48, 50, 56)
                };
                job.append(found, 0.0, highlight);
                end = range.end;
            }
        }
        job.append(&text[end..], 0.0, format);
        job
    }
}

impl Document {
    pub fn open_find(&mut self) {
        self.find.open = true;
        self.find.focus = true;
        self.find.refresh(&self.text);
        self.find.scroll_to_match = true;
        self.show_raw = true;
    }

    pub fn handle_find_shortcuts(&mut self, context: &egui::Context) {
        let shortcut = |modifiers, key| {
            context.input_mut(|input| {
                input.consume_shortcut(&egui::KeyboardShortcut::new(modifiers, key))
            })
        };
        if shortcut(egui::Modifiers::COMMAND, egui::Key::F) {
            self.open_find();
        }
        if !self.find.open {
            return;
        }
        if shortcut(egui::Modifiers::NONE, egui::Key::Escape) {
            self.find.open = false;
            context.memory_mut(|memory| memory.surrender_focus(self.find_id()));
            return;
        }
        let focused = context.memory(|memory| memory.has_focus(self.find_id()));
        if shortcut(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::G,
        ) || (focused && shortcut(egui::Modifiers::SHIFT, egui::Key::Enter))
        {
            self.find.advance(true);
        } else if shortcut(egui::Modifiers::COMMAND, egui::Key::G)
            || (focused && shortcut(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.find.advance(false);
        }
    }

    fn find_id(&self) -> egui::Id {
        egui::Id::new(&self.path).with("find_query")
    }

    pub(super) fn show_find(&mut self, context: &egui::Context) {
        if !self.find.open {
            return;
        }
        egui::TopBottomPanel::top("document_find")
            .frame(pane_frame(DEEP))
            .show(context, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find in source");
                    let id = self.find_id();
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.find.query)
                            .id(id)
                            .hint_text("Search document…")
                            .desired_width(240.0),
                    );
                    if self.find.focus {
                        input.request_focus();
                        // Reopening Find selects the previous query for easy replacement.
                        if let Some(mut state) = egui::TextEdit::load_state(context, id) {
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(self.find.query.chars().count()),
                                )));
                            state.store(context, id);
                        }
                        self.find.focus = false;
                    }
                    if input.changed() {
                        self.find.current = 0;
                        self.find.refresh(&self.text);
                        self.find.scroll_to_match = true;
                    }
                    let count = self.find.matches.len();
                    ui.label(if self.find.query.is_empty() {
                        "Case-insensitive".to_owned()
                    } else if count == 0 {
                        "No matches".to_owned()
                    } else {
                        format!("{} of {count}", self.find.current + 1)
                    });
                    if ui
                        .add_enabled(count > 0, egui::Button::new("Previous"))
                        .on_hover_text("Shift+Enter / Cmd+Shift+G")
                        .clicked()
                    {
                        self.find.advance(true);
                    }
                    if ui
                        .add_enabled(count > 0, egui::Button::new("Next"))
                        .on_hover_text("Enter / Cmd+G")
                        .clicked()
                    {
                        self.find.advance(false);
                    }
                    if ui.button("×").on_hover_text("Close search (Esc)").clicked() {
                        self.find.open = false;
                    }
                });
            });
        if self.find.scroll_to_match && !self.find.matches.is_empty() {
            self.show_raw = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_unicode_search_counts_and_wraps() {
        let mut find = Find {
            query: "café.*".into(),
            ..Default::default()
        };
        let text = "🦀 CAFÉ.* then café.* and cafe";
        find.refresh(text);
        assert_eq!(find.matches.len(), 2);
        assert_eq!(&text[find.matches[0].clone()], "CAFÉ.*");
        find.advance(true);
        assert_eq!(find.current, 1);
        find.advance(false);
        assert_eq!(find.current, 0);
        find.refresh("nothing");
        assert!(find.matches.is_empty());
        assert_eq!(find.current, 0);
        find.advance(true);
        find.query.clear();
        find.refresh(text);
        assert!(find.matches.is_empty());
    }

    #[test]
    fn layout_highlights_matches_and_tolerates_edits() {
        let mut find = Find {
            open: true,
            query: "a".into(),
            ..Default::default()
        };
        find.refresh("a a");
        let job = find.layout("a a", FontId::monospace(16.0), 400.0);
        assert_eq!(job.text, "a a");
        assert_eq!(
            job.sections
                .iter()
                .filter(|section| section.format.background != Color32::TRANSPARENT)
                .count(),
            2
        );
        for changed in ["", "🦀", "éa", "aaaa"] {
            let job = find.layout(changed, FontId::monospace(16.0), 400.0);
            assert_eq!(job.text, changed);
        }
    }
}
