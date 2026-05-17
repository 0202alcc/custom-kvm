# macOS Keyboard Injection Notes

This document outlines the approach to inject keyboard events on macOS using CoreGraphics CGEvent APIs.

- Use CoreGraphics `CGEventCreateKeyboardEvent` equivalents via the `core-graphics` crate.
- Maintain a mapping from Linux input-event keycodes to macOS virtual keycodes (`src/keycodes.rs`).
- Handle modifier state (Shift/Ctrl/Alt) carefully — send modifier press/release events when appropriate.
- Test with common layouts (US) and document limitations for other keyboard layouts.
