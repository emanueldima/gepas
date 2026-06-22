use {
    clap::ValueEnum,
    serde::{Deserialize, Serialize},
    std::str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentencePair {
    pub german: String,
    pub english: String,
    pub level: Level,
    pub grammar: GrammarForm,
    pub topic: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum, Serialize, Deserialize,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    A1,
    A2,
    B1,
    B2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrammarForm {
    PresentStatement,
    Negation,
    YesNoQuestion,
    QuestionWord,
    Imperative,
    ModalVerb,
    Accusative,
    PerfectTense,
    SimplePast,
    Dative,
    SeparableVerb,
    Comparison,
    WeilClause,
    DassClause,
    Future,
    Conditional,
    Passive,
    RelativeClause,
    ReflexiveVerb,
    ReportedSpeech,
    AdvancedPassive,
    ComplexConjunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarSelection {
    All,
    Form(GrammarForm),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrammarSpec {
    pub form: GrammarForm,
    pub min_level: Level,
    pub german_instruction: &'static str,
    pub english_instruction: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrammarQuota {
    pub spec: &'static GrammarSpec,
    pub count: usize,
}

pub const GRAMMAR_CATALOGUE: &[GrammarSpec] = &[
    GrammarSpec {
        form: GrammarForm::PresentStatement,
        min_level: Level::A1,
        german_instruction: "Use a short statement in the present tense.",
        english_instruction: "Translate it as a natural present-tense statement.",
    },
    GrammarSpec {
        form: GrammarForm::Negation,
        min_level: Level::A1,
        german_instruction: "Negate a simple statement with nicht or kein.",
        english_instruction: "Use the natural English negative form.",
    },
    GrammarSpec {
        form: GrammarForm::YesNoQuestion,
        min_level: Level::A1,
        german_instruction: "Ask a simple yes-or-no question with verb-first word order.",
        english_instruction: "Ask the equivalent natural yes-or-no question.",
    },
    GrammarSpec {
        form: GrammarForm::QuestionWord,
        min_level: Level::A1,
        german_instruction: "Ask a simple question beginning with a question word.",
        english_instruction: "Use the corresponding English question word.",
    },
    GrammarSpec {
        form: GrammarForm::Imperative,
        min_level: Level::A1,
        german_instruction: "Use a common informal or polite imperative.",
        english_instruction: "Translate it as a natural English instruction.",
    },
    GrammarSpec {
        form: GrammarForm::ModalVerb,
        min_level: Level::A1,
        german_instruction: "Use a common modal verb with an infinitive.",
        english_instruction: "Use the closest natural English modal construction.",
    },
    GrammarSpec {
        form: GrammarForm::Accusative,
        min_level: Level::A1,
        german_instruction: "Use an explicit masculine or neuter accusative object.",
        english_instruction: "Translate the object naturally without imitating German case marking.",
    },
    GrammarSpec {
        form: GrammarForm::PerfectTense,
        min_level: Level::A2,
        german_instruction: "Use conversational Perfekt with haben or sein.",
        english_instruction: "Choose simple past or present perfect according to the meaning.",
    },
    GrammarSpec {
        form: GrammarForm::SimplePast,
        min_level: Level::A2,
        german_instruction: "Use Präteritum with a common verb where it sounds natural.",
        english_instruction: "Use a natural English past-tense equivalent.",
    },
    GrammarSpec {
        form: GrammarForm::Dative,
        min_level: Level::A2,
        german_instruction: "Use an explicit dative object or common dative preposition.",
        english_instruction: "Translate the relationship naturally rather than copying German case.",
    },
    GrammarSpec {
        form: GrammarForm::SeparableVerb,
        min_level: Level::A2,
        german_instruction: "Use a common separable-prefix verb with correct word order.",
        english_instruction: "Use the natural English verb or phrasal verb.",
    },
    GrammarSpec {
        form: GrammarForm::Comparison,
        min_level: Level::A2,
        german_instruction: "Use a comparative or superlative adjective.",
        english_instruction: "Preserve the comparison in idiomatic English.",
    },
    GrammarSpec {
        form: GrammarForm::WeilClause,
        min_level: Level::A2,
        german_instruction: "Use a main clause and a weil clause with final verb position.",
        english_instruction: "Use a natural clause introduced by because.",
    },
    GrammarSpec {
        form: GrammarForm::DassClause,
        min_level: Level::A2,
        german_instruction: "Use a dass clause with correct final verb position.",
        english_instruction: "Use a natural English that clause, omitting that if appropriate.",
    },
    GrammarSpec {
        form: GrammarForm::Future,
        min_level: Level::B1,
        german_instruction: "Use Futur I where it is meaningful rather than merely possible.",
        english_instruction: "Express the future naturally in English.",
    },
    GrammarSpec {
        form: GrammarForm::Conditional,
        min_level: Level::B1,
        german_instruction: "Use würde or Konjunktiv II for a realistic hypothetical situation.",
        english_instruction: "Use the corresponding natural English conditional.",
    },
    GrammarSpec {
        form: GrammarForm::Passive,
        min_level: Level::B1,
        german_instruction: "Use a clear present or past process passive construction.",
        english_instruction: "Keep the passive only when it sounds natural in English.",
    },
    GrammarSpec {
        form: GrammarForm::RelativeClause,
        min_level: Level::B1,
        german_instruction: "Use a short relative clause with the correct relative pronoun.",
        english_instruction: "Use an idiomatic English relative clause.",
    },
    GrammarSpec {
        form: GrammarForm::ReflexiveVerb,
        min_level: Level::B1,
        german_instruction: "Use a common reflexive verb with the correct reflexive pronoun.",
        english_instruction: "Translate idiomatically, whether or not English uses a reflexive pronoun.",
    },
    GrammarSpec {
        form: GrammarForm::ReportedSpeech,
        min_level: Level::B2,
        german_instruction: "Use concise reported speech with Konjunktiv I where appropriate.",
        english_instruction: "Use natural English reported speech and tense sequence.",
    },
    GrammarSpec {
        form: GrammarForm::AdvancedPassive,
        min_level: Level::B2,
        german_instruction: "Use a passive construction with a modal verb or an impersonal passive.",
        english_instruction: "Choose an equivalent passive or impersonal English construction.",
    },
    GrammarSpec {
        form: GrammarForm::ComplexConjunction,
        min_level: Level::B2,
        german_instruction: "Connect clauses with obwohl, während, nachdem, or bevor.",
        english_instruction: "Use the equivalent English conjunction with natural word order.",
    },
];

impl FromStr for GrammarSelection {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("all") {
            return Ok(Self::All);
        }

        GrammarForm::from_str(value, true)
            .map(Self::Form)
            .map_err(|_| format!("unknown grammar form {value:?}"))
    }
}

pub fn grammar_quotas(
    level: Level,
    selection: GrammarSelection,
    count: usize,
) -> Result<Vec<GrammarQuota>, String> {
    let selected = match selection {
        GrammarSelection::All => GRAMMAR_CATALOGUE
            .iter()
            .filter(|spec| spec.min_level <= level)
            .collect::<Vec<_>>(),
        GrammarSelection::Form(form) => {
            let spec = GRAMMAR_CATALOGUE
                .iter()
                .find(|spec| spec.form == form)
                .ok_or_else(|| format!("grammar form {form:?} is missing from the catalogue"))?;
            if spec.min_level > level {
                return Err(format!(
                    "grammar form {form:?} requires level {:?} or higher",
                    spec.min_level
                ));
            }
            vec![spec]
        }
    };
    let mut quotas = vec![0; selected.len()];
    for index in 0..count {
        quotas[index % selected.len()] += 1;
    }

    Ok(selected
        .into_iter()
        .zip(quotas)
        .filter_map(|(spec, count)| (count > 0).then_some(GrammarQuota { spec, count }))
        .collect())
}

pub fn to_jsonl(pairs: &[SentencePair]) -> Result<String, serde_json::Error> {
    let mut output = pairs
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_pair_has_stable_json() {
        let pair = SentencePair {
            german: "Ich komme morgen.".into(),
            english: "I am coming tomorrow.".into(),
            level: Level::A1,
            grammar: GrammarForm::PresentStatement,
            topic: "plans".into(),
        };
        let json = serde_json::to_string(&pair).unwrap();

        assert_eq!(
            json,
            r#"{"german":"Ich komme morgen.","english":"I am coming tomorrow.","level":"A1","grammar":"present-statement","topic":"plans"}"#
        );
        assert_eq!(serde_json::from_str::<SentencePair>(&json).unwrap(), pair);
        assert_eq!(to_jsonl(&[pair]).unwrap(), format!("{json}\n"));
    }

    #[test]
    fn catalogue_contains_every_form_once_and_in_level_order() {
        let forms = GRAMMAR_CATALOGUE
            .iter()
            .map(|spec| spec.form)
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(forms.len(), GRAMMAR_CATALOGUE.len());
        assert_eq!(forms.len(), GrammarForm::value_variants().len());
        assert!(
            GRAMMAR_CATALOGUE
                .windows(2)
                .all(|pair| pair[0].min_level <= pair[1].min_level)
        );
    }

    #[test]
    fn grammar_selection_accepts_all_and_catalogue_names() {
        assert_eq!("all".parse(), Ok(GrammarSelection::All));
        assert_eq!(
            "relative-clause".parse(),
            Ok(GrammarSelection::Form(GrammarForm::RelativeClause))
        );
        assert!("relative".parse::<GrammarSelection>().is_err());
    }

    #[test]
    fn grammar_quotas_cover_compatible_forms() {
        let quotas = grammar_quotas(Level::A1, GrammarSelection::All, 8).unwrap();

        assert_eq!(quotas.len(), 7);
        assert_eq!(quotas[0].count, 2);
        assert_eq!(quotas.iter().map(|quota| quota.count).sum::<usize>(), 8);
        assert!(
            grammar_quotas(
                Level::A1,
                GrammarSelection::Form(GrammarForm::RelativeClause),
                1
            )
            .unwrap_err()
            .contains("requires level B1")
        );
    }
}
