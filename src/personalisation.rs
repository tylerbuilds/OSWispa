//! Local-only, user-curated transcript personalisation.
//!
//! The persisted document deliberately contains only explicit phrase replacements. OSWispa does
//! not observe edits in other applications and does not learn entries automatically.

use crate::{get_data_dir, persistence};
use anyhow::{Context, Result};
use regex::{Regex, RegexBuilder};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const PERSONALISATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_DICTIONARY_ENTRIES: usize = 500;
pub const MAX_SPOKEN_CHARS: usize = 100;
pub const MAX_WRITTEN_CHARS: usize = 200;
pub const MAX_VOCABULARY_TERMS: usize = 50;
pub const MAX_VOCABULARY_PROMPT_BYTES: usize = 1024;
const MAX_IMPORT_BYTES: u64 = 1024 * 1024;
const VOCABULARY_PROMPT_PREFIX: &str = "Preferred spellings: ";

fn default_enabled() -> bool {
    true
}

/// One explicit, literal phrase replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub spoken: String,
    pub written: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersonalisationDocument {
    schema_version: u32,
    #[serde(default)]
    dictionary: Vec<DictionaryEntry>,
}

#[derive(Debug, Clone)]
struct CompiledDictionaryEntry {
    regex: Regex,
    written: String,
}

/// Validated personalisation data with cached literal matchers.
#[derive(Debug, Clone)]
pub struct Personalisation {
    document: PersonalisationDocument,
    compiled_dictionary: Vec<CompiledDictionaryEntry>,
}

impl PartialEq for Personalisation {
    fn eq(&self, other: &Self) -> bool {
        self.document == other.document
    }
}

impl Eq for Personalisation {}

impl Default for Personalisation {
    fn default() -> Self {
        Self::from_dictionary(Vec::new()).expect("an empty dictionary is valid")
    }
}

impl Serialize for Personalisation {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.document.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Personalisation {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let document = PersonalisationDocument::deserialize(deserializer)?;
        Self::from_document(document).map_err(D::Error::custom)
    }
}

#[derive(Debug)]
struct ReplacementMatch {
    start: usize,
    end: usize,
    entry_index: usize,
}

impl Personalisation {
    pub fn from_dictionary(dictionary: Vec<DictionaryEntry>) -> Result<Self> {
        Self::from_document(PersonalisationDocument {
            schema_version: PERSONALISATION_SCHEMA_VERSION,
            dictionary,
        })
    }

    fn from_document(document: PersonalisationDocument) -> Result<Self> {
        validate_document(&document)?;
        let compiled_dictionary = compile_dictionary(&document.dictionary)?;
        Ok(Self {
            document,
            compiled_dictionary,
        })
    }

    pub fn dictionary(&self) -> &[DictionaryEntry] {
        &self.document.dictionary
    }

    /// Replace literal phrases in one pass. Matches are found only in the original transcript, so
    /// replacement text is never interpreted as another dictionary entry.
    pub fn apply_dictionary(&self, text: &str) -> String {
        let mut matches = Vec::new();

        for (entry_index, entry) in self.compiled_dictionary.iter().enumerate() {
            for matched in entry.regex.find_iter(text) {
                if has_literal_boundaries(text, matched.start(), matched.end()) {
                    matches.push(ReplacementMatch {
                        start: matched.start(),
                        end: matched.end(),
                        entry_index,
                    });
                }
            }
        }

        if matches.is_empty() {
            return text.to_string();
        }

        matches.sort_by(|left, right| {
            left.start
                .cmp(&right.start)
                .then_with(|| (right.end - right.start).cmp(&(left.end - left.start)))
                .then_with(|| left.entry_index.cmp(&right.entry_index))
        });

        let mut output = String::with_capacity(text.len());
        let mut cursor = 0;
        for matched in matches {
            if matched.start < cursor {
                continue;
            }
            output.push_str(&text[cursor..matched.start]);
            output.push_str(&self.compiled_dictionary[matched.entry_index].written);
            cursor = matched.end;
        }
        output.push_str(&text[cursor..]);
        output
    }

    /// Build a stable, size-bounded Whisper prompt from enabled preferred spellings.
    pub fn vocabulary_prompt(&self) -> Option<String> {
        let mut terms: Vec<&str> = self
            .document
            .dictionary
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.written.as_str())
            .collect();
        terms.sort_by_cached_key(|term| (term.to_lowercase(), *term));
        terms.dedup_by(|left, right| left.to_lowercase() == right.to_lowercase());

        let mut prompt = VOCABULARY_PROMPT_PREFIX.to_string();
        let mut included = 0;
        for term in terms.into_iter().take(MAX_VOCABULARY_TERMS) {
            let separator = if included == 0 { "" } else { ", " };
            if prompt.len() + separator.len() + term.len() > MAX_VOCABULARY_PROMPT_BYTES {
                continue;
            }
            prompt.push_str(separator);
            prompt.push_str(term);
            included += 1;
        }

