//! Translate winit keyboard events into a semantic [`KeyAction`] enum that the
//! app shell can dispatch on. The whole point of this scaffold is to verify
//! that macOS-level cmd / option modifiers reach our event loop unintercepted;
//! every translated action is logged at `debug` so the prototype run can be
//! eyeballed against the v1 plan's modifier matrix.

#![cfg(feature = "dev-wgpu")]

use winit::event::KeyEvent;
use winit::keyboard::{Key, ModifiersState, NamedKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// `Cmd + 0..=9` — workspace tab switch.
    CmdDigit(u8),
    /// `Cmd + <letter>` — generic command. The letter is lowercased.
    CmdLetter(char),
    /// `Cmd + \`` — cycle workspaces.
    CmdBackquote,
    /// `Cmd + Shift + \`` — reverse-cycle workspaces (Cmd+~ on US layout).
    CmdShiftBackquote,
    /// `Option + \`` — alternate cycle.
    OptionBackquote,
    /// `Cmd + W` — close current window/tab.
    CmdW,
    /// `Cmd + N` — new workspace.
    CmdN,
    /// Escape pressed (no modifiers).
    Escape,
    /// Anything else; the inner string is a debug-friendly description.
    Other(String),
}

/// Translate a winit key event + the latest modifier state into a semantic
/// [`KeyAction`]. Only `Pressed` events produce non-`Other` results — releases
/// are reported as `Other` so the caller can decide whether to ignore them.
pub fn translate(event: &KeyEvent, modifiers: ModifiersState) -> KeyAction {
    if !event.state.is_pressed() {
        return KeyAction::Other(format!("release {:?}", event.logical_key));
    }

    let cmd = modifiers.super_key();
    let alt = modifiers.alt_key();
    let shift = modifiers.shift_key();

    match &event.logical_key {
        Key::Named(NamedKey::Escape) if !cmd && !alt => KeyAction::Escape,
        Key::Character(s) => {
            let mut chars = s.chars();
            let first = chars.next();
            let rest_empty = chars.next().is_none();
            match (first, rest_empty) {
                (Some(c), true) if cmd && c.is_ascii_digit() => {
                    KeyAction::CmdDigit((c as u8) - b'0')
                }
                (Some('`'), true) if cmd && shift => KeyAction::CmdShiftBackquote,
                (Some('~'), true) if cmd => KeyAction::CmdShiftBackquote,
                (Some('`'), true) if cmd => KeyAction::CmdBackquote,
                (Some('`'), true) if alt => KeyAction::OptionBackquote,
                (Some('w'), true) | (Some('W'), true) if cmd => KeyAction::CmdW,
                (Some('n'), true) | (Some('N'), true) if cmd => KeyAction::CmdN,
                (Some(c), true) if cmd && c.is_ascii_alphabetic() => {
                    KeyAction::CmdLetter(c.to_ascii_lowercase())
                }
                _ => KeyAction::Other(format!(
                    "char {s:?} cmd={cmd} alt={alt} shift={shift}"
                )),
            }
        }
        other => KeyAction::Other(format!(
            "key {other:?} cmd={cmd} alt={alt} shift={shift}"
        )),
    }
}

