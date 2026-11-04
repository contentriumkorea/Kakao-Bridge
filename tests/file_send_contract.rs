#[allow(dead_code)]
#[path="../src/file_send.rs"]
mod file_send;

#[test]
fn missing_or_relative_files_are_rejected_before_touching_windows() {
    assert_eq!(file_send::attempt(0,0,0,"synthetic","relative.txt"),Err("INVALID_FILE"));
    assert_eq!(file_send::attempt(0,0,0,"synthetic","C:\\synthetic\\bad\0file"),Err("INVALID_FILE"));
}
#[allow(dead_code)]
#[path="../src/shortcut.rs"]
mod shortcut;

#[allow(dead_code)]
#[path="../src/popup_ocr.rs"] mod popup_ocr;
