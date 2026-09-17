use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "glyphwire-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }

    fn file(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn frame(
    context: &egui::Context,
    events: Vec<egui::Event>,
    mut show: impl FnMut(&egui::Context),
) -> egui::FullOutput {
    context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1480.0, 920.0),
            )),
            events,
            ..Default::default()
        },
        |context| show(context),
    )
}

fn click(
    context: &egui::Context,
    pos: egui::Pos2,
    mut show: impl FnMut(&egui::Context),
) -> egui::FullOutput {
    frame(
        context,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut show,
    );
    frame(
        context,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        show,
    )
}

fn text_center(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.visual_bounding_rect().center())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing UI text: {text}"))
}

#[test]
fn tabs_preserve_buffers_visibility_scroll_and_autosave_in_background() {
    let directory = TestDirectory::new();
    let first = directory.file("first.md", "First");
    let second = directory.file("second.md", "Second");
    let context = egui::Context::default();
    let mut app = GlyphwireApp::open(first.clone());
    app.documents[0].text.push_str(" edited");
    app.documents[0].show_raw = false;
    app.documents[0].scroll_fraction = 0.6;
    app.documents[0].schedule_save(&context);
    app.documents[0].dirty_since = Some(Instant::now() - AUTO_SAVE_DELAY);
    app.load_file(second.clone());
    app.documents[1].text.push_str(" edited too");
    app.documents[1].schedule_save(&context);
    app.documents[1].dirty_since = Some(Instant::now() - AUTO_SAVE_DELAY);
    for document in &mut app.documents {
        document.save_if_due(&context);
    }
    assert_eq!(fs::read_to_string(&first).unwrap(), "First edited");
    assert_eq!(fs::read_to_string(second).unwrap(), "Second edited too");
    app.load_file(first);
    assert_eq!(app.documents.len(), 2);
    assert_eq!(app.active, Some(0));
    assert!(!app.documents[0].show_raw);
    assert_eq!(app.documents[0].scroll_fraction, 0.6);
    assert_eq!(app.documents[0].text, "First edited");
}

#[test]
fn closing_tabs_saves_and_keeps_a_valid_active_index() {
    let directory = TestDirectory::new();
    let first = directory.file("first.md", "First");
    let second = directory.file("second.md", "Second");
    let third = directory.file("third.md", "Third");
    let mut app = GlyphwireApp::open(first.clone());
    app.documents[0].text = "Saved on close".into();
    app.documents[0].schedule_save(&egui::Context::default());
    app.load_file(second);
    app.load_file(third.clone());
    app.close_tab(0);
    assert_eq!(app.active, Some(1));
    assert_eq!(app.active_document().unwrap().path, third);
    assert_eq!(fs::read_to_string(first).unwrap(), "Saved on close");
    app.close_tab(1);
    assert_eq!(app.active, Some(0));
    app.close_tab(0);
    assert_eq!(app.active, None);
    assert!(app.documents.is_empty());
}

#[test]
fn save_failure_keeps_tab_and_unsaved_text_and_does_not_block_other_saves() {
    let directory = TestDirectory::new();
    let first = directory.file("first.md", "First");
    let second = directory.file("second.md", "Second");
    let context = egui::Context::default();
    let mut app = GlyphwireApp::open(first.clone());
    app.documents[0].text = "Unsaved".into();
    app.documents[0].schedule_save(&context);
    fs::remove_file(&first).unwrap();
    app.load_file(second.clone());
    app.documents[1].text = "Saved".into();
    app.documents[1].schedule_save(&context);
    assert!(!app.save_all());
    assert_eq!(fs::read_to_string(second).unwrap(), "Saved");
    app.close_tab(0);
    assert_eq!(app.documents.len(), 2);
    assert_eq!(app.active, Some(0));
    assert_eq!(app.documents[0].text, "Unsaved");
    assert!(app.documents[0].is_dirty());
    assert!(app.documents[0].error().is_some());
    assert!(
        !first.exists(),
        "Autosave must not resurrect a deleted file"
    );
}

#[test]
fn deleting_an_open_dirty_file_closes_it_without_recreating_it() {
    let directory = TestDirectory::new();
    let path = directory.file("delete.md", "Original");
    let mut app = GlyphwireApp::open(path.clone());
    app.documents[0].text = "Unsaved".into();
    app.documents[0].schedule_save(&egui::Context::default());
    app.delete_file(&path);
    assert!(!path.exists());
    assert!(app.documents.is_empty());
    assert_eq!(app.active, None);
    assert!(app.save_all());
    assert!(!path.exists());
    let plain = directory.file("other.txt", "Not Markdown");
    app.delete_file(&plain);
    assert!(!plain.exists());
}

#[test]
fn failed_delete_preserves_open_document() {
    let directory = TestDirectory::new();
    let path = directory.file("keep.md", "Keep me");
    let mut app = GlyphwireApp::open(path.clone());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap(); // remove_file must refuse to remove a directory
    app.delete_file(&path);
    assert!(app.error.is_some());
    assert_eq!(app.documents.len(), 1);
    assert_eq!(app.active_document().unwrap().text, "Keep me");
    assert!(path.is_dir());
}

#[cfg(unix)]
#[test]
fn deleting_a_symlink_leaves_target_and_its_buffer_intact() {
    let directory = TestDirectory::new();
    let target = directory.file("target.md", "Keep me");
    let link = directory.0.join("link.md");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let mut app = GlyphwireApp::open(target.clone());
    app.load_file(link.clone());
    assert_eq!(app.documents.len(), 1);
    app.delete_file(&link);
    assert!(target.exists());
    assert!(!link.exists());
    assert_eq!(app.documents.len(), 1);
}

