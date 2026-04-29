## ADDED Requirements

### Requirement: Input bar clipboard shortcuts (dev-wgpu)

The input bar in the `dev-wgpu` backend SHALL handle `Cmd+C`, `Cmd+X`, and `Cmd+V` to trigger clipboard copy, cut, and paste respectively. These shortcuts SHALL be dispatched through the existing `CmdLetter` translation path in `keys.rs`.

#### Scenario: Cmd+C is dispatched as copy in wgpu backend

- **WHEN** the user presses `Cmd+C` while the wgpu window has focus
- **THEN** `handle_key` routes the event to the clipboard copy operation
- **AND** the cursor blink is reset and the frame is redrawn

#### Scenario: Cmd+X is dispatched as cut in wgpu backend

- **WHEN** the user presses `Cmd+X` while the wgpu window has focus
- **THEN** `handle_key` routes the event to the clipboard cut operation
- **AND** the cursor blink is reset and the frame is redrawn

#### Scenario: Cmd+V is dispatched as paste in wgpu backend

- **WHEN** the user presses `Cmd+V` while the wgpu window has focus
- **THEN** `handle_key` routes the event to the clipboard paste operation
- **AND** the cursor blink is reset and the frame is redrawn

### Requirement: Input bar clipboard shortcuts (dev-crossterm)

The input bar in the `dev-crossterm` backend SHALL handle `Ctrl+C`, `Ctrl+X`, and `Ctrl+V` as the terminal equivalents for clipboard copy, cut, and paste.

#### Scenario: Ctrl+C copies in crossterm backend

- **WHEN** the user presses `Ctrl+C` in the crossterm backend
- **THEN** the selected text (if any) is written to the system clipboard
- **AND** the application does not exit

#### Scenario: Ctrl+X cuts in crossterm backend

- **WHEN** the user presses `Ctrl+X` in the crossterm backend
- **THEN** the selected text (if any) is cut to the system clipboard

#### Scenario: Ctrl+V pastes in crossterm backend

- **WHEN** the user presses `Ctrl+V` in the crossterm backend
- **THEN** clipboard text is inserted at the cursor position
