/// A shell-token classifier for the Tools tab's `Bash` and `Monitor`
/// labels. One class per `char` keeps the classifier independent of the
/// wrapper: a token that wraps across lines (a long quoted string) keeps
/// its class on every line.
///
/// The classifier is a lexer with one bit of state ("the next word is a
/// command word"), not a shell parser: it never fails and never changes
/// the text, and the worst case on odd syntax is a wrong color.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A plain argument word — the muted text color.
    Arg,
    /// The first word of a command segment.
    Command,
    /// A word in argument position that starts with `-`.
    Flag,
    /// Text inside single or double quotes, quotes included.
    Str,
    /// A `$` variable: `$NAME`, `$1`, `$?`, `${...}`, and a leading
    /// `NAME=value` assignment.
    Variable,
    /// Separators (`|`, `||`, `&&`, `;`, `&`), substitution marks
    /// (`(`, `)`, `$(`) and redirections (`>`, `>>`, `<`, `<<`, `<<<`,
    /// `&>`, with a leading descriptor or a trailing `&n`).
    Operator,
    /// A `#` comment, from the `#` to the end of the label.
    Comment,
}

/// Classify every `char` of a shell command label. The returned vector has
/// one entry per `char` of `label`, in order.
pub fn classify(label: &str) -> Vec<Class> {
    use Class::*;

    let chars: Vec<char> = label.chars().collect();
    let mut out = Vec::with_capacity(chars.len());

    // True while the next plain word is a command word: at the start of
    // the label and after a separator or `$(`. A leading `NAME=value`
    // assignment leaves it set.
    let mut expect_command = true;
    // Class of the word in progress; `None` between words.
    let mut word_class: Option<Class> = None;
    // Set right after a redirection: the target word that follows is Arg.
    let mut redirect_pending = false;
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            out.push(Arg);
            word_class = None;
            i += 1;
            continue;
        }

        // The start of any word consumes a pending redirection target:
        // the word that follows a redirection is Arg.
        let mut pending_redirect = false;
        if word_class.is_none() {
            pending_redirect = redirect_pending;
            redirect_pending = false;
        }

        match c {
            '\'' => {
                // Single quotes: every char to the closing quote is Str.
                out.push(Str);
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    out.push(Str);
                    i += 1;
                }
                if i < chars.len() {
                    out.push(Str); // closing quote
                    i += 1;
                }
            }
            '"' => {
                // Double quotes: Str throughout; `$` stays Str, a `\`
                // makes the next char plain Str.
                out.push(Str);
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        out.push(Str);
                        out.push(Str);
                        i += 2;
                    } else {
                        out.push(Str);
                        i += 1;
                    }
                }
                if i < chars.len() {
                    out.push(Str); // closing quote
                    i += 1;
                }
            }
            '\\' => {
                // A backslash makes the next char plain text of the
                // word's own class.
                if word_class.is_none() {
                    word_class = Some(if expect_command { Command } else { Arg });
                }
                out.push(word_class.unwrap_or(Arg));
                if i + 1 < chars.len() {
                    out.push(word_class.unwrap_or(Arg));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            '#' if word_class.is_none() => {
                // A `#` that starts a word comments out the rest.
                while i < chars.len() {
                    out.push(Comment);
                    i += 1;
                }
            }
            '$' if chars.get(i + 1) == Some(&'(') => {
                // `$(` is a substitution mark; the next word is a command.
                out.push(Operator);
                out.push(Operator);
                expect_command = true;
                word_class = None;
                redirect_pending = false;
                i += 2;
            }
            '$' => {
                let mut end = i + 1;
                if chars.get(end) == Some(&'{') {
                    while end < chars.len() && chars[end] != '}' {
                        end += 1;
                    }
                    if end < chars.len() {
                        end += 1; // the closing brace
                    }
                } else {
                    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                        end += 1;
                    }
                    if end == i + 1 && chars.get(end) == Some(&'?') {
                        end += 1; // `$?`
                    }
                }
                if end > i + 1 {
                    while i < end {
                        out.push(Variable);
                        i += 1;
                    }
                } else {
                    // A lone `$` is plain text of the word's own class.
                    out.push(word_class.unwrap_or(Arg));
                    i += 1;
                }
            }
            '|' | ';' => {
                out.push(Operator);
                expect_command = true;
                word_class = None;
                redirect_pending = false;
                i += 1;
            }
            '&' => {
                // `&>` is a redirection; a bare `&` is a separator.
                if chars.get(i + 1) == Some(&'>') {
                    out.push(Operator);
                    out.push(Operator);
                    redirect_pending = true;
                    expect_command = false;
                    word_class = None;
                    i += 2;
                } else {
                    out.push(Operator);
                    expect_command = true;
                    word_class = None;
                    redirect_pending = false;
                    i += 1;
                }
            }
            '(' | ')' => {
                out.push(Operator);
                if c == '(' {
                    expect_command = true;
                }
                word_class = None;
                redirect_pending = false;
                i += 1;
            }
            _ => {
                if word_class.is_none() {
                    // A new word begins. A redirection takes the whole
                    // run: an optional leading descriptor digit, the
                    // operator, and an optional trailing `&n`.
                    let redirect_len = redirection_len(&chars[i..]);
                    if redirect_len > 0 {
                        for _ in 0..redirect_len {
                            out.push(Operator);
                            i += 1;
                        }
                        redirect_pending = true;
                        expect_command = false;
                        continue;
                    }
                    word_class = Some(if pending_redirect {
                        Arg
                    } else if expect_command {
                        if is_assignment(&chars[i..]) {
                            Variable
                        } else {
                            expect_command = false;
                            Command
                        }
                    } else if c == '-' {
                        Flag
                    } else {
                        Arg
                    });
                }
                out.push(word_class.unwrap_or(Arg));
                i += 1;
            }
        }
    }

    out
}

