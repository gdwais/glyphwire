use super::*;

fn poll(document: &mut Document, context: &egui::Context) {
    document.disk_checked_at = Instant::now() - Duration::from_secs(2);
    document.save_if_due(context);
}

fn edit(document: &mut Document, text: &str, context: &egui::Context) {
    document.text = text.into();
    document.schedule_save(context);
}

fn resolve(document: &mut Document, context: &egui::Context, button: &str) {
    let output = frame(context, vec![], |context| document.show(context));
    click(context, text_center(&output, button), |context| {
        document.show(context)
    });
}

fn confirm(document: &mut Document, context: &egui::Context, button: &str) {
    // Give the modal a frame to settle its position.
    frame(context, vec![], |context| document.show(context));
    resolve(document, context, button);
}

#[test]
fn clean_tabs_reload_content_and_atomic_replacements_without_losing_view_state() {
    let directory = TestDirectory::new();
    let first = directory.file("first.md", "First");
    let second = directory.file("second.md", "Second");
    let mut app = GlyphwireApp::open(first.clone());
    app.documents[0].show_raw = false;
    app.documents[0].scroll_fraction = 0.6;
    app.load_file(second.clone());
    let modified = fs::metadata(&first).unwrap().modified().unwrap();
    fs::write(&first, "Other").unwrap(); // same byte length and timestamp
    fs::File::open(&first)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let replacement = directory.file("replacement.md", "Atomic replacement 🦀");
    fs::rename(replacement, &second).unwrap();
    let context = egui::Context::default();
    for document in &mut app.documents {
        poll(document, &context);
    }
    assert_eq!(app.active, Some(1));
    assert_eq!(app.documents[0].text, "Other");
    assert_eq!(app.documents[1].text, "Atomic replacement 🦀");
    assert!(!app.documents[0].show_raw);
    assert_eq!(app.documents[0].scroll_fraction, 0.6);
    for document in &app.documents {
        assert!(!document.is_dirty());
        assert!(!document.has_conflict());
        assert!(document.error().is_none());
        assert_eq!(document.saved_text, document.text);
    }
}

#[test]
fn autosave_detects_external_changes_before_the_next_poll_and_does_not_overwrite() {
    let directory = TestDirectory::new();
    let path = directory.file("notes.md", "Original");
    let mut document = Document::open(path.clone()).unwrap();
    let context = egui::Context::default();
    edit(&mut document, "Local edits", &context);
    fs::write(&path, "External edits").unwrap();
    document.dirty_since = Some(Instant::now() - AUTO_SAVE_DELAY);
    document.save_if_due(&context);
    assert!(document.has_conflict());
    assert!(document.is_dirty());
    assert_eq!(document.text, "Local edits");
    assert_eq!(fs::read_to_string(&path).unwrap(), "External edits");
    assert!(document.dirty_since.is_none());
    assert!(!document.save_now());
    edit(&mut document, "More local edits", &context);
    poll(&mut document, &context);
    assert!(document.has_conflict());
    assert_eq!(document.text, "More local edits");
    assert_eq!(fs::read_to_string(path).unwrap(), "External edits");
}

#[test]
fn close_and_save_all_block_conflicts_but_save_other_tabs() {
    let directory = TestDirectory::new();
    let first = directory.file("first.md", "Original");
    let second = directory.file("second.md", "Second");
    let mut app = GlyphwireApp::open(first.clone());
    let context = egui::Context::default();
    edit(&mut app.documents[0], "Local", &context);
    fs::write(&first, "External").unwrap();
    app.load_file(second.clone());
    edit(&mut app.documents[1], "Other local", &context);
    assert!(!app.save_all());
    assert_eq!(app.active, Some(0));
    assert_eq!(fs::read_to_string(second).unwrap(), "Other local");
    app.close_tab(0);
    assert_eq!(app.documents.len(), 2);
    assert!(app.documents[0].has_conflict());
    assert_eq!(fs::read_to_string(first).unwrap(), "External");
}

#[test]
fn use_disk_discards_local_edits_and_reads_the_latest_disk_version() {
    let directory = TestDirectory::new();
    let path = directory.file("notes.md", "Original");
    let mut document = Document::open(path.clone()).unwrap();
    let context = egui::Context::default();
    edit(&mut document, "Local", &context);
    fs::write(&path, "External").unwrap();
    poll(&mut document, &context);
    fs::write(&path, "Even newer external").unwrap();
    resolve(&mut document, &context, "Use disk (discard edits)");
    assert_eq!(document.text, "Even newer external");
    assert!(!document.is_dirty());
    assert!(!document.has_conflict());
    assert!(document.error().is_none());
    edit(&mut document, "Edit after reload", &context);
    assert!(document.save_now());
    assert_eq!(fs::read_to_string(path).unwrap(), "Edit after reload");
}

