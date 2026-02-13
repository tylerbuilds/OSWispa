//! Punctuation command processing
//!
//! Converts spoken punctuation commands into actual punctuation marks.
//! For example: "Hello period" becomes "Hello."

use regex::Regex;
use std::sync::LazyLock;

/// Punctuation command mappings
static PUNCTUATION_COMMANDS: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
    vec![
        // End punctuation
        ("period", "."),
        ("full stop", "."),
        ("dot", "."),
        ("comma", ","),
        ("exclamation point", "!"),
        ("exclamation mark", "!"),
        ("question mark", "?"),
        ("semicolon", ";"),
        ("colon", ":"),
        ("ellipsis", "..."),

        // Quotes and brackets
        ("open quote", "\""),
        ("close quote", "\""),
        ("quote", "\""),
        ("quotation mark", "\""),
        ("double quote", "\""),
        ("open paren", "("),
        ("close paren", ")"),
        ("open parenthesis", "("),
        ("close parenthesis", ")"),
        ("open bracket", "["),
        ("close bracket", "]"),
        ("open brace", "{"),
        ("close brace", "}"),

        // Other punctuation
        ("hyphen", "-"),
        ("dash", "—"),
        ("underscore", "_"),
        ("apostrophe", "'"),
        ("at sign", "@"),
        ("hashtag", "#"),
        ("hash", "#"),
        ("ampersand", "&"),
        ("asterisk", "*"),
        ("percent", "%"),
        ("dollar sign", "$"),
        ("plus sign", "+"),
        ("equals sign", "="),
        ("slash", "/"),
        ("backslash", "\\"),

        // Formatting commands
        ("new line", "\n"),
        ("newline", "\n"),
        ("line break", "\n"),
        ("new paragraph", "\n\n"),
        ("tab", "\t"),
    ]
});

/// Regex patterns for case-insensitive matching
static PUNCTUATION_REGEX: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    PUNCTUATION_COMMANDS
        .iter()
        .map(|(pattern, replacement)| {
            // Match the pattern with optional surrounding spaces
            // Word boundaries to avoid matching partial words
            let regex = Regex::new(&format!(r"(?i)\s*\b{}\b\s*", regex::escape(pattern)))
                .expect("Invalid regex pattern");
            (regex, *replacement)
        })
        .collect()
});

/// Apply punctuation commands to transcribed text
///
/// Converts spoken commands like "period" to actual punctuation.
/// Handles spacing appropriately:
/// - "hello period" -> "hello."
/// - "hello comma world" -> "hello, world"
pub fn apply_punctuation_commands(text: &str) -> String {
    let mut result = text.to_string();

    for (regex, replacement) in PUNCTUATION_REGEX.iter() {
        // Determine proper spacing based on punctuation type
        let formatted_replacement = match *replacement {
            // End punctuation - no space before, space after (trimmed at end)
            "." | "," | "!" | "?" | ";" | ":" | "..." => {
                format!("{} ", replacement)
            }
            // Opening brackets - space before, no space after
            "(" | "[" | "{" | "\"" => {
                // Check if it's likely an opening quote by context
                format!(" {}", replacement)
            }
            // Closing brackets - no space before, space after
            ")" | "]" | "}" => {
                format!("{} ", replacement)
            }
            // Newlines - just the character
            "\n" | "\n\n" | "\t" => replacement.to_string(),
            // Everything else - replace with appropriate spacing
            _ => replacement.to_string(),
        };

        result = regex.replace_all(&result, formatted_replacement.as_str()).to_string();
    }

    // Clean up multiple spaces
    let multi_space = Regex::new(r" {2,}").unwrap();
    result = multi_space.replace_all(&result, " ").to_string();

    // Clean up spaces before punctuation
    let space_before_punct = Regex::new(r" ([.,!?;:])").unwrap();
    result = space_before_punct.replace_all(&result, "$1").to_string();

    // Normalize spacing around explicit line breaks from voice commands.
    let space_around_newline = Regex::new(r"[ \t]*\n[ \t]*").unwrap();
    result = space_around_newline.replace_all(&result, "\n").to_string();

    // Trim horizontal whitespace only; preserve intentional newlines.
    result
        .trim_matches(|c| c == ' ' || c == '\t')
        .to_string()
}

/// Check if a word is a punctuation command
pub fn is_punctuation_command(word: &str) -> bool {
    let lower = word.to_lowercase();
    PUNCTUATION_COMMANDS.iter().any(|(cmd, _)| *cmd == lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period() {
        assert_eq!(apply_punctuation_commands("hello period"), "hello.");
        assert_eq!(apply_punctuation_commands("hello world period"), "hello world.");
    }

    #[test]
    fn test_comma() {
        assert_eq!(
            apply_punctuation_commands("hello comma world"),
            "hello, world"
        );
    }

    #[test]
    fn test_question_mark() {
        assert_eq!(
            apply_punctuation_commands("how are you question mark"),
            "how are you?"
        );
    }

    #[test]
    fn test_multiple_punctuation() {
        assert_eq!(
            apply_punctuation_commands("hello comma how are you question mark"),
            "hello, how are you?"
        );
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(apply_punctuation_commands("hello PERIOD"), "hello.");
        assert_eq!(apply_punctuation_commands("hello Period"), "hello.");
    }

    #[test]
    fn test_new_line() {
        assert_eq!(
            apply_punctuation_commands("hello new line world"),
            "hello\nworld"
        );
    }

    #[test]
    fn test_quotation_mark_alias() {
        let output = apply_punctuation_commands("hello quotation mark world quotation mark");
        assert_eq!(output.matches('"').count(), 2);
    }

    #[test]
    fn test_trailing_newline_preserved() {
        assert_eq!(apply_punctuation_commands("note new line"), "note\n");
    }

    #[test]
    fn test_exclamation() {
        assert_eq!(
            apply_punctuation_commands("wow exclamation point"),
            "wow!"
        );
    }

    #[test]
    fn test_no_commands() {
        assert_eq!(
            apply_punctuation_commands("hello world"),
            "hello world"
        );
    }

    #[test]
    fn test_is_punctuation_command() {
        assert!(is_punctuation_command("period"));
        assert!(is_punctuation_command("Period"));
        assert!(is_punctuation_command("COMMA"));
        assert!(!is_punctuation_command("hello"));
    }
}