/// Length of a redirection run at the start of `rest`, or 0 when `rest`
/// does not begin with one. A run is an optional descriptor digit, then
/// `>`, `>>`, `<`, `<<`, `<<<` or `&>`, then an optional `&n`.
/// A digit alone followed by nothing directional is a plain word (`42`).
fn redirection_len(rest: &[char]) -> usize {
    // An optional leading descriptor digit; `42` alone falls through to
    // the no-operator return below and stays a plain word.
    let mut idx = usize::from(rest.first().is_some_and(|c| c.is_ascii_digit()));
    let op_len = if rest[idx..].starts_with(&['<', '<', '<']) {
        3
    } else if rest[idx..].starts_with(&['<', '<']) || rest[idx..].starts_with(&['>', '>']) {
        2
    } else if rest[idx..].starts_with(&['&', '>'])
        || rest[idx..].starts_with(&['<'])
        || rest[idx..].starts_with(&['>'])
    {
        1
    } else {
        return 0;
    };
    idx += op_len;
    // Optional trailing `&n` file descriptor (`2>&1`).
    if rest.get(idx) == Some(&'&') && rest.get(idx + 1).is_some_and(|c| c.is_ascii_digit()) {
        idx += 2;
        while rest.get(idx).is_some_and(|c| c.is_ascii_digit()) {
            idx += 1;
        }
    }
    idx
}

/// Whether `rest` starts with a `NAME=value` assignment word
/// (`[A-Za-z_][A-Za-z0-9_]*=`).
fn is_assignment(rest: &[char]) -> bool {
    let mut idx = usize::from(rest.first().is_some_and(|c| c.is_alphabetic() || *c == '_'));
    if idx == 0 {
        return false;
    }
    while rest
        .get(idx)
        .is_some_and(|c| c.is_alphanumeric() || *c == '_')
    {
        idx += 1;
    }
    rest.get(idx) == Some(&'=')
}

#[cfg(test)]
mod tests {
    use super::*;
    use Class::*;

    /// Build the expected class vector from `(text, class)` pairs.
    fn expected(parts: &[(&str, Class)]) -> Vec<Class> {
        let mut out = Vec::new();
        for (text, class) in parts {
            for _ in text.chars() {
                out.push(*class);
            }
        }
        out
    }

    #[test]
    fn pipeline_colors_each_segment_command() {
        let classes = classify("cat a.log | grep foo && echo done");
        assert_eq!(
            classes,
            expected(&[
                ("cat", Command),
                (" ", Arg),
                ("a.log", Arg),
                (" ", Arg),
                ("|", Operator),
                (" ", Arg),
                ("grep", Command),
                (" ", Arg),
                ("foo", Arg),
                (" ", Arg),
                ("&&", Operator),
                (" ", Arg),
                ("echo", Command),
                (" ", Arg),
                ("done", Arg),
            ])
        );
    }

    #[test]
    fn leading_assignment_is_variable_and_next_word_is_command() {
        assert_eq!(
            classify("RUST_LOG=debug cargo test"),
            expected(&[
                ("RUST_LOG=debug", Variable),
                (" ", Arg),
                ("cargo", Command),
                (" ", Arg),
                ("test", Arg),
            ])
        );
    }

