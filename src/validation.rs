use crate::sentence::{GrammarSelection, Level, SentencePair};

pub struct ValidationOptions {
    pub expected_count: usize,
    pub level: Level,
    pub grammar: GrammarSelection,
    pub max_words: usize,
}

pub struct ValidationResult {
    pub pairs: Vec<SentencePair>,
    pub reports: Vec<String>,
}

pub fn validate(pairs: Vec<SentencePair>, options: ValidationOptions) -> ValidationResult {
    private::validate(pairs, options)
}

mod private {
    use {
        crate::sentence::{GrammarForm, grammar_quotas},
        lingua::{Language, LanguageDetectorBuilder},
        std::collections::{HashMap, HashSet},
    };

    pub(super) fn validate(
        pairs: Vec<super::SentencePair>,
        options: super::ValidationOptions,
    ) -> super::ValidationResult {
        let mut reports = Vec::new();
        if pairs.len() != options.expected_count {
            reports.push(format!(
                "generation returned {} pairs, requested {}",
                pairs.len(),
                options.expected_count
            ));
        }

        let expected = match grammar_quotas(options.level, options.grammar, options.expected_count)
        {
            Ok(quotas) => quotas
                .into_iter()
                .map(|quota| (quota.spec.form, quota.count))
                .collect::<HashMap<_, _>>(),
            Err(error) => {
                reports.push(error);
                HashMap::new()
            }
        };
        let mut actual = HashMap::<GrammarForm, usize>::new();
        let mut german_sentences = HashSet::new();
        let mut english_sentences = HashSet::new();
        let mut valid = Vec::new();
        let language_detector =
            LanguageDetectorBuilder::from_languages(&[Language::German, Language::English]).build();

        for (index, pair) in pairs.into_iter().enumerate() {
            let mut problems = Vec::new();
            for (language, sentence) in [("German", &pair.german), ("English", &pair.english)] {
                if sentence.is_empty() {
                    problems.push(format!("{language} sentence is empty"));
                    continue;
                }
                if sentence.trim() != sentence {
                    problems.push(format!("{language} sentence has surrounding whitespace"));
                }
                if sentence.contains('\n') || sentence.contains('\r') {
                    problems.push(format!("{language} sentence contains a newline"));
                }
                if sentence.split_whitespace().count() > options.max_words {
                    problems.push(format!(
                        "{language} sentence exceeds {} words",
                        options.max_words
                    ));
                }
                if !matches!(sentence.chars().last(), Some('.' | '?' | '!')) {
                    problems.push(format!("{language} sentence has no terminal punctuation"));
                }
            }
            if pair.topic.trim().is_empty() {
                problems.push("topic is empty".into());
            }
            if pair.level != options.level {
                problems.push(format!(
                    "level is {:?}, expected {:?}",
                    pair.level, options.level
                ));
            }
            if !expected.contains_key(&pair.grammar) {
                problems.push(format!("grammar {:?} was not requested", pair.grammar));
            }

            let german = normalize(&pair.german);
            let english = normalize(&pair.english);
            if !german.is_empty() && german == english {
                problems.push("German and English sentences are identical".into());
            }
            if language_detector.detect_language_of(&pair.german) == Some(Language::English)
                && language_detector.detect_language_of(&pair.english) == Some(Language::German)
            {
                problems.push("German and English fields appear to be swapped".into());
            }
            if problems.is_empty() && german_sentences.contains(&german) {
                problems.push("German sentence duplicates an earlier valid pair".into());
            }
            if problems.is_empty() && english_sentences.contains(&english) {
                problems.push("English sentence duplicates an earlier valid pair".into());
            }
            if problems.is_empty()
                && actual.get(&pair.grammar).copied().unwrap_or_default()
                    >= expected.get(&pair.grammar).copied().unwrap_or_default()
            {
                problems.push(format!("grammar {:?} exceeds its quota", pair.grammar));
            }

            if !problems.is_empty() {
                reports.push(format!(
                    "sentence {} dropped: {}",
                    index + 1,
                    problems.join("; ")
                ));
                continue;
            }

            german_sentences.insert(german);
            english_sentences.insert(english);
            *actual.entry(pair.grammar).or_default() += 1;
            valid.push(pair);
        }

        for (grammar, expected_count) in expected {
            let actual_count = actual.get(&grammar).copied().unwrap_or_default();
            if actual_count < expected_count {
                reports.push(format!(
                "grammar {grammar:?} retained {actual_count} of {expected_count} requested pairs"
            ));
            }
        }

        super::ValidationResult {
            pairs: valid,
            reports,
        }
    }

