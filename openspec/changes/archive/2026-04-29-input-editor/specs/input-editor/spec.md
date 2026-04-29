## ADDED Requirements

### Requirement: InputEditor owns text buffer and cursor
The `InputEditor` struct SHALL own the text buffer (`String`), cursor position (byte index into the buffer), and an optional selection anchor (byte index). The cursor and anchor SHALL always sit on UTF-8 char boundaries. `InputEditor` SHALL expose `text() -> &str` and `cursor_byte_pos() -> usize` for read access.

#### Scenario: Default state
- **WHEN** `InputEditor::default()` is constructed
- **THEN** text is empty, cursor is at byte 0, and no selection is active

#### Scenario: Push string
- **WHEN** `push_str(s)` is called
- **THEN** `s` is appended at the cursor position and the cursor advances to the end of the inserted text

#### Scenario: Cursor stays on char boundary after multibyte insert
- **WHEN** a multibyte UTF-8 character is inserted
- **THEN** the cursor byte position points to the first byte after that character

### Requirement: Character movement
`InputEditor` SHALL support moving the cursor one Unicode scalar value at a time.

#### Scenario: Move right within text
- **WHEN** `move_right(false)` is called and the cursor is not at the end
- **THEN** the cursor advances to the start of the next char

#### Scenario: Move right at end
- **WHEN** `move_right(false)` is called and the cursor is at the end of the buffer
- **THEN** the cursor does not move

#### Scenario: Move left within text
- **WHEN** `move_left(false)` is called and the cursor is not at the start
- **THEN** the cursor retreats to the start of the previous char

#### Scenario: Move left at start
- **WHEN** `move_left(false)` is called and the cursor is at byte 0
- **THEN** the cursor does not move

#### Scenario: Move right collapses selection
- **WHEN** a selection is active and `move_right(false)` is called
- **THEN** the cursor moves to the end of the selection and the selection is cleared

#### Scenario: Move left collapses selection
- **WHEN** a selection is active and `move_left(false)` is called
- **THEN** the cursor moves to the start of the selection and the selection is cleared

### Requirement: Word movement
`InputEditor` SHALL support jumping to the start of the previous word and the end of the next word. A "word" is a maximal run of non-whitespace characters.

#### Scenario: Move word right from mid-word
- **WHEN** `move_word_right(false)` is called while the cursor is inside a word
- **THEN** the cursor moves to the position immediately after the last character of that word (the next whitespace or end of buffer)

#### Scenario: Move word right from whitespace
- **WHEN** `move_word_right(false)` is called while the cursor is in leading whitespace before a word
- **THEN** the cursor moves to the position immediately after the last character of the next word

#### Scenario: Move word left from mid-word
- **WHEN** `move_word_left(false)` is called while the cursor is inside or just after a word
- **THEN** the cursor moves to the start of that word

#### Scenario: Move word left from whitespace
- **WHEN** `move_word_left(false)` is called while the cursor is in whitespace after a word
- **THEN** the cursor moves to the start of that preceding word

### Requirement: Line-boundary movement
`InputEditor` SHALL support jumping to the start and end of the line.

#### Scenario: Move to start
- **WHEN** `move_to_start(false)` is called
- **THEN** the cursor moves to byte 0

#### Scenario: Move to end
- **WHEN** `move_to_end(false)` is called
- **THEN** the cursor moves to `text().len()` (one past the last byte)

### Requirement: Shift-extend selection
Every movement operation SHALL accept a `bool` argument (`extend_selection`). When `true`, the anchor is set to the cursor's pre-move position (if no anchor exists) and the cursor moves, extending the selection. When `false`, any active selection is cleared.

#### Scenario: Shift-right starts selection
- **WHEN** no selection is active and `move_right(true)` is called
- **THEN** the anchor is set to the original cursor position and the cursor advances one char

#### Scenario: Shift-right extends existing selection
- **WHEN** a selection is active and `move_right(true)` is called
- **THEN** the anchor is unchanged and the cursor advances one char

