//! Rendering the prompt segment.
//!
//! The escape sequences here are byte-for-byte what `config/zsh/.zshrc`
//! produced via `$fg_bold[blue]`, `$fg[red]`, `$fg[blue]`, `$fg[yellow]` and
//! `$reset_color`. They are written out rather than computed because zsh's
//! `%{...%}` markers must wrap each escape individually or the prompt's
//! cursor-width accounting breaks and the line redraws on every keystroke.

/// zsh prompt-expansion markers. `%{` and `%}` tell zsh the enclosed bytes are
/// zero-width; `%1{✘}` tells it the glyph is one cell wide even though it is a
/// nerd-font symbol.
const ZW: &str = "%{";
const ZW_END: &str = "%}";
const WIDTH_1: &str = "%1{";

/// Bold blue — the `git:(` opener and the closing paren.
const BLUE_BOLD: &str = "\x1b[01;34m";
/// Red — the branch name itself.
const RED: &str = "\x1b[31m";
/// Plain blue — the closing paren and separator.
const BLUE: &str = "\x1b[34m";
/// Yellow — the dirty marker.
const YELLOW: &str = "\x1b[33m";
/// Reset.
const RESET: &str = "\x1b[00m";

/// The prompt segment for a repository state.
///
/// Returns an empty string when there is nothing to show, which is what a
/// prompt substitution wants for "not in a repository".
#[must_use]
pub fn segment(branch: &str, dirty: bool) -> String {
    let mut out = String::with_capacity(64);

    out.push_str(ZW);
    out.push_str(BLUE_BOLD);
    out.push_str(ZW_END);
    out.push_str("git:(");

    out.push_str(ZW);
    out.push_str(RED);
    out.push_str(ZW_END);
    out.push_str(branch);

    if dirty {
        // Clean output ends `git:(branch) `; dirty inserts ` ✘` before the
        // reset, matching the old $ZSH_THEME_GIT_PROMPT_* expansion.
        out.push_str(ZW);
        out.push_str(BLUE);
        out.push_str(ZW_END);
        out.push_str(") ");
        out.push_str(ZW);
        out.push_str(YELLOW);
        out.push_str(ZW_END);
        out.push_str(WIDTH_1);
        out.push('\u{2718}'); // ✘
        out.push_str(ZW_END);
    } else {
        out.push_str(ZW);
        out.push_str(BLUE);
        out.push_str(ZW_END);
        out.push(')');
    }

    out.push_str(ZW);
    out.push_str(RESET);
    out.push_str(ZW_END);
    out.push(' ');

    out
}

/// Escapes a branch name for safe inclusion in a prompt.
///
/// Branch names cannot contain control characters, but `HEAD` is a file on
/// disk and a corrupt or hostile repository should not be able to inject escape
/// sequences into the user's terminal.
#[must_use]
pub fn sanitize(branch: &str) -> String {
    branch
        .chars()
        .filter(|c| !c.is_control() && *c != '\u{1b}')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_segment_matches_the_previous_shell_expansion() {
        let rendered = segment("main", false);

        // Byte-for-byte what `$ZSH_THEME_GIT_PROMPT_PREFIX` + branch +
        // `$ZSH_THEME_GIT_PROMPT_CLEAN` + `$ZSH_THEME_GIT_PROMPT_SUFFIX` gave.
        assert_eq!(
            rendered,
            "%{\x1b[01;34m%}git:(%{\x1b[31m%}main%{\x1b[34m%})%{\x1b[00m%} "
        );
    }

    #[test]
    fn dirty_segment_matches_the_previous_shell_expansion() {
        let rendered = segment("main", true);

        // Byte-for-byte what the shell produced. Note the marker is `%1{✘%}`:
        // the closing token is `%}`, not a bare `}`. A bare brace renders as a
        // literal `}` on screen, which is why this is pinned exactly.
        assert_eq!(
            rendered,
            "%{\x1b[01;34m%}git:(%{\x1b[31m%}main%{\x1b[34m%}) %{\x1b[33m%}%1{\u{2718}%}%{\x1b[00m%} "
        );
    }

    #[test]
    fn every_escape_sequence_is_wrapped_in_zero_width_markers() {
        for dirty in [true, false] {
            let rendered = segment("main", dirty);

            // Every escape byte must sit inside a `%{ ... %}` pair, otherwise
            // zsh counts the sequence as printable width and the prompt
            // redraws on each keystroke.
            let mut open = false;
            let bytes = rendered.as_bytes();
            for index in 0..bytes.len() {
                if bytes[index] == b'\x1b' {
                    assert!(
                        open,
                        "escape at byte {index} is not zero-width: {rendered:?}"
                    );
                }

                // Track both token shapes without confusing `%1{` — which opens
                // with a digit — for a `%{`. Matched as bytes so the multi-byte
                // glyph cannot panic the scan.
                match bytes[index..] {
                    [b'%', b'{', ..] => open = true,
                    [b'%', b'}', ..] => open = false,
                    _ => {}
                }
            }

            assert!(!open, "unterminated zero-width marker: {rendered:?}");
        }
    }

    #[test]
    fn dirty_marker_is_declared_one_cell_wide() {
        // A nerd-font glyph measured as zero width makes the prompt redraw
        // itself on every keystroke, so the width marker matters.
        assert!(segment("main", true).contains("%1{\u{2718}%}"));
    }

    #[test]
    fn branch_names_are_shown_verbatim() {
        assert!(segment("feature/some/deep/name", false).contains("feature/some/deep/name"));
    }

    #[test]
    fn control_characters_are_stripped_from_branch_names() {
        let dirty = format!("ma{}in", '\u{1b}');
        let rendered = segment(&sanitize(&dirty), false);

        assert_eq!(
            rendered.matches('\u{1b}').count(),
            4,
            "only the four styling escapes should remain"
        );
        assert!(rendered.contains("main"));
    }
}
