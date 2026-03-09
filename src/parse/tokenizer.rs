use crate::error::{CurlJackError, Result};

/// Replace `\<newline>` line continuations only when outside of quotes.
/// Inside single or double quotes, `\<newline>` is preserved as-is.
fn strip_continuations(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                out.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                out.push(ch);
            }
            '\\' if !in_single && !in_double => {
                if chars.peek() == Some(&'\n') {
                    // Backslash-newline outside quotes: skip both (line continuation)
                    chars.next();
                } else {
                    out.push(ch);
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Preprocess the input string: strip leading `$`, handle `\` line continuations,
/// then use shell-words for POSIX shell tokenization.
pub fn tokenize(input: &str) -> Result<Vec<String>> {
    let input = input.trim();
    let input = input.strip_prefix('$').unwrap_or(input).trim();
    let input = strip_continuations(input);

    shell_words::split(&input).map_err(|e| CurlJackError::Tokenize(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenize() {
        let tokens = tokenize("curl https://example.com").unwrap();
        assert_eq!(tokens, vec!["curl", "https://example.com"]);
    }

    #[test]
    fn test_strip_dollar() {
        let tokens = tokenize("$ curl https://example.com").unwrap();
        assert_eq!(tokens, vec!["curl", "https://example.com"]);
    }

    #[test]
    fn test_line_continuation() {
        let tokens = tokenize("curl \\\n  -X POST \\\n  https://example.com").unwrap();
        assert_eq!(tokens, vec!["curl", "-X", "POST", "https://example.com"]);
    }

    #[test]
    fn test_single_quotes() {
        let tokens = tokenize("curl -H 'Content-Type: application/json' https://example.com").unwrap();
        assert_eq!(tokens, vec!["curl", "-H", "Content-Type: application/json", "https://example.com"]);
    }

    #[test]
    fn test_double_quotes() {
        let tokens = tokenize(r#"curl -d "{\"key\": \"value\"}" https://example.com"#).unwrap();
        assert_eq!(tokens, vec!["curl", "-d", r#"{"key": "value"}"#, "https://example.com"]);
    }

    #[test]
    fn test_multiline_single_quoted_data() {
        let input = "curl \\\n--data 'line1\nline2\nline3' \\\nhttps://example.com";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens, vec!["curl", "--data", "line1\nline2\nline3", "https://example.com"]);
    }

    #[test]
    fn test_newline_preserved_in_quotes() {
        // Backslash-newline inside single quotes is literal, not a continuation
        let input = "curl -d 'hello\\\nworld' https://example.com";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens, vec!["curl", "-d", "hello\\\nworld", "https://example.com"]);
    }
}