#### Scenario: Non-shift clears selection
- **WHEN** a selection is active and any movement is called with `extend_selection = false`
- **THEN** the anchor is cleared (selection collapses per the direction rule)

### Requirement: Character deletion
`InputEditor` SHALL support deleting the character immediately before the cursor (backspace) and immediately after the cursor (forward delete).

#### Scenario: Backspace with text before cursor
- **WHEN** `delete_before()` is called and the cursor is not at the start
- **THEN** the character immediately before the cursor is removed and the cursor retreats by that char's byte length

#### Scenario: Backspace at start
- **WHEN** `delete_before()` is called and the cursor is at byte 0
- **THEN** the buffer is unchanged

#### Scenario: Forward delete with text after cursor
- **WHEN** `delete_after()` is called and the cursor is not at the end
- **THEN** the character immediately after the cursor is removed; the cursor does not move

#### Scenario: Forward delete at end
- **WHEN** `delete_after()` is called and the cursor is at the end
- **THEN** the buffer is unchanged

#### Scenario: Backspace with active selection
- **WHEN** a selection is active and `delete_before()` is called
- **THEN** the selected text is deleted, the cursor moves to the start of the selection, and the selection is cleared

### Requirement: Word deletion
`InputEditor` SHALL support deleting from the cursor to the start of the previous word (Option+Backspace).

#### Scenario: Delete word before cursor
- **WHEN** `delete_word_before()` is called with at least one non-whitespace char before the cursor
- **THEN** all characters from the start of the preceding word up to the cursor are removed and the cursor moves to the new position

#### Scenario: Delete word before cursor with only whitespace before
- **WHEN** `delete_word_before()` is called and only whitespace precedes the cursor
- **THEN** all whitespace before the cursor is removed

### Requirement: Delete to line start
`InputEditor` SHALL support deleting from the cursor back to byte 0 (Cmd+Backspace).

#### Scenario: Delete to start with text before cursor
- **WHEN** `delete_to_start()` is called and the cursor is not at byte 0
- **THEN** all text from byte 0 up to (but not including) the cursor is removed and the cursor moves to byte 0

#### Scenario: Delete to start at byte 0
- **WHEN** `delete_to_start()` is called at byte 0
- **THEN** the buffer is unchanged

### Requirement: Selection range query
`InputEditor` SHALL expose the active selection as `selection() -> Option<(usize, usize)>` returning `(start_byte, end_byte)` with `start <= end`, or `None` if no selection is active.

#### Scenario: Selection returned as ordered range
- **WHEN** the cursor is before the anchor
- **THEN** `selection()` returns `Some((cursor_pos, anchor_pos))`

#### Scenario: Selection when cursor is after anchor
- **WHEN** the cursor is after the anchor
- **THEN** `selection()` returns `Some((anchor_pos, cursor_pos))`

#### Scenario: No selection
- **WHEN** no anchor is set
- **THEN** `selection()` returns `None`

### Requirement: render_input draws cursor and selection
`render_input` SHALL accept `&InputEditor` and SHALL render the input text with a block cursor (REVERSED span) at the cursor position and, when a selection is active, a highlighted span over the selected text.

#### Scenario: Cursor at end of text
- **WHEN** cursor is at the end and cursor_visible is true
- **THEN** a REVERSED space is rendered immediately after the last character

#### Scenario: Cursor mid-text
- **WHEN** cursor is in the middle of the text and cursor_visible is true
- **THEN** the character at the cursor position is rendered with REVERSED style and the remaining text follows

#### Scenario: Selection highlight
- **WHEN** a selection is active
- **THEN** the selected text is rendered with a distinct background (REVERSED or highlighted style) to distinguish it from unselected text

#### Scenario: Cursor hidden (blink off)
- **WHEN** cursor_visible is false
- **THEN** no cursor span is rendered (text appears without cursor block)
