## ADDED Requirements

### Requirement: Copy selection to clipboard

When text is selected in the input bar, `Cmd+C` SHALL copy the selected text to the system clipboard. If no text is selected the operation SHALL be a no-op.

#### Scenario: Cmd+C with active selection copies text

- **WHEN** the user has selected one or more characters in the input bar
- **AND** presses `Cmd+C`
- **THEN** the selected text is written to the system clipboard
- **AND** the selection and cursor position are unchanged

#### Scenario: Cmd+C with no selection is a no-op

- **WHEN** no text is selected in the input bar
- **AND** the user presses `Cmd+C`
- **THEN** the clipboard is not modified
- **AND** the buffer and cursor are unchanged

### Requirement: Cut selection to clipboard

`Cmd+X` SHALL copy the selected text to the system clipboard and then delete it from the buffer. If no text is selected the operation SHALL be a no-op.

#### Scenario: Cmd+X with active selection cuts text

- **WHEN** the user has selected one or more characters in the input bar
- **AND** presses `Cmd+X`
- **THEN** the selected text is written to the system clipboard
- **AND** the selected text is removed from the buffer
- **AND** the cursor is placed at the start of the deleted range with no active selection

#### Scenario: Cmd+X with no selection is a no-op

- **WHEN** no text is selected in the input bar
- **AND** the user presses `Cmd+X`
- **THEN** the clipboard is not modified
- **AND** the buffer and cursor are unchanged

### Requirement: Paste from clipboard

`Cmd+V` SHALL insert the current system clipboard text at the cursor. If text is selected it SHALL be replaced by the pasted text. If the clipboard is empty or contains non-text content the operation SHALL be a no-op.

#### Scenario: Cmd+V pastes at cursor with no selection

- **WHEN** no text is selected in the input bar
- **AND** the user presses `Cmd+V`
- **AND** the clipboard contains a non-empty string
- **THEN** the clipboard text is inserted at the current cursor position
- **AND** the cursor is placed after the last inserted character
- **AND** no selection is active

#### Scenario: Cmd+V replaces selection with clipboard text

- **WHEN** text is selected in the input bar
- **AND** the user presses `Cmd+V`
- **AND** the clipboard contains a non-empty string
- **THEN** the selected text is deleted and replaced by the clipboard text
- **AND** the cursor is placed after the last inserted character
- **AND** no selection is active

#### Scenario: Cmd+V with empty clipboard is a no-op

- **WHEN** the user presses `Cmd+V`
- **AND** the clipboard is empty or contains no accessible text
- **THEN** the buffer and cursor are unchanged

### Requirement: Clipboard errors are non-fatal

If the system clipboard is unavailable (e.g., headless environment, display server not running), clipboard operations SHALL fail silently — the application MUST NOT panic and MUST NOT display an error to the user.

#### Scenario: Clipboard initialisation failure is swallowed

- **WHEN** the OS clipboard API fails to initialise on a copy, cut, or paste operation
- **THEN** the operation is skipped
- **AND** a warning is emitted to the tracing log
- **AND** the application continues running normally
