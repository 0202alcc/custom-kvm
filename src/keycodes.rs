//! Keyboard keycode mapping from Linux evdev to macOS virtual keycodes.
//!
//! This module provides translation between Linux input event keycodes (evdev)
//! and macOS CGEvent virtual keycodes. Note that this mapping is not exhaustive
//! and some platform-specific keys may not have equivalents on the other platform.

/// Maps a Linux evdev keycode to a macOS virtual keycode.
///
/// Returns the corresponding macOS virtual keycode if available, or `None` if
/// the keycode is unknown or has no macOS equivalent.
///
/// # Arguments
/// * `linux_code` - The Linux evdev keycode
///
/// # Returns
/// * `Some(u32)` - The corresponding macOS virtual keycode
/// * `None` - If the keycode is unknown or unsupported
pub fn linux_to_macos_keycode(linux_code: u16) -> Option<u32> {
    match linux_code {
        // Letter keys (A-Z)
        30 => Some(0),   // KEY_A -> kVK_ANSI_A
        48 => Some(11),  // KEY_B -> kVK_ANSI_B
        46 => Some(8),   // KEY_C -> kVK_ANSI_C
        32 => Some(2),   // KEY_D -> kVK_ANSI_D
        18 => Some(14),  // KEY_E -> kVK_ANSI_E
        33 => Some(3),   // KEY_F -> kVK_ANSI_F
        34 => Some(5),   // KEY_G -> kVK_ANSI_G
        35 => Some(4),   // KEY_H -> kVK_ANSI_H
        23 => Some(34),  // KEY_I -> kVK_ANSI_I
        36 => Some(38),  // KEY_J -> kVK_ANSI_J
        37 => Some(40),  // KEY_K -> kVK_ANSI_K
        38 => Some(37),  // KEY_L -> kVK_ANSI_L
        50 => Some(46),  // KEY_M -> kVK_ANSI_M
        49 => Some(45),  // KEY_N -> kVK_ANSI_N
        24 => Some(31),  // KEY_O -> kVK_ANSI_O
        25 => Some(35),  // KEY_P -> kVK_ANSI_P
        16 => Some(12),  // KEY_Q -> kVK_ANSI_Q
        19 => Some(15),  // KEY_R -> kVK_ANSI_R
        31 => Some(1),   // KEY_S -> kVK_ANSI_S
        20 => Some(17),  // KEY_T -> kVK_ANSI_T
        22 => Some(32),  // KEY_U -> kVK_ANSI_U
        47 => Some(9),   // KEY_V -> kVK_ANSI_V
        17 => Some(13),  // KEY_W -> kVK_ANSI_W
        45 => Some(7),   // KEY_X -> kVK_ANSI_X
        21 => Some(16),  // KEY_Y -> kVK_ANSI_Y
        44 => Some(6),   // KEY_Z -> kVK_ANSI_Z

        // Number keys (0-9)
        11 => Some(29),  // KEY_0 -> kVK_ANSI_0
        2 => Some(18),   // KEY_1 -> kVK_ANSI_1
        3 => Some(19),   // KEY_2 -> kVK_ANSI_2
        4 => Some(20),   // KEY_3 -> kVK_ANSI_3
        5 => Some(21),   // KEY_4 -> kVK_ANSI_4
        6 => Some(23),   // KEY_5 -> kVK_ANSI_5
        7 => Some(22),   // KEY_6 -> kVK_ANSI_6
        8 => Some(26),   // KEY_7 -> kVK_ANSI_7
        9 => Some(28),   // KEY_8 -> kVK_ANSI_8
        10 => Some(25),  // KEY_9 -> kVK_ANSI_9

        // Modifier keys
        42 => Some(56),   // KEY_LEFTSHIFT -> kVK_Shift
        54 => Some(60),   // KEY_RIGHTSHIFT -> kVK_RightShift
        29 => Some(59),   // KEY_LEFTCTRL -> kVK_Control
        97 => Some(62),   // KEY_RIGHTCTRL -> kVK_RightControl
        56 => Some(58),   // KEY_LEFTALT -> kVK_Option
        100 => Some(61),  // KEY_RIGHTALT -> kVK_RightOption

        // Special keys
        57 => Some(49),   // KEY_SPACE -> kVK_Space
        28 => Some(36),   // KEY_ENTER -> kVK_Return
        15 => Some(48),   // KEY_TAB -> kVK_Tab
        14 => Some(51),   // KEY_BACKSPACE -> kVK_Delete
        1 => Some(53),    // KEY_ESCAPE -> kVK_Escape

        // Arrow keys
        103 => Some(126), // KEY_UP -> kVK_UpArrow
        108 => Some(125), // KEY_DOWN -> kVK_DownArrow
        105 => Some(123), // KEY_LEFT -> kVK_LeftArrow
        106 => Some(124), // KEY_RIGHT -> kVK_RightArrow

        // Navigation keys
        102 => Some(115), // KEY_HOME -> kVK_Home
        107 => Some(119), // KEY_END -> kVK_End
        104 => Some(116), // KEY_PAGEUP -> kVK_PageUp
        109 => Some(121), // KEY_PAGEDOWN -> kVK_PageDown

        // Function keys (F1-F12)
        59 => Some(122),  // KEY_F1 -> kVK_F1
        60 => Some(120),  // KEY_F2 -> kVK_F2
        61 => Some(99),   // KEY_F3 -> kVK_F3
        62 => Some(118),  // KEY_F4 -> kVK_F4
        63 => Some(96),   // KEY_F5 -> kVK_F5
        64 => Some(97),   // KEY_F6 -> kVK_F6
        65 => Some(98),   // KEY_F7 -> kVK_F7
        66 => Some(100),  // KEY_F8 -> kVK_F8
        67 => Some(101),  // KEY_F9 -> kVK_F9
        68 => Some(109),  // KEY_F10 -> kVK_F10
        87 => Some(103),  // KEY_F11 -> kVK_F11
        88 => Some(111),  // KEY_F12 -> kVK_F12

        // Special characters and symbols
        12 => Some(27),   // KEY_MINUS -> kVK_ANSI_Minus
        13 => Some(24),   // KEY_EQUAL -> kVK_ANSI_Equal
        26 => Some(33),   // KEY_LEFTBRACE -> kVK_ANSI_LeftBracket
        27 => Some(30),   // KEY_RIGHTBRACE -> kVK_ANSI_RightBracket
        39 => Some(41),   // KEY_SEMICOLON -> kVK_ANSI_Semicolon
        40 => Some(39),   // KEY_APOSTROPHE -> kVK_ANSI_Quote
        41 => Some(50),   // KEY_GRAVE -> kVK_ANSI_Grave
        43 => Some(42),   // KEY_BACKSLASH -> kVK_ANSI_Backslash
        51 => Some(43),   // KEY_COMMA -> kVK_ANSI_Comma
        52 => Some(47),   // KEY_DOT -> kVK_ANSI_Period
        53 => Some(44),   // KEY_SLASH -> kVK_ANSI_Slash

        // Unknown keycode
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letter_keys() {
        assert_eq!(linux_to_macos_keycode(30), Some(0));   // A
        assert_eq!(linux_to_macos_keycode(48), Some(11));  // B
        assert_eq!(linux_to_macos_keycode(44), Some(6));   // Z
    }

    #[test]
    fn test_number_keys() {
        assert_eq!(linux_to_macos_keycode(2), Some(18));   // 1
        assert_eq!(linux_to_macos_keycode(11), Some(29));  // 0
    }

    #[test]
    fn test_special_keys() {
        assert_eq!(linux_to_macos_keycode(57), Some(49));  // SPACE
        assert_eq!(linux_to_macos_keycode(28), Some(36));  // ENTER
        assert_eq!(linux_to_macos_keycode(1), Some(53));   // ESCAPE
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(linux_to_macos_keycode(103), Some(126)); // UP
        assert_eq!(linux_to_macos_keycode(108), Some(125)); // DOWN
    }

    #[test]
    fn test_function_keys() {
        assert_eq!(linux_to_macos_keycode(59), Some(122));  // F1
        assert_eq!(linux_to_macos_keycode(88), Some(111));  // F12
    }

    #[test]
    fn test_unknown_keycode() {
        assert_eq!(linux_to_macos_keycode(9999), None);
    }
}
