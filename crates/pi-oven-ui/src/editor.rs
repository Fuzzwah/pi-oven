pub struct InputEditor {
    buf: String,
    cursor: usize,
    anchor: Option<usize>,
}

impl Default for InputEditor {
    fn default() -> Self {
        Self { buf: String::new(), cursor: 0, anchor: None }
    }
}

impl InputEditor {
    pub fn text(&self) -> &str {
        &self.buf
    }

    pub fn cursor_byte_pos(&self) -> usize {
        self.cursor
    }

    pub fn selection(&self) -> Option<(usize, usize)> {
        self.anchor.map(|anchor| {
            if self.cursor <= anchor { (self.cursor, anchor) } else { (anchor, self.cursor) }
        })
    }

    pub fn push_str(&mut self, s: &str) {
        self.buf.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.cursor = self.clamp_to_char_boundary(self.cursor);
        self.anchor = None;
    }

    pub fn move_right(&mut self, extend: bool) {
        if !extend {
            if let Some(anchor) = self.anchor {
                self.cursor = self.cursor.max(anchor);
                self.cursor = self.clamp_to_char_boundary(self.cursor);
                self.anchor = None;
                return;
            }
        }
        if extend && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        }
        if self.cursor < self.buf.len() {
            let ch = self.buf[self.cursor..].chars().next().unwrap();
            self.cursor += ch.len_utf8();
            self.cursor = self.clamp_to_char_boundary(self.cursor);
        }
    }

    pub fn move_left(&mut self, extend: bool) {
        if !extend {
            if let Some(anchor) = self.anchor {
                self.cursor = self.cursor.min(anchor);
                self.cursor = self.clamp_to_char_boundary(self.cursor);
                self.anchor = None;
                return;
            }
        }
        if extend && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        }
        if self.cursor > 0 {
            let mut pos = self.cursor - 1;
            while pos > 0 && !self.buf.is_char_boundary(pos) {
                pos -= 1;
            }
            self.cursor = pos;
        }
    }

    pub fn move_word_right(&mut self, extend: bool) {
        if extend && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !extend {
            self.anchor = None;
        }
        let text = &self.buf[self.cursor..];
        let mut offset = 0;
        for c in text.chars() {
            if c.is_whitespace() { offset += c.len_utf8(); } else { break; }
        }
        for c in text[offset..].chars() {
            if !c.is_whitespace() { offset += c.len_utf8(); } else { break; }
        }
        self.cursor += offset;
        self.cursor = self.clamp_to_char_boundary(self.cursor);
    }

    pub fn move_word_left(&mut self, extend: bool) {
        if extend && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !extend {
            self.anchor = None;
        }
        let mut offset = self.cursor;
        for c in self.buf[..self.cursor].chars().rev() {
            if c.is_whitespace() { offset -= c.len_utf8(); } else { break; }
        }
        for c in self.buf[..offset].chars().rev() {
            if !c.is_whitespace() { offset -= c.len_utf8(); } else { break; }
        }
        self.cursor = offset;
        self.cursor = self.clamp_to_char_boundary(self.cursor);
    }

    pub fn move_to_start(&mut self, extend: bool) {
        if extend && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !extend {
            self.anchor = None;
        }
        self.cursor = 0;
    }

    pub fn move_to_end(&mut self, extend: bool) {
        if extend && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !extend {
            self.anchor = None;
        }
        self.cursor = self.buf.len();
    }

    pub fn delete_before(&mut self) {
        if let Some(anchor) = self.anchor {
            let (start, end) =
                if self.cursor <= anchor { (self.cursor, anchor) } else { (anchor, self.cursor) };
            self.buf.drain(start..end);
            self.cursor = start;
            self.anchor = None;
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let mut pos = self.cursor - 1;
        while pos > 0 && !self.buf.is_char_boundary(pos) {
            pos -= 1;
        }
        self.buf.drain(pos..self.cursor);
        self.cursor = pos;
    }

    pub fn delete_after(&mut self) {
        if self.cursor >= self.buf.len() {
            return;
        }
        let ch = self.buf[self.cursor..].chars().next().unwrap();
        self.buf.drain(self.cursor..self.cursor + ch.len_utf8());
    }

    pub fn delete_word_before(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut offset = self.cursor;
        for c in self.buf[..self.cursor].chars().rev() {
            if c.is_whitespace() { offset -= c.len_utf8(); } else { break; }
        }
        let offset2 = offset;
        for c in self.buf[..offset2].chars().rev() {
            if !c.is_whitespace() { offset -= c.len_utf8(); } else { break; }
        }
        self.buf.drain(offset..self.cursor);
        self.cursor = offset;
        self.anchor = None;
    }

    pub fn delete_to_start(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.buf.drain(0..self.cursor);
        self.cursor = 0;
        self.anchor = None;
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection().map(|(start, end)| self.buf[start..end].to_string())
    }

    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection() {
            self.buf.drain(start..end);
            self.cursor = start;
            self.anchor = None;
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.cursor = 0;
        self.anchor = None;
    }

    fn clamp_to_char_boundary(&self, pos: usize) -> usize {
        let len = self.buf.len();
        if pos >= len {
            return len;
        }
        let mut p = pos;
        while p > 0 && !self.buf.is_char_boundary(p) {
            p -= 1;
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 2.1 push_str ──────────────────────────────────────────────────────────

    #[test]
    fn push_str_ascii_advances_cursor() {
        let mut ed = InputEditor::default();
        ed.push_str("hi");
        assert_eq!(ed.text(), "hi");
        assert_eq!(ed.cursor_byte_pos(), 2);
    }

    #[test]
    fn push_str_multibyte_cursor_on_boundary() {
        let mut ed = InputEditor::default();
        ed.push_str("é"); // U+00E9, 2 bytes in UTF-8
        assert_eq!(ed.cursor_byte_pos(), 2);
        assert!(ed.buf.is_char_boundary(ed.cursor));
    }

    #[test]
    fn push_str_inserts_at_cursor_not_end() {
        let mut ed = InputEditor::default();
        ed.push_str("ac");
        ed.move_left(false); // cursor at 1 (between 'a' and 'c')
        ed.push_str("b");
        assert_eq!(ed.text(), "abc");
        assert_eq!(ed.cursor_byte_pos(), 2);
    }

    // ── 2.2 move_right / move_left ────────────────────────────────────────────

    #[test]
    fn move_right_advances_one_char() {
        let mut ed = InputEditor::default();
        ed.push_str("ab");
        ed.move_to_start(false);
        ed.move_right(false);
        assert_eq!(ed.cursor_byte_pos(), 1);
    }

    #[test]
    fn move_right_at_end_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("x");
        ed.move_right(false);
        assert_eq!(ed.cursor_byte_pos(), 1);
    }

    #[test]
    fn move_left_retreats_one_char() {
        let mut ed = InputEditor::default();
        ed.push_str("ab");
        ed.move_left(false);
        assert_eq!(ed.cursor_byte_pos(), 1);
    }

    #[test]
    fn move_left_at_start_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("x");
        ed.move_to_start(false);
        ed.move_left(false);
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn move_right_multibyte() {
        let mut ed = InputEditor::default();
        ed.push_str("é"); // 2 bytes
        ed.move_to_start(false);
        ed.move_right(false);
        assert_eq!(ed.cursor_byte_pos(), 2);
        assert!(ed.buf.is_char_boundary(ed.cursor));
    }

    #[test]
    fn move_left_multibyte() {
        let mut ed = InputEditor::default();
        ed.push_str("é"); // 2 bytes, cursor at 2
        ed.move_left(false);
        assert_eq!(ed.cursor_byte_pos(), 0);
        assert!(ed.buf.is_char_boundary(ed.cursor));
    }

    #[test]
    fn move_right_collapses_selection_to_end() {
        let mut ed = InputEditor::default();
        ed.push_str("abc");
        ed.move_to_start(false);
        ed.move_right(true); // select 'a'
        ed.move_right(false); // collapse → cursor at end of selection = 1
        assert_eq!(ed.cursor_byte_pos(), 1);
        assert!(ed.anchor.is_none());
    }

    #[test]
    fn move_left_collapses_selection_to_start() {
        let mut ed = InputEditor::default();
        ed.push_str("abc");
        ed.move_right(true); // select from end... no wait, cursor is at end after push_str
        // Let me set up: cursor at end, shift-left to select 'c'
        ed.move_to_end(false);
        ed.move_left(true); // anchor=3, cursor=2 → selection is 'c'
        ed.move_left(false); // collapse → cursor at start of selection = 2
        assert_eq!(ed.cursor_byte_pos(), 2);
        assert!(ed.anchor.is_none());
    }

    // ── 2.3 move_word_right / move_word_left ─────────────────────────────────

    #[test]
    fn move_word_right_from_start_of_word() {
        let mut ed = InputEditor::default();
        ed.push_str("hello world");
        ed.move_to_start(false);
        ed.move_word_right(false);
        assert_eq!(ed.cursor_byte_pos(), 5); // after "hello"
    }

    #[test]
    fn move_word_right_from_mid_word() {
        let mut ed = InputEditor::default();
        ed.push_str("hello world");
        ed.move_to_start(false);
        ed.move_right(false);
        ed.move_right(false); // cursor at 2 (inside "hello")
        ed.move_word_right(false);
        assert_eq!(ed.cursor_byte_pos(), 5); // end of "hello"
    }

    #[test]
    fn move_word_right_from_whitespace() {
        let mut ed = InputEditor::default();
        ed.push_str("hello world");
        ed.move_to_start(false);
        // move to the space (byte 5)
        for _ in 0..5 { ed.move_right(false); }
        ed.move_word_right(false);
        assert_eq!(ed.cursor_byte_pos(), 11); // after "world"
    }

    #[test]
    fn move_word_right_at_end_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("hi");
        ed.move_word_right(false);
        assert_eq!(ed.cursor_byte_pos(), 2);
    }

    #[test]
    fn move_word_left_from_mid_word() {
        let mut ed = InputEditor::default();
        ed.push_str("hello world");
        // move to byte 8 (inside "world")
        ed.move_to_start(false);
        for _ in 0..8 { ed.move_right(false); }
        ed.move_word_left(false);
        assert_eq!(ed.cursor_byte_pos(), 6); // start of "world"
    }

    #[test]
    fn move_word_left_from_whitespace() {
        let mut ed = InputEditor::default();
        ed.push_str("hello world");
        // move to the space (byte 5)
        ed.move_to_start(false);
        for _ in 0..5 { ed.move_right(false); }
        ed.move_word_left(false);
        assert_eq!(ed.cursor_byte_pos(), 0); // start of "hello"
    }

    #[test]
    fn move_word_left_at_start_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("hi");
        ed.move_to_start(false);
        ed.move_word_left(false);
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    // ── 2.4 move_to_start / move_to_end ──────────────────────────────────────

    #[test]
    fn move_to_start_goes_to_zero() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.move_to_start(false);
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn move_to_end_goes_to_buf_len() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.move_to_start(false);
        ed.move_to_end(false);
        assert_eq!(ed.cursor_byte_pos(), 5);
    }

    // ── 2.5 shift-selection ───────────────────────────────────────────────────

    #[test]
    fn shift_right_starts_selection() {
        let mut ed = InputEditor::default();
        ed.push_str("ab");
        ed.move_to_start(false);
        ed.move_right(true);
        assert_eq!(ed.anchor, Some(0));
        assert_eq!(ed.cursor_byte_pos(), 1);
    }

    #[test]
    fn shift_right_extends_existing_selection() {
        let mut ed = InputEditor::default();
        ed.push_str("abc");
        ed.move_to_start(false);
        ed.move_right(true); // anchor=0, cursor=1
        ed.move_right(true); // anchor unchanged=0, cursor=2
        assert_eq!(ed.anchor, Some(0));
        assert_eq!(ed.cursor_byte_pos(), 2);
    }

    #[test]
    fn non_shift_move_clears_selection() {
        let mut ed = InputEditor::default();
        ed.push_str("abc");
        ed.move_to_start(false);
        ed.move_right(true); // start selection
        assert!(ed.anchor.is_some());
        ed.move_right(false); // clear selection
        assert!(ed.anchor.is_none());
    }

    #[test]
    fn shift_to_start_sets_selection() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.move_to_start(true);
        assert_eq!(ed.anchor, Some(5));
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn shift_to_end_sets_selection() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.move_to_start(false);
        ed.move_to_end(true);
        assert_eq!(ed.anchor, Some(0));
        assert_eq!(ed.cursor_byte_pos(), 5);
    }

    // ── 2.6 delete_before / delete_after ─────────────────────────────────────

    #[test]
    fn delete_before_removes_prev_char() {
        let mut ed = InputEditor::default();
        ed.push_str("ab");
        ed.delete_before();
        assert_eq!(ed.text(), "a");
        assert_eq!(ed.cursor_byte_pos(), 1);
    }

    #[test]
    fn delete_before_at_start_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("x");
        ed.move_to_start(false);
        ed.delete_before();
        assert_eq!(ed.text(), "x");
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn delete_before_multibyte() {
        let mut ed = InputEditor::default();
        ed.push_str("aé"); // 'a'=1 byte, 'é'=2 bytes → len=3
        ed.delete_before();
        assert_eq!(ed.text(), "a");
        assert_eq!(ed.cursor_byte_pos(), 1);
        assert!(ed.buf.is_char_boundary(ed.cursor));
    }

    #[test]
    fn delete_after_removes_next_char() {
        let mut ed = InputEditor::default();
        ed.push_str("ab");
        ed.move_to_start(false);
        ed.delete_after();
        assert_eq!(ed.text(), "b");
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn delete_after_at_end_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("x");
        ed.delete_after();
        assert_eq!(ed.text(), "x");
        assert_eq!(ed.cursor_byte_pos(), 1);
    }

    #[test]
    fn delete_after_multibyte() {
        let mut ed = InputEditor::default();
        ed.push_str("éa");
        ed.move_to_start(false);
        ed.delete_after();
        assert_eq!(ed.text(), "a");
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    // ── 2.7 delete_before with active selection ───────────────────────────────

    #[test]
    fn delete_before_with_selection_deletes_range() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.move_to_start(false);
        ed.move_right(true);
        ed.move_right(true); // select "he" (anchor=0, cursor=2)
        ed.delete_before();
        assert_eq!(ed.text(), "llo");
        assert_eq!(ed.cursor_byte_pos(), 0);
        assert!(ed.anchor.is_none());
    }

    #[test]
    fn delete_before_with_reverse_selection() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        // anchor > cursor: select backwards
        ed.move_left(true);
        ed.move_left(true); // anchor=5, cursor=3 → selection "lo"
        ed.delete_before();
        assert_eq!(ed.text(), "hel");
        assert_eq!(ed.cursor_byte_pos(), 3);
    }

    // ── 2.8 delete_word_before / delete_to_start ─────────────────────────────

    #[test]
    fn delete_word_before_removes_preceding_word() {
        let mut ed = InputEditor::default();
        ed.push_str("hello world");
        ed.delete_word_before();
        assert_eq!(ed.text(), "hello ");
        assert_eq!(ed.cursor_byte_pos(), 6);
    }

    #[test]
    fn delete_word_before_with_only_whitespace() {
        // Only whitespace precedes cursor → all whitespace before cursor removed
        let mut ed = InputEditor::default();
        ed.push_str("   world");
        ed.move_to_start(false);
        for _ in 0..3 { ed.move_right(false); } // cursor at byte 3 (after spaces)
        ed.delete_word_before();
        assert_eq!(ed.text(), "world");
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn delete_word_before_at_start_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("hi");
        ed.move_to_start(false);
        ed.delete_word_before();
        assert_eq!(ed.text(), "hi");
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn delete_to_start_removes_everything_before_cursor() {
        let mut ed = InputEditor::default();
        ed.push_str("hello world");
        // move cursor to after "hello "
        ed.move_to_start(false);
        for _ in 0..6 { ed.move_right(false); }
        ed.delete_to_start();
        assert_eq!(ed.text(), "world");
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    #[test]
    fn delete_to_start_at_start_does_nothing() {
        let mut ed = InputEditor::default();
        ed.push_str("hi");
        ed.move_to_start(false);
        ed.delete_to_start();
        assert_eq!(ed.text(), "hi");
        assert_eq!(ed.cursor_byte_pos(), 0);
    }

    // ── 2.9 selection() ordering ──────────────────────────────────────────────

    #[test]
    fn selection_returns_ordered_when_cursor_before_anchor() {
        let mut ed = InputEditor::default();
        ed.push_str("abc");
        // cursor at end (3), shift-left twice → anchor=3, cursor=1
        ed.move_left(true);
        ed.move_left(true);
        assert_eq!(ed.selection(), Some((1, 3)));
    }

    #[test]
    fn selection_returns_ordered_when_cursor_after_anchor() {
        let mut ed = InputEditor::default();
        ed.push_str("abc");
        ed.move_to_start(false);
        // anchor=0, cursor=2
        ed.move_right(true);
        ed.move_right(true);
        assert_eq!(ed.selection(), Some((0, 2)));
    }

    #[test]
    fn selection_returns_none_when_no_anchor() {
        let mut ed = InputEditor::default();
        ed.push_str("abc");
        assert_eq!(ed.selection(), None);
    }

    // ── 2.10 selected_text() ──────────────────────────────────────────────────

    #[test]
    fn selected_text_returns_selected_bytes() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.move_to_start(false);
        ed.move_right(true);
        ed.move_right(true); // select "he"
        assert_eq!(ed.selected_text(), Some("he".to_string()));
    }

    #[test]
    fn selected_text_reverse_selection_returns_same_content() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        // select "lo" backwards: cursor goes left from end
        ed.move_left(true);
        ed.move_left(true); // anchor=5, cursor=3
        assert_eq!(ed.selected_text(), Some("lo".to_string()));
    }

    #[test]
    fn selected_text_no_selection_returns_none() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        assert_eq!(ed.selected_text(), None);
    }

    // ── 2.11 delete_selection() ───────────────────────────────────────────────

    #[test]
    fn delete_selection_removes_selected_range() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.move_to_start(false);
        ed.move_right(true);
        ed.move_right(true); // select "he"
        ed.delete_selection();
        assert_eq!(ed.text(), "llo");
        assert_eq!(ed.cursor_byte_pos(), 0);
        assert!(ed.anchor.is_none());
    }

    #[test]
    fn delete_selection_no_selection_is_noop() {
        let mut ed = InputEditor::default();
        ed.push_str("hello");
        ed.delete_selection();
        assert_eq!(ed.text(), "hello");
        assert_eq!(ed.cursor_byte_pos(), 5);
        assert!(ed.anchor.is_none());
    }
}