        (included > 0).then_some(prompt)
    }
}

pub fn personalisation_path() -> PathBuf {
    get_data_dir().join("personalisation.json")
}

/// Load the canonical private document, creating a versioned empty one on first use.
pub fn load_personalisation() -> Result<Personalisation> {
    let path = personalisation_path();
    if path.exists() {
        persistence::read_json_private(&path)
            .with_context(|| format!("Personalisation file is invalid: {:?}", path))
    } else {
        let personalisation = Personalisation::default();
        save_personalisation(&personalisation)?;
        Ok(personalisation)
    }
}

pub fn save_personalisation(personalisation: &Personalisation) -> Result<()> {
    persistence::write_json_private(&personalisation_path(), personalisation)
}

/// Import and validate a personalisation document without changing the selected source file.
pub fn import_personalisation(path: &Path) -> Result<Personalisation> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to inspect import file {:?}", path))?;
    if !metadata.is_file() {
        anyhow::bail!("Import path is not a regular file: {:?}", path);
    }
    if metadata.len() > MAX_IMPORT_BYTES {
        anyhow::bail!("Import file exceeds the 1 MiB limit");
    }

    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open import file {:?}", path))?;
    let mut bytes = Vec::new();
    file.take(MAX_IMPORT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_IMPORT_BYTES {
        anyhow::bail!("Import file exceeds the 1 MiB limit");
    }
    serde_json::from_slice(&bytes)
        .with_context(|| format!("Failed to parse personalisation import {:?}", path))
}

/// Export a validated snapshot without changing permissions on the destination directory.
pub fn export_personalisation(personalisation: &Personalisation, path: &Path) -> Result<()> {
    if std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        anyhow::bail!("Refusing to export through a symbolic link: {:?}", path);
    }

    let mut json = serde_json::to_vec_pretty(personalisation)?;
    json.push(b'\n');

    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("Failed to create export file {:?}", path))?;
    file.write_all(&json)?;
    file.sync_all()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

fn validate_document(document: &PersonalisationDocument) -> Result<()> {
    if document.schema_version != PERSONALISATION_SCHEMA_VERSION {
        anyhow::bail!(
            "Unsupported personalisation schema version {} (expected {})",
            document.schema_version,
            PERSONALISATION_SCHEMA_VERSION
        );
    }
    if document.dictionary.len() > MAX_DICTIONARY_ENTRIES {
        anyhow::bail!(
            "Dictionary contains {} entries; the limit is {}",
            document.dictionary.len(),
            MAX_DICTIONARY_ENTRIES
        );
    }

    let mut spoken_phrases = HashSet::new();
    for (index, entry) in document.dictionary.iter().enumerate() {
        validate_field(&entry.spoken, "spoken phrase", MAX_SPOKEN_CHARS, index)?;
        validate_field(&entry.written, "written phrase", MAX_WRITTEN_CHARS, index)?;

        let duplicate_key = entry.spoken.to_lowercase();
        if !spoken_phrases.insert(duplicate_key) {
            anyhow::bail!("Dictionary entry {} duplicates a spoken phrase", index + 1);
        }
    }

    Ok(())
}

fn validate_field(value: &str, field: &str, max_chars: usize, index: usize) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("Dictionary entry {} has an empty {}", index + 1, field);
    }
    if value.trim() != value {
        anyhow::bail!(
            "Dictionary entry {} {} has leading or trailing whitespace",
            index + 1,
            field
        );
    }
    if value.chars().count() > max_chars {
        anyhow::bail!(
            "Dictionary entry {} {} exceeds {} characters",
            index + 1,
            field,
            max_chars
        );
    }
    if value.chars().any(char::is_control) {
        anyhow::bail!(
            "Dictionary entry {} {} contains a control character",
            index + 1,
            field
        );
    }
    Ok(())
}

fn compile_dictionary(entries: &[DictionaryEntry]) -> Result<Vec<CompiledDictionaryEntry>> {
    entries
        .iter()
        .filter(|entry| entry.enabled)
        .map(|entry| {
            let regex = RegexBuilder::new(&regex::escape(&entry.spoken))
                .case_insensitive(!entry.case_sensitive)
                .unicode(true)
                .build()
                .context("Failed to compile dictionary entry")?;
            Ok(CompiledDictionaryEntry {
                regex,
                written: entry.written.clone(),
            })
        })
        .collect()
}