    #[test]
    fn command_substitution_starts_a_command_segment() {
        assert_eq!(
            classify("echo $(git rev-parse HEAD)"),
            expected(&[
                ("echo", Command),
                (" ", Arg),
                ("$(", Operator),
                ("git", Command),
                (" ", Arg),
                ("rev", Arg),
                ("-", Arg),
                ("parse", Arg),
                (" ", Arg),
                ("HEAD", Arg),
                (")", Operator),
            ])
        );
    }

    #[test]
    fn short_and_long_flags() {
        assert_eq!(
            classify("ls -la --color=auto /tmp"),
            expected(&[
                ("ls", Command),
                (" ", Arg),
                ("-la", Flag),
                (" ", Arg),
                ("--color=auto", Flag),
                (" ", Arg),
                ("/tmp", Arg),
            ])
        );
    }

    #[test]
    fn quoted_text_keeps_pipe_inside_string() {
        assert_eq!(
            classify("grep \"a | b\" file"),
            expected(&[
                ("grep", Command),
                (" ", Arg),
                ("\"", Str),
                ("a", Str),
                (" ", Str),
                ("|", Str),
                (" ", Str),
                ("b", Str),
                ("\"", Str),
                (" ", Arg),
                ("file", Arg),
            ])
        );
    }

    #[test]
    fn unclosed_string_runs_to_the_end() {
        assert_eq!(
            classify("echo \"never closed"),
            expected(&[("echo", Command), (" ", Arg), ("\"never closed", Str),])
        );
    }

    #[test]
    fn variables_get_their_own_class() {
        assert_eq!(
            classify("echo $HOME ${USER}"),
            expected(&[
                ("echo", Command),
                (" ", Arg),
                ("$HOME", Variable),
                (" ", Arg),
                ("${USER}", Variable),
            ])
        );
    }

    #[test]
    fn comment_runs_to_the_end() {
        assert_eq!(
            classify("make # build all"),
            expected(&[
                ("make", Command),
                (" ", Arg),
                ("#", Comment),
                (" build all", Comment),
            ])
        );
    }

    #[test]
    fn hash_inside_a_word_is_plain() {
        assert_eq!(
            classify("git log abc#1"),
            expected(&[
                ("git", Command),
                (" ", Arg),
                ("log", Arg),
                (" ", Arg),
                ("abc#1", Arg),
            ])
        );
    }

    #[test]
    fn redirections_are_operators_and_targets_are_args() {
        assert_eq!(
            classify("make 2>&1 > build.log"),
            expected(&[
                ("make", Command),
                (" ", Arg),
                ("2>&1", Operator),
                (" ", Arg),
                (">", Operator),
                (" ", Arg),
                ("build.log", Arg),
            ])
        );
    }

    #[test]
    fn separator_starts_a_command_segment() {
        assert_eq!(
            classify("cd src; ls"),
            expected(&[
                ("cd", Command),
                (" ", Arg),
                ("src", Arg),
                (";", Operator),
                (" ", Arg),
                ("ls", Command),
            ])
        );
    }

    #[test]
    fn backslash_makes_next_char_plain() {
        // The escaped dash must not become a flag.
        assert_eq!(
            classify("echo \\-x"),
            expected(&[("echo", Command), (" ", Arg), ("\\-x", Arg),])
        );
    }

    #[test]
    fn empty_label_has_no_classes() {
        assert!(classify("").is_empty());
    }

    #[test]
    fn non_ascii_text_has_one_class_per_char() {
        let label = "echo héllo 日本語";
        let classes = classify(label);
        assert_eq!(classes.len(), label.chars().count());
        // `echo` is the command; the rest stays argument.
        assert_eq!(classes[0], Command);
        assert!(classes[5..].iter().all(|&c| c == Arg || c == Str));
    }

    #[test]
    fn double_quoted_variable_stays_string() {
        assert_eq!(
            classify("echo \"$HOME\""),
            expected(&[("echo", Command), (" ", Arg), ("\"$HOME\"", Str),])
        );
    }

    #[test]
    fn single_quoted_text_is_string() {
        assert_eq!(
            classify("echo 'a | b'"),
            expected(&[("echo", Command), (" ", Arg), ("'a | b'", Str),])
        );
    }

    #[test]
    fn every_class_is_reported_somewhere() {
        let label = "cd x; FOO=1 grep -i \"a b\" $HOME | wc # note";
        let classes = classify(label);
        for class in [
            Class::Arg,
            Class::Command,
            Class::Flag,
            Class::Str,
            Class::Variable,
            Class::Operator,
            Class::Comment,
        ] {
            assert!(classes.contains(&class), "{class:?} missing for {label:?}");
        }
    }
}