    fn normalize(sentence: &str) -> String {
        sentence
            .chars()
            .flat_map(char::to_lowercase)
            .filter(|character| character.is_alphanumeric())
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use {super::super::*, crate::sentence::Level};

        #[test]
        fn valid_pairs_are_retained() {
            let result = validate(
                vec![pair("Ich komme morgen.", "I am coming tomorrow.")],
                options(1),
            );

            assert_eq!(result.pairs.len(), 1);
            assert!(result.reports.is_empty());
        }

        #[test]
        fn swapped_language_fields_are_dropped() {
            let result = validate(
                vec![pair("Do you have time today?", "Hast du heute Zeit?")],
                options(1),
            );

            assert!(result.pairs.is_empty());
            assert!(
                result
                    .reports
                    .iter()
                    .any(|report| report.contains("swapped"))
            );
        }

        #[test]
        fn malformed_and_duplicate_pairs_are_dropped() {
            let result = validate(
                vec![
                    pair("Ich komme morgen.", "I am coming tomorrow."),
                    pair("Ich komme morgen!", "Tomorrow I am coming."),
                    pair(
                        " Zu viele Wörter stehen jetzt in diesem sehr langen Satz.",
                        "Too long.",
                    ),
                    pair("Hotel.", "Hotel."),
                ],
                options(4),
            );

            assert_eq!(result.pairs.len(), 1);
            assert!(
                result
                    .reports
                    .iter()
                    .any(|report| report.contains("duplicates"))
            );
            assert!(
                result
                    .reports
                    .iter()
                    .any(|report| report.contains("surrounding whitespace"))
            );
            assert!(
                result
                    .reports
                    .iter()
                    .any(|report| report.contains("identical"))
            );
        }

        #[test]
        fn wrong_metadata_and_quota_overflow_are_dropped() {
            let mut wrong_level = pair("Ich komme morgen.", "I am coming tomorrow.");
            wrong_level.level = Level::A2;
            let mut wrong_grammar = pair("Ich bleibe hier.", "I am staying here.");
            wrong_grammar.grammar = super::GrammarForm::Negation;
            let result = validate(
                vec![
                    wrong_level,
                    wrong_grammar,
                    pair("Ich lerne heute.", "I am studying today."),
                    pair("Wir arbeiten hier.", "We work here."),
                ],
                options(1),
            );

            assert_eq!(result.pairs.len(), 1);
            assert!(
                result
                    .reports
                    .iter()
                    .any(|report| report.contains("level is"))
            );
            assert!(
                result
                    .reports
                    .iter()
                    .any(|report| report.contains("not requested"))
            );
            assert!(
                result
                    .reports
                    .iter()
                    .any(|report| report.contains("exceeds its quota"))
            );
        }

        fn pair(german: &str, english: &str) -> SentencePair {
            SentencePair {
                german: german.into(),
                english: english.into(),
                level: Level::A1,
                grammar: super::GrammarForm::PresentStatement,
                topic: "daily-life".into(),
            }
        }

        fn options(expected_count: usize) -> ValidationOptions {
            ValidationOptions {
                expected_count,
                level: Level::A1,
                grammar: GrammarSelection::Form(super::GrammarForm::PresentStatement),
                max_words: 8,
            }
        }
    }
}