#[test]
fn keep_local_requires_confirmation_and_saves_only_the_approved_disk_version() {
    let directory = TestDirectory::new();
    let path = directory.file("notes.md", "Original");
    let mut document = Document::open(path.clone()).unwrap();
    let context = egui::Context::default();
    edit(&mut document, "Local", &context);
    fs::write(&path, "External").unwrap();
    poll(&mut document, &context);
    resolve(&mut document, &context, "Keep my edits…");
    assert!(document.has_sync_dialog());
    assert_eq!(fs::read_to_string(&path).unwrap(), "External");
    confirm(&mut document, &context, "Cancel");
    assert!(!document.has_sync_dialog());
    assert!(document.has_conflict());
    resolve(&mut document, &context, "Keep my edits…");
    // Even when a poll refreshes the warning, the approval snapshot stays frozen.
    fs::write(&path, "Newer external").unwrap();
    poll(&mut document, &context);
    confirm(&mut document, &context, "Overwrite disk");
    assert_eq!(fs::read_to_string(&path).unwrap(), "Newer external");
    assert_eq!(document.text, "Local");
    assert!(document.has_conflict());
    resolve(&mut document, &context, "Keep my edits…");
    confirm(&mut document, &context, "Overwrite disk");
    assert_eq!(fs::read_to_string(&path).unwrap(), "Local");
    assert!(!document.has_conflict());
    assert!(!document.is_dirty());
    poll(&mut document, &context);
    assert!(
        !document.has_conflict(),
        "Our own save must not appear as an external change"
    );
    assert!(document.error().is_none());
}

#[test]
fn missing_or_invalid_files_preserve_the_buffer_and_recover_when_restored() {
    let directory = TestDirectory::new();
    let path = directory.file("notes.md", "Original");
    let mut document = Document::open(path.clone()).unwrap();
    let context = egui::Context::default();
    fs::remove_file(&path).unwrap();
    poll(&mut document, &context);
    assert_eq!(document.text, "Original");
    assert!(document.error().is_some());
    edit(&mut document, "Local", &context);
    assert!(!document.save_now());
    assert!(!path.exists());
    fs::write(&path, [0xff, 0xfe]).unwrap();
    poll(&mut document, &context);
    assert_eq!(document.text, "Local");
    assert!(!document.save_now());
    assert_eq!(fs::read(&path).unwrap(), [0xff, 0xfe]);
    fs::write(&path, "Original").unwrap();
    poll(&mut document, &context);
    assert!(document.error().is_none());
    assert!(!document.has_conflict());
    assert!(document.dirty_since.is_some());
    document.dirty_since = Some(Instant::now() - AUTO_SAVE_DELAY);
    document.save_if_due(&context);
    assert_eq!(fs::read_to_string(path).unwrap(), "Local");
}

#[test]
fn identical_or_reverted_changes_do_not_produce_false_conflicts() {
    let directory = TestDirectory::new();
    let path = directory.file("notes.md", "Original");
    let mut document = Document::open(path.clone()).unwrap();
    let context = egui::Context::default();
    edit(&mut document, "Same edits", &context);
    fs::write(&path, "Same edits").unwrap();
    assert!(document.save_now());
    assert!(!document.is_dirty());
    assert!(!document.has_conflict());
    edit(&mut document, "Local", &context);
    fs::write(&path, "External").unwrap();
    poll(&mut document, &context);
    assert!(document.has_conflict());
    fs::write(&path, "Same edits").unwrap();
    poll(&mut document, &context);
    assert!(!document.has_conflict());
    assert!(document.dirty_since.is_some());
    assert!(document.save_now());
    assert_eq!(fs::read_to_string(&path).unwrap(), "Local");
    edit(&mut document, "Will undo", &context);
    edit(&mut document, "Local", &context);
    assert!(!document.is_dirty());
    fs::write(&path, "Latest external").unwrap();
    poll(&mut document, &context);
    assert_eq!(document.text, "Latest external");
}

#[test]
fn reload_refreshes_search_and_clears_stale_editor_undo_state() {
    let directory = TestDirectory::new();
    let path = directory.file("notes.md", "hello");
    let mut document = Document::open(path.clone()).unwrap();
    let context = egui::Context::default();
    document.open_find();
    frame(&context, vec![], |context| document.show(context));
    frame(
        &context,
        vec![egui::Event::Text("hello".into())],
        |context| document.show(context),
    );
    assert_eq!(document.find.matches.len(), 1);
    let editor_id = egui::Id::new(&path).with("editor");
    let mut state = egui::TextEdit::load_state(&context, editor_id).unwrap();
    let cursor = egui::text::CCursorRange::one(egui::text::CCursor::new(0));
    let mut undoer = state.undoer();
    undoer.add_undo(&(cursor, "old buffer".into()));
    assert!(undoer.has_undo(&(cursor, document.text.clone())));
    state.set_undoer(undoer);
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(999),
        )));
    state.store(&context, editor_id);
    fs::write(&path, "hello hello 🦀").unwrap();
    poll(&mut document, &context);
    assert_eq!(document.find.matches.len(), 2);
    assert!(document.reset_editor_state);
    frame(&context, vec![], |context| document.show(context));
    assert!(!document.reset_editor_state);
    assert_eq!(document.text, "hello hello 🦀");
    assert!(!document.is_dirty());
    let state = egui::TextEdit::load_state(&context, editor_id).unwrap();
    let current = (
        state.cursor.char_range().unwrap_or(cursor),
        document.text.clone(),
    );
    assert!(!state.undoer().has_undo(&current));
}
