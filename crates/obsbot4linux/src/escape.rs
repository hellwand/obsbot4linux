// SPDX-License-Identifier: GPL-3.0-or-later

//! Text that did not originate in this program (the daemon's replies, names stored on the
//! camera, error texts) is made safe for a terminal before human output prints it. Control
//! characters (C0, DEL, C1, so also ESC), line and paragraph separators and bidirectional
//! formatting characters are replaced by a visible `\xNN` or `\u{NNNN}` stand-in; ordinary
//! text stays as it is. `--json` output never goes through here: the serializer escapes it.

use std::borrow::Cow;
use std::fmt::Write;

fn unsafe_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{2028}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
        )
}

/// `s` with every character a terminal could act on shown as an escape. Idempotent: the
/// result holds no such character, and a backslash is left alone.
pub fn escape(s: &str) -> Cow<'_, str> {
    if !s.chars().any(unsafe_char) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if !unsafe_char(c) {
            out.push(c);
        } else if (c as u32) < 0x100 {
            let _ = write!(out, "\\x{:02x}", c as u32);
        } else {
            let _ = write!(out, "\\u{{{:04x}}}", c as u32);
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_text_is_untouched() {
        assert!(matches!(escape("Desk – 12.5° µs"), Cow::Borrowed(_)));
    }

    #[test]
    fn terminal_active_characters_become_visible() {
        assert_eq!(
            escape("a\nb\rc\x1b[31m\x07\x7f\u{9b}\u{2028}\u{2029}\u{202e}\u{2066}"),
            "a\\x0ab\\x0dc\\x1b[31m\\x07\\x7f\\x9b\\u{2028}\\u{2029}\\u{202e}\\u{2066}"
        );
    }

    #[test]
    fn escaping_twice_changes_nothing() {
        let once = escape("x\x1by").into_owned();
        assert_eq!(escape(&once), once);
    }
}