fn has_literal_boundaries(text: &str, start: usize, end: usize) -> bool {
    let before_is_word = text[..start]
        .chars()
        .next_back()
        .map(is_word_character)
        .unwrap_or(false);
    let after_is_word = text[end..]
        .chars()
        .next()
        .map(is_word_character)
        .unwrap_or(false);
    !before_is_word && !after_is_word
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(spoken: &str, written: &str) -> DictionaryEntry {
        DictionaryEntry {
            spoken: spoken.to_string(),
            written: written.to_string(),
            enabled: true,
            case_sensitive: false,
        }
    }

    #[test]
    fn document_round_trip_is_versioned_and_validated() {
        let original =
            Personalisation::from_dictionary(vec![entry("os whisper", "OSWispa")]).unwrap();
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("\"schema_version\":1"));
        let loaded: Personalisation = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn rejects_unknown_schema_empty_fields_controls_duplicates_and_limits() {
        let unknown = r#"{"schema_version":2,"dictionary":[]}"#;
        assert!(serde_json::from_str::<Personalisation>(unknown).is_err());
        assert!(Personalisation::from_dictionary(vec![entry("", "value")]).is_err());
        assert!(Personalisation::from_dictionary(vec![entry("bad\0phrase", "value")]).is_err());
        assert!(Personalisation::from_dictionary(vec![entry(" phrase", "value")]).is_err());
        assert!(Personalisation::from_dictionary(vec![entry(
            &"s".repeat(MAX_SPOKEN_CHARS + 1),
            "value"
        )])
        .is_err());
        assert!(Personalisation::from_dictionary(vec![entry(
            "phrase",
            &"w".repeat(MAX_WRITTEN_CHARS + 1)
        )])
        .is_err());
        assert!(Personalisation::from_dictionary(vec![
            entry("OS Wisper", "OSWispa"),
            entry("os wisper", "OSWispa")
        ])
        .is_err());
        assert!(Personalisation::from_dictionary(
            (0..=MAX_DICTIONARY_ENTRIES)
                .map(|index| entry(&format!("phrase {index}"), "value"))
                .collect()
        )
        .is_err());
    }

    #[test]
    fn longest_literal_phrase_wins_at_word_boundaries() {
        let dictionary = Personalisation::from_dictionary(vec![
            entry("whisper", "Whisper"),
            entry("os whisper", "OSWispa"),
            entry("cat", "dog"),
        ])
        .unwrap();

        assert_eq!(
            dictionary.apply_dictionary("try os whisper and concatenate cat"),
            "try OSWispa and concatenate dog"
        );
    }

    #[test]
    fn empty_dictionary_preserves_text_exactly() {
        let text = "Keep  spacing, punctuation, and\nline breaks.";
        assert_eq!(Personalisation::default().apply_dictionary(text), text);
        assert_eq!(Personalisation::default().vocabulary_prompt(), None);
    }

    #[test]
    fn replacement_is_case_aware_and_non_recursive() {
        let mut exact = entry("api", "API");
        exact.case_sensitive = true;
        let dictionary = Personalisation::from_dictionary(vec![
            exact,
            entry("voice app", "os whisper"),
            entry("os whisper", "OSWispa"),
        ])
        .unwrap();

        assert_eq!(dictionary.apply_dictionary("Api api"), "Api API");
        assert_eq!(
            dictionary.apply_dictionary("the voice app works"),
            "the os whisper works"
        );
    }

    #[test]
    fn disabled_entries_and_unicode_are_handled_safely() {
        let mut disabled = entry("resume", "résumé");
        disabled.enabled = false;
        let dictionary =
            Personalisation::from_dictionary(vec![disabled, entry("café noir", "Café Noir")])
                .unwrap();
        assert_eq!(
            dictionary.apply_dictionary("CAFÉ NOIR and resume"),
            "Café Noir and resume"
        );
    }

    #[test]
    fn vocabulary_prompt_is_deterministic_deduplicated_and_bounded() {
        let dictionary = Personalisation::from_dictionary(vec![
            entry("zulu", "Zulu"),
            entry("alpha", "Alpha"),
            entry("another alpha", "alpha"),
        ])
        .unwrap();
        assert_eq!(
            dictionary.vocabulary_prompt().as_deref(),
            Some("Preferred spellings: Alpha, Zulu")
        );

        let many = Personalisation::from_dictionary(
            (0..MAX_DICTIONARY_ENTRIES)
                .map(|index| entry(&format!("spoken {index}"), &format!("written-{index:03}")))
                .collect(),
        )
        .unwrap();
        let prompt = many.vocabulary_prompt().unwrap();
        assert!(prompt.len() <= MAX_VOCABULARY_PROMPT_BYTES);
        assert_eq!(prompt.matches(", ").count() + 1, MAX_VOCABULARY_TERMS);
    }

    #[test]
    fn import_and_export_round_trip_without_following_export_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("dictionary.json");
        let original =
            Personalisation::from_dictionary(vec![entry("tyler casey", "Tyler Casey")]).unwrap();
        export_personalisation(&original, &path).unwrap();
        assert_eq!(import_personalisation(&path).unwrap(), original);

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link = directory.path().join("link.json");
            symlink(&path, &link).unwrap();
            assert!(export_personalisation(&original, &link).is_err());
        }
    }
}