#[test]
fn copy_button_copies_complete_unsaved_markdown_with_raw_hidden() {
    let directory = TestDirectory::new();
    let path = directory.file("copy.md", "Original");
    let mut app = GlyphwireApp::open(path);
    let text = "# Café\n\n- [x] Done\n- [ ] 待つ\n\n```rs\nlet x = 1;\n```\n";
    app.documents[0].text = text.into();
    app.documents[0].show_raw = false;
    let context = egui::Context::default();
    configure_theme(&context);
    let output = frame(&context, vec![], |context| app.show_toolbar(context));
    let pos = text_center(&output, "Copy Markdown");
    let output = click(&context, pos, |context| app.show_toolbar(context));
    assert!(output.platform_output.commands.iter().any(|command| {
        matches!(command, egui::OutputCommand::CopyText(copied) if copied == text)
    }));
}

#[test]
fn raw_pane_can_be_hidden_and_reopened_without_losing_text() {
    let directory = TestDirectory::new();
    let path = directory.file("raw.md", "# Hello");
    let mut document = Document::open(path).unwrap();
    let context = egui::Context::default();
    configure_theme(&context);
    let output = frame(&context, vec![], |context| document.show(context));
    click(&context, text_center(&output, "Hide raw"), |context| {
        document.show(context)
    });
    assert!(!document.show_raw);
    let output = frame(&context, vec![], |context| document.show(context));
    click(&context, text_center(&output, "Show raw"), |context| {
        document.show(context)
    });
    assert!(document.show_raw);
    assert_eq!(document.text, "# Hello");
}

#[test]
fn preview_checkbox_changes_only_its_marker_and_autosaves_both_directions() {
    let directory = TestDirectory::new();
    let original =
        "# Café 🦀\r\n\r\n- [ ] Task\r\n  - [X] Nested\r\n\r\n```md\r\n- [ ] Not a task\r\n```\r\n";
    let path = directory.file("tasks.md", original);
    let mut document = Document::open(path.clone()).unwrap();
    document.show_raw = false;
    let context = egui::Context::default();
    configure_theme(&context);
    let output = frame(&context, vec![], |context| document.show(context));
    let icon_width = context.style().spacing.icon_width;
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(shape)
                if (shape.rect.width() - icon_width).abs() < 0.1
                    && (shape.rect.height() - icon_width).abs() < 0.1 =>
            {
                Some(shape.rect.center())
            }
            _ => None,
        })
        .expect("Rendered checkbox");
    click(&context, pos, |context| document.show(context));
    assert_eq!(document.text, original.replacen("[ ]", "[x]", 1));
    assert!(document.is_dirty());
    document.dirty_since = Some(Instant::now() - AUTO_SAVE_DELAY);
    document.save_if_due(&context);
    assert_eq!(fs::read_to_string(&path).unwrap(), document.text);
    click(&context, pos, |context| document.show(context));
    assert_eq!(document.text, original);
    assert!(document.save_now());
    assert_eq!(fs::read_to_string(path).unwrap(), original);
}

#[test]
fn file_browser_right_click_requests_confirmation_without_deleting() {
    let directory = TestDirectory::new();
    let path = directory.file("plain.txt", "Not Markdown");
    let context = egui::Context::default();
    let mut open = None;
    let mut delete = None;
    {
        let mut show = |context: &egui::Context| {
            egui::CentralPanel::default().show(context, |ui| {
                render_directory(ui, &directory.0, None, &mut open, &mut delete);
            });
        };
        let output = frame(&context, vec![], &mut show);
        let pos = text_center(&output, "plain.txt");
        for pressed in [true, false] {
            frame(
                &context,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                &mut show,
            );
        }
        let output = frame(&context, vec![], &mut show);
        click(&context, text_center(&output, "Delete file…"), &mut show);
    }
    assert_eq!(delete, Some(path.clone()));
    assert_eq!(open, None);
    assert!(
        path.exists(),
        "Context menu must not delete before confirmation"
    );
}

#[test]
fn unreadable_file_does_not_replace_current_tab() {
    let directory = TestDirectory::new();
    let path = directory.file("keep.md", "Keep me");
    let mut app = GlyphwireApp::open(path);
    app.load_file(directory.0.join("missing.md"));
    assert_eq!(app.documents.len(), 1);
    assert_eq!(app.active, Some(0));
    assert_eq!(app.active_document().unwrap().text, "Keep me");
    assert!(app.error.is_some());
}

#[test]
fn delete_confirmation_can_be_cancelled_or_confirmed() {
    let directory = TestDirectory::new();
    let path = directory.file("delete.md", "Keep until confirmed");
    let mut app = GlyphwireApp::open(path.clone());
    let context = egui::Context::default();
    app.pending_delete = Some(path.clone());
    // Modal areas settle their position after the first frame.
    frame(&context, vec![], |context| {
        app.show_delete_confirmation(context)
    });
    let output = frame(&context, vec![], |context| {
        app.show_delete_confirmation(context)
    });
    click(&context, text_center(&output, "Cancel"), |context| {
        app.show_delete_confirmation(context)
    });
    assert!(app.pending_delete.is_none());
    assert!(path.exists());
    assert_eq!(app.documents.len(), 1);
    app.pending_delete = Some(path.clone());
    let output = frame(&context, vec![], |context| {
        app.show_delete_confirmation(context)
    });
    click(&context, text_center(&output, "Delete file"), |context| {
        app.show_delete_confirmation(context)
    });
    assert!(app.pending_delete.is_none());
    assert!(!path.exists());
    assert!(app.documents.is_empty());
}
