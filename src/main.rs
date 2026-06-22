use {
    anyhow::{Context, Result},
    clap::Parser,
    gepas::{
        audio::{AudioFormat, PlaybackOrder, assemble},
        elevenlabs::{ElevenLabs, SynthesisRequest},
        openai::{GenerationRequest, generate},
        sentence::{GrammarSelection, Level, to_jsonl},
        validation::{ValidationOptions, validate},
    },
    std::{env, fs, path::PathBuf},
};

const DEFAULT_TOPIC: &str = "varied everyday situations";
const DEFAULT_VOICE: &str = "GtvrBPZOsCHv7plFvX8i";
const TTS_OUTPUT_FORMAT: &str = "pcm_24000";
const INPUT_TEMPLATE: &str = "Generate exactly {count} German-English sentence pairs at CEFR level {level}.\n\
Topic: {topic}.\n\
Each German and English sentence must contain at most {max_words} words.\n\
Use these exact grammar quotas:\n{grammar_instructions}\
Set each pair's grammar field to its assigned quota name and its topic field to a short lowercase label.";
const INSTRUCTIONS: &str = "Create short, natural, standalone sentences for German-English language learners. Always put German text in the german field and English text in the english field; never swap them. The translations must have equivalent meaning and be idiomatic in both languages; do not copy German syntax into English. Use common vocabulary appropriate to the requested CEFR level. Do not include explanations, alternatives, annotations, or pronunciation guides.";

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Only generate the sentence manifest; do not create audio.
    #[arg(long)]
    text_only: bool,

    /// Number of sentence pairs to generate.
    #[arg(long, default_value_t = 3)] // todo: change default back to 100
    count: usize,

    /// OpenAI model used to generate sentence pairs.
    #[arg(long, default_value = "gpt-5.4-mini")]
    model: String,

    /// CEFR language level.
    #[arg(long, value_enum, default_value = "a2")]
    level: Level,

    /// Grammar category to cover, or "all".
    #[arg(long, default_value = "all")]
    grammar: GrammarSelection,

    /// Optional sentence topic.
    #[arg(long)]
    topic: Option<String>,

    /// Maximum number of words in each sentence.
    #[arg(long, default_value_t = 12)]
    max_words: usize,

    /// Audio output format.
    #[arg(long, value_enum, default_value = "mp3")]
    format: AudioFormat,

    /// ElevenLabs model used to synthesize speech.
    #[arg(long, default_value = "eleven_multilingual_v2")]
    tts_model: String,

    /// Language played first in each audio file.
    #[arg(long, value_enum, default_value = "english-first")]
    order: PlaybackOrder,

    /// Pause between translations, in milliseconds.
    #[arg(long, default_value_t = 800)]
    pause_ms: u64,

    /// ElevenLabs voice ID for German sentences.
    #[arg(long, default_value = DEFAULT_VOICE)]
    german_voice: String,

    /// ElevenLabs voice ID for English sentences.
    #[arg(long, default_value = DEFAULT_VOICE)]
    english_voice: String,

    /// Speech-rate multiplier.
    #[arg(long, default_value_t = 1.0)]
    speech_rate: f64,

    /// Directory for generated audio files.
    #[arg(long, default_value = "output")]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let topic = cli.topic.as_deref().unwrap_or(DEFAULT_TOPIC);
    let output_name = output_name(topic, cli.level, cli.grammar)?;

    let openai_api_key = env::var("OPENAI_API_KEY").context("OPENAI_API_KEY is not set")?;

    let elevenlabs_api_key = (!cli.text_only)
        .then(|| env::var("ELEVENLABS_API_KEY").context("ELEVENLABS_API_KEY is not set"))
        .transpose()?;

    let generated = generate(
        &openai_api_key,
        &cli.model,
        GenerationRequest {
            count: cli.count,
            level: cli.level,
            grammar: cli.grammar,
            topic,
            max_words: cli.max_words,
            input_template: INPUT_TEMPLATE,
            instructions: INSTRUCTIONS,
        },
    )?;
    let validation = validate(
        generated,
        ValidationOptions {
            expected_count: cli.count,
            level: cli.level,
            grammar: cli.grammar,
            max_words: cli.max_words,
        },
    );
    for report in validation.reports {
        eprintln!("Warning: {report}");
    }
    let pairs = validation.pairs;
    fs::create_dir_all(&cli.output_dir)
        .with_context(|| format!("failed to create {}", cli.output_dir.display()))?;
    let manifest_path = cli.output_dir.join(format!("{output_name}.jsonl"));
    fs::write(&manifest_path, to_jsonl(&pairs)?)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    println!(
        "Wrote {} sentence pairs to {}",
        pairs.len(),
        manifest_path.display()
    );

    if cli.text_only {
        return Ok(());
    }

    let clips_dir = cli.output_dir.join(".clips");
    fs::create_dir_all(&clips_dir)
        .with_context(|| format!("failed to create {}", clips_dir.display()))?;
    let elevenlabs = ElevenLabs::new(elevenlabs_api_key.context("ELEVENLABS_API_KEY is not set")?)?;
    let mut total_characters = 0;
    let mut has_character_count = false;

    for (index, pair) in pairs.iter().enumerate() {
        for (language, voice_id, text) in [
            ("de", &cli.german_voice, pair.german.as_str()),
            ("en", &cli.english_voice, pair.english.as_str()),
        ] {
            let result = elevenlabs
                .synthesize_pcm(SynthesisRequest {
                    voice_id,
                    text,
                    model: &cli.tts_model,
                    language_code: language,
                    output_format: TTS_OUTPUT_FORMAT,
                    speed: cli.speech_rate,
                })
                .with_context(|| {
                    format!("failed to synthesize sentence {} in {language}", index + 1)
                })?;
            let path = clips_dir.join(format!("{:04}-{language}.pcm", index + 1));
            fs::write(&path, result.audio)
                .with_context(|| format!("failed to write {}", path.display()))?;

            let mut metadata = Vec::new();
            if let Some(character_count) = result.character_count {
                if let Ok(count) = character_count.parse::<u64>() {
                    total_characters += count;
                    has_character_count = true;
                }
                metadata.push(format!("characters: {character_count}"));
            }
            if let Some(request_id) = result.request_id {
                metadata.push(format!("request ID: {request_id}"));
            }
            if !metadata.is_empty() {
                println!(
                    "Synthesized sentence {} in {language} ({})",
                    index + 1,
                    metadata.join(", ")
                );
            }
        }
    }

    println!(
        "Wrote {} speech clips to {}",
        pairs.len() * 2,
        clips_dir.display()
    );
    if has_character_count {
        println!("ElevenLabs reported {total_characters} characters used");
    }
    assemble(
        &clips_dir,
        &cli.output_dir,
        pairs.len(),
        cli.order,
        cli.pause_ms,
        cli.format,
        &output_name,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_only_does_not_require_voice_ids() {
        assert!(Cli::try_parse_from(["gepas", "--text-only"]).is_ok());
    }

    #[test]
    fn audio_mode_uses_default_voice_ids() {
        let cli = Cli::try_parse_from(["gepas"]).unwrap();

        assert_eq!(cli.german_voice, DEFAULT_VOICE);
        assert_eq!(cli.english_voice, DEFAULT_VOICE);
    }

    #[test]
    fn output_name_combines_topic_level_and_grammar() {
        assert_eq!(
            output_name(
                "Everyday Life!",
                Level::A2,
                GrammarSelection::Form(gepas::sentence::GrammarForm::PerfectTense)
            )
            .unwrap(),
            "everyday-life-a2-perfect-tense"
        );
        assert_eq!(
            output_name("Über das Wetter", Level::B1, GrammarSelection::All).unwrap(),
            "über-das-wetter-b1"
        );
    }
}

fn output_name(topic: &str, level: Level, grammar: GrammarSelection) -> Result<String> {
    let level = serde_json::to_value(level)?
        .as_str()
        .context("level did not serialize as a string")?
        .to_lowercase();
    let mut topic_slug = String::new();
    for character in topic.trim().chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            topic_slug.push(character);
        } else if !topic_slug.is_empty() && !topic_slug.ends_with('-') {
            topic_slug.push('-');
        }
    }
    let topic_slug = topic_slug.trim_end_matches('-');
    if topic_slug.is_empty() {
        anyhow::bail!("topic must contain at least one letter or number");
    }

    match grammar {
        GrammarSelection::All => Ok(format!("{topic_slug}-{level}")),
        GrammarSelection::Form(form) => Ok(format!(
            "{topic_slug}-{level}-{}",
            serde_json::to_value(form)?
                .as_str()
                .context("grammar form did not serialize as a string")?
        )),
    }
}
