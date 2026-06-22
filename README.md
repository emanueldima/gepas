# gepas

**German-English Parallel Audio Sentences**

gepas creates audio files containing short sentences in German and English. The
parallel recordings are intended for language learners who want to practise
listening and compare natural sentences in both languages.

## Plan

gepas will be a Rust CLI with this pipeline:

```text
CLI options
  -> generate sentence pairs
  -> validate and save a manifest
  -> synthesize German and English clips
  -> combine clips with pauses
  -> export WAV or MP3
```

### Sentence generation

An LLM will generate sentence pairs as structured data:

```json
{
  "german": "Ich habe gestern Brot gekauft.",
  "english": "I bought bread yesterday.",
  "level": "A2",
  "grammar": "perfect-tense",
  "topic": "shopping"
}
```

Generation will be controlled by CLI options for:

- number of sentences;
- OpenAI model;
- CEFR level;
- grammatical forms;
- topic;
- maximum sentence length.

An explicit grammar catalogue will provide measurable coverage instead of
relying on the model to produce variety by itself. It will include tenses,
moods, negation, questions, cases, modal verbs, and subordinate clauses.

| Minimum level | Grammar forms |
| --- | --- |
| A1 | Present statement, negation, yes/no question, question word, imperative, modal verb, accusative |
| A2 | Perfect tense, simple past, dative, separable verb, comparison, `weil` clause, `dass` clause |
| B1 | Future, conditional, passive, relative clause, reflexive verb |
| B2 | Reported speech, advanced passive, complex conjunction |

Grammar form names are stable kebab-case values such as `perfect-tense` and
`relative-clause`. `--grammar all` selects every form compatible with the
requested level.

OpenAI's Responses API with Structured Outputs is the initial choice for
generating data that follows the required schema. The default model is
`gpt-5.4-mini` and can be changed with `--model`.

### Validation and manifest

Generated sentences will be checked for empty or malformed fields, swapped
German and English fields, length, punctuation, requested level and grammar
coverage, and duplicates. Problematic pairs are reported and omitted; valid
pairs are saved beside the final audio in `--output-dir` before audio is
generated. Grammar correctness and translation equivalence are currently
enforced through the generation prompt rather than a second model review.

The manifest preserves generated text independently of the audio files so it can
be reviewed, edited, or consumed by other tools.

### Text to speech

The initial TTS choice is ElevenLabs Multilingual v2, with a configurable voice
for each language. The model can be changed with `--tts-model`. OpenAI's TTS API
is a simpler alternative but its built-in voices are optimized for English.

The relevant CLI options will include:

- `--german-voice`;
- `--english-voice`;
- `--tts-model`;
- `--speech-rate`.

Both voice IDs are required for audio runs. Validated pairs are synthesized as
24 kHz, signed 16-bit little-endian PCM clips before audio assembly:

```text
output/.clips/
  0001-de.pcm
  0001-en.pcm
  0002-de.pcm
  0002-en.pcm
```

When ElevenLabs supplies them, gepas logs the character count and request ID for
each clip, followed by the total reported character usage for the run.

### Audio assembly

All sentence pairs are combined into one lesson file containing:

```text
First language sentence
  -> configured pause
Second language sentence
  -> configured pause
Next sentence pair
```

WAV uses 24 kHz mono, signed 16-bit samples. MP3 is encoded directly in gepas
with the pure-Rust `shine-rs` encoder. Raw PCM clips remain in `.clips` for
future reuse.

Possible later modes include English-first playback, repeated target-language
sentences, separate language files, and one combined lesson file.

Example output:

```text
output/
  everyday-life-a2.mp3
  everyday-life-a2.jsonl
  .clips/
```

### CLI design

```console
gepas \
  --count 100 \
  --model gpt-5.4-mini \
  --level A2 \
  --grammar all \
  --topic everyday-life \
  --german-voice GERMAN_VOICE_ID \
  --english-voice ENGLISH_VOICE_ID \
  --format mp3 \
  --order german-first \
  --pause-ms 800 \
  --output-dir output

gepas \
  --count 100 \
  --level A2 \
  --text-only \
  --output-dir output
```

By default, gepas generates both the manifest and its audio files. `--text-only`
stops after writing the manifest so that the generated text can be inspected or
used independently.

### Building and running

```console
cargo build
cargo run -- --help
```

Set API keys through the environment rather than command-line arguments so that
they are not exposed in shell history or process listings:

```console
export OPENAI_API_KEY=...
export ELEVENLABS_API_KEY=...
```

`OPENAI_API_KEY` is required for every run. `ELEVENLABS_API_KEY` is not required
with `--text-only`.

### Implementation phases

1. **Complete:** Initialize the Rust CLI with `clap`, configuration, and
   environment-based API keys.
2. **Complete:** Define the sentence schema and grammar catalogue.
3. **Complete:** Implement structured sentence generation and JSONL output.
4. **Complete:** Add validation and duplicate detection.
5. **Complete:** Implement text-to-speech synthesis.
6. **Complete:** Assemble WAV files and encode MP3 output.
7. Make runs resumable by skipping existing outputs and retrying failed API
   calls.
8. Add tests using recorded API responses, with live API tests kept opt-in.

## Status

This project is under development.
