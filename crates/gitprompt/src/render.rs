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

/// Makes a branch name safe to splice into `PROMPT`.
///
/// The dangerous character here is `%`, not ESC. This output is not printed to
/// a terminal — it is expanded by zsh as part of the prompt, where `%` opens a
/// prompt escape. `git check-ref-format` accepts `%` in ref names, and a cloned
/// repository adopts the remote's branch names, so a hostile remote named its
/// default branch `hotfix%<` would otherwise break the prompt on first
/// checkout.
///
/// Consequences of not escaping, all reproduced against real zsh:
///
/// - `%<` starts a left-truncation construct that swallows the rest of the
///   prompt: no closing paren, no reset, no `%#` indicator.
/// - `%d` and `%m` expand to the working directory and hostname.
/// - `%}` closes a `%{` zero-width marker early, corrupting width accounting.
///
/// `%%` is zsh's literal `%`, so doubling is the correct escape. ESC and other
/// control characters are dropped outright rather than escaped, since there is
/// no legitimate reason for them in a ref name.
///
/// Width is deliberately *not* adjusted for multi-cell characters (CJK, emoji).
/// zsh's `%1{...%}` marker handles a known literal like the dirty glyph; a
/// computed width for arbitrary branch text would need terminal metrics this
/// process cannot query, and guessing wrong is worse than leaving it to zsh.
#[must_use]
pub fn sanitize(branch: &str) -> String {
    let mut out = String::with_capacity(branch.len());
    for c in branch.chars() {
        if c == '%' {
            out.push_str("%%");
        } else if !c.is_control() {
            out.push(c);
        }
    }
    out
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

    // --- `%` escaping -------------------------------------------------------
    //
    // `git check-ref-format` accepts `%` in ref names, and a clone adopts the
    // remote's branch names, so these are reachable without anyone typing them.

    #[test]
    fn percent_in_a_branch_name_is_doubled() {
        // `%%` is zsh's literal percent.
        assert_eq!(sanitize("pct50%"), "pct50%%");
        assert_eq!(sanitize("a%b"), "a%%b");
    }

    /// Strips this crate's own prompt tokens, leaving only the branch text.
    ///
    /// The `%%` sequences are the escaped form and are removed outright. What
    /// survives is any `%` the branch contributed unescaped — which is exactly
    /// what an active prompt escape would look like.
    fn branch_text_only(rendered: &str) -> String {
        rendered
            .replace("%1{", "")
            .replace("%%", "")
            .replace("%{", "")
            .replace("%}", "")
    }

    #[test]
    fn a_percent_truncation_construct_cannot_escape_the_segment() {
        // `%<` opens a left-truncation that runs to the end of the prompt. Left
        // unescaped it ate the closing paren, the reset, and everything after.
        let rendered = segment(&sanitize("hotfix%<"), false);

        assert!(
            !branch_text_only(&rendered).contains('%'),
            "unescaped percent survived into {rendered:?}"
        );
        assert!(rendered.contains("hotfix%%<"));
    }

    #[test]
    fn information_leaking_prompts_are_neutralised() {
        // `%d` -> cwd, `%m`/`%n` -> hostname/username. All become literals.
        for hostile in ["fix%d", "fix%m", "fix%n", "fix%<", "fix%(", "fix%s"] {
            let rendered = segment(&sanitize(hostile), false);
            assert!(
                !branch_text_only(&rendered).contains('%'),
                "{hostile} left an active prompt escape: {rendered:?}"
            );
        }
    }

    #[test]
    fn a_percent_brace_cannot_close_the_zero_width_marker_early() {
        // `%}` would end a `%{` early and corrupt zsh's width accounting. Both
        // metacharacters must be doubled: `%` so it is literal, and the brace
        // that follows is then just ordinary text.
        let sanitized = sanitize("a%}b%{c");
        assert_eq!(sanitized, "a%%}b%%{c");

        let rendered = segment(&sanitized, false);
        // The branch text contributes no bare `%`, only the escaped `%%` form.
        assert!(!branch_text_only(&rendered).contains('%'));
        assert!(rendered.contains("a%%}b%%{c"));
    }

    #[test]
    fn ordinary_branch_names_are_untouched_by_escaping() {
        for name in ["main", "feature/login", "v1.2.3", "release-2024"] {
            assert_eq!(sanitize(name), name, "{name} should pass through unchanged");
        }
    }
}
