## Requirements

### Requirement: Native macOS window via winit

The client SHALL open a native macOS window using `winit` and run a windowing event loop until the window is closed. The window SHALL NOT be a terminal emulator and SHALL NOT depend on a host terminal for input or output.

#### Scenario: Window opens on launch

- **WHEN** the user runs `cargo run -p pi-oven` (or launches the bundled `.app`)
- **THEN** a native macOS window appears with title `pi-oven`
- **AND** the window is resizable and has standard macOS chrome (close, minimise, fullscreen)

#### Scenario: Window closes cleanly

- **WHEN** the user clicks the window's close button or presses Cmd+W (when supported)
- **THEN** the event loop exits and the process terminates with status zero

#### Scenario: Process is not a terminal app

- **WHEN** the client is running
- **THEN** no terminal window is brought to the foreground
- **AND** the host terminal (if launched from one) is not used for input or output

### Requirement: First-class capture of cmd and option modifiers

The client SHALL observe `Cmd` and `Option` (Alt) modifiers on every keyboard event, distinguishable from `Shift` and `Control`, without intervening interception by macOS or any host terminal.

#### Scenario: Cmd+letter is observable

- **WHEN** the user presses `Cmd+N` while the client window has focus
- **THEN** the client's key handler receives an event with `super_key()` (logical Command) set and the corresponding logical key
- **AND** logs at `debug` level include the modifier state and key

#### Scenario: Cmd+digit is observable

- **WHEN** the user presses `Cmd+1` through `Cmd+9` while the client window has focus
- **THEN** the client's key handler receives an event with `super_key()` set and the corresponding digit key

#### Scenario: Cmd+backquote is observable

- **WHEN** the user presses `Cmd+\`` while the client window has focus
- **THEN** the client's key handler receives an event distinguishable from `Cmd+~` (Cmd+Shift+\`)
- **AND** the same key combination produces an identical event whether or not a terminal is in the foreground

#### Scenario: Option modifier distinguishable from Cmd

- **WHEN** the user presses `Option+\``
- **THEN** the client's key handler receives an event with `alt_key()` (logical Option) set and `super_key()` not set

### Requirement: Cell grid render pipeline using wgpu and glyphon

The client SHALL maintain an in-memory cell grid (rows × columns of `{char, fg, bg, attrs}`) and render the grid to the window each frame using `wgpu` for the GPU pass and `glyphon` for monospace text shaping.

#### Scenario: Initial render shows placeholder text

- **WHEN** the client window is first painted
- **THEN** the cell grid contains the string `pi-oven` at row 0
- **AND** the rendered window displays that text using a monospace font

#### Scenario: Window resize updates grid dimensions

- **WHEN** the window is resized
- **THEN** the cell grid is reallocated to match the new size in cells (computed from window size and current font metrics)
- **AND** the next frame renders without errors

#### Scenario: HiDPI rendering on retina displays

- **WHEN** the client runs on a retina display
- **THEN** text is rendered at the display's native pixel density without blurring
- **AND** the wgpu surface uses the correct scale factor reported by winit

### Requirement: Custom ratatui Backend writing into the cell grid

The client SHALL implement `ratatui::backend::Backend` such that ratatui widgets and layouts produce output by writing into the client's cell grid, not by emitting terminal escape sequences.

#### Scenario: Ratatui widget renders into grid

- **WHEN** the client uses any standard ratatui widget (e.g. `Paragraph`) to draw a layout
- **THEN** the resulting characters and styles appear in the cell grid at the layout's coordinates
- **AND** the rendered window shows the widget visually

#### Scenario: Style attributes are preserved

- **WHEN** ratatui emits a styled cell with foreground colour, background colour, and bold/italic/underline attributes
- **THEN** the cell grid stores all of these attributes
- **AND** the renderer applies them when painting the window

### Requirement: Bundled as a macOS .app

The client SHALL be packageable as a macOS `.app` bundle containing the binary, an `Info.plist` declaring the bundle identifier, version, and a Dock icon. Building the bundle SHALL be a single command.

#### Scenario: Cargo bundle produces .app

- **WHEN** the developer runs `cargo bundle --release`
- **THEN** the output directory contains `pi-oven.app/`
- **AND** the bundle has an `Info.plist` with `CFBundleIdentifier`, `CFBundleName = "pi-oven"`, and `CFBundleShortVersionString` matching the crate version

#### Scenario: Launching the .app opens the same window

- **WHEN** the user double-clicks the produced `pi-oven.app`
- **THEN** the same native window appears as `cargo run -p pi-oven` produces
- **AND** the app appears in the Dock with the bundle's icon

### Requirement: Development workflow with cargo run

Developers SHALL be able to launch the unbundled client with `cargo run -p pi-oven` for fast iteration, without needing to rebuild a `.app` between code changes.

#### Scenario: Cargo run launches a window

- **WHEN** a developer runs `cargo run -p pi-oven` from the repo root
- **THEN** the same window opens as the bundled app would produce
- **AND** standard Rust logging appears on stdout/stderr (controllable via `RUST_LOG`)
