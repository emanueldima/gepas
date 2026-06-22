use {
    crate::sentence::{GrammarSelection, Level, SentencePair, grammar_quotas},
    anyhow::{Context, Result, bail},
    reqwest::blocking::Client,
    serde::Deserialize,
    serde_json::{Value, json},
    std::time::Duration,
};

pub struct GenerationRequest<'a> {
    pub count: usize,
    pub level: Level,
    pub grammar: GrammarSelection,
    pub topic: &'a str,
    pub max_words: usize,
    pub input_template: &'a str,
    pub instructions: &'a str,
}

pub fn generate(
    api_key: &str,
    model: &str,
    request: GenerationRequest<'_>,
) -> Result<Vec<SentencePair>> {
    let body = private::build_request(model, request)?;
    let response = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to create OpenAI HTTP client")?
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .context("failed to call the OpenAI Responses API")?;
    let status = response.status();
    let body = response
        .text()
        .context("failed to read the OpenAI response")?;

    if !status.is_success() {
        bail!("OpenAI API returned {status}: {body}");
    }

    private::parse_response(&body)
}

mod private {
    use super::*;

    #[derive(Deserialize)]
    struct GeneratedPairs {
        pairs: Vec<SentencePair>,
    }

    #[derive(Deserialize)]
    struct ApiResponse {
        output: Vec<OutputItem>,
    }

    #[derive(Deserialize)]
    struct OutputItem {
        #[serde(default)]
        content: Vec<ContentItem>,
    }

    #[derive(Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum ContentItem {
        OutputText {
            text: String,
        },
        Refusal {
            refusal: String,
        },
        #[serde(other)]
        Other,
    }

    pub(super) fn build_request(model: &str, request: GenerationRequest<'_>) -> Result<Value> {
        if request.count == 0 {
            bail!("sentence count must be greater than zero");
        }
        if request.max_words == 0 {
            bail!("maximum word count must be greater than zero");
        }

        let mut grammar_instructions = String::new();
        let mut grammar_names = Vec::new();
        for quota in grammar_quotas(request.level, request.grammar, request.count)
            .map_err(anyhow::Error::msg)?
        {
            let spec = quota.spec;
            let name = serde_json::to_value(spec.form)?
                .as_str()
                .context("grammar form did not serialize as a string")?
                .to_owned();
            grammar_names.push(name.clone());
            grammar_instructions.push_str(&format!(
                "- {name}: exactly {} pair(s). German: {} English: {}\n",
                quota.count, spec.german_instruction, spec.english_instruction
            ));
        }

        let level = serde_json::to_value(request.level)?;
        let input = request
            .input_template
            .replace("{count}", &request.count.to_string())
            .replace(
                "{level}",
                level
                    .as_str()
                    .context("level did not serialize as a string")?,
            )
            .replace("{max_words}", &request.max_words.to_string())
            .replace("{grammar_instructions}", &grammar_instructions)
            .replace("{topic}", request.topic);

        Ok(json!({
            "model": model,
            "store": false,
            "instructions": request.instructions,
            "input": input,
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "sentence_pairs",
                    "strict": true,
                    "schema": {
                        "type": "object",
                        "properties": {
                            "pairs": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "german": {
                                            "type": "string",
                                            "description": "A sentence written only in German."
                                        },
                                        "english": {
                                            "type": "string",
                                            "description": "An equivalent sentence written only in English."
                                        },
                                        "level": { "type": "string", "enum": [level] },
                                        "grammar": { "type": "string", "enum": grammar_names },
                                        "topic": { "type": "string" }
                                    },
                                    "required": ["german", "english", "level", "grammar", "topic"],
                                    "additionalProperties": false
                                }
                            }
                        },
                        "required": ["pairs"],
                        "additionalProperties": false
                    }
                }
            }
        }))
    }

    pub(super) fn parse_response(body: &str) -> Result<Vec<SentencePair>> {
        let response: ApiResponse =
            serde_json::from_str(body).context("OpenAI returned malformed response JSON")?;
        let mut output_text = None;

        for content in response.output.into_iter().flat_map(|item| item.content) {
            match content {
                ContentItem::OutputText { text } => output_text = Some(text),
                ContentItem::Refusal { refusal } => bail!("OpenAI refused the request: {refusal}"),
                ContentItem::Other => {}
            }
        }

        let generated: GeneratedPairs = serde_json::from_str(
            &output_text.context("OpenAI response did not contain structured output text")?,
        )
        .context("OpenAI returned malformed sentence-pair output")?;
        Ok(generated.pairs)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn request_distributes_compatible_grammar_forms() {
            let body = build_request(
                "test-model",
                GenerationRequest {
                    count: 8,
                    level: Level::A1,
                    grammar: GrammarSelection::All,
                    topic: "travel",
                    max_words: 10,
                    input_template: "Count {count}; level {level}; topic {topic}; max {max_words}.\n{grammar_instructions}",
                    instructions: "Test instructions.",
                },
            )
            .unwrap();
            let input = body["input"].as_str().unwrap();
            let grammar = body["text"]["format"]["schema"]["properties"]["pairs"]["items"]
                ["properties"]["grammar"]["enum"]
                .as_array()
                .unwrap();

            assert_eq!(body["model"], "test-model");
            assert_eq!(grammar.len(), 7);
            assert!(input.contains("present-statement: exactly 2 pair(s)"));
            assert!(input.contains("negation: exactly 1 pair(s)"));
            assert!(input.contains("topic travel"));
            assert_eq!(body["instructions"], "Test instructions.");
        }

        #[test]
        fn request_rejects_grammar_above_level() {
            let result = build_request(
                "test-model",
                GenerationRequest {
                    count: 1,
                    level: Level::A1,
                    grammar: GrammarSelection::Form(crate::sentence::GrammarForm::RelativeClause),
                    topic: "varied",
                    max_words: 10,
                    input_template: "{grammar_instructions}",
                    instructions: "Test instructions.",
                },
            );

            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("requires level B1")
            );
        }

        #[test]
        fn response_parses_structured_sentence_pairs() {
            let body = r#"{
                "output": [{
                    "content": [{
                        "type": "output_text",
                        "text": "{\"pairs\":[{\"german\":\"Ich komme morgen.\",\"english\":\"I am coming tomorrow.\",\"level\":\"A1\",\"grammar\":\"present-statement\",\"topic\":\"plans\"}]}"
                    }]
                }]
            }"#;

            let pairs = parse_response(body).unwrap();

            assert_eq!(pairs[0].german, "Ich komme morgen.");
            assert_eq!(
                pairs[0].grammar,
                crate::sentence::GrammarForm::PresentStatement
            );
        }

        #[test]
        fn response_reports_refusal_and_allows_validation_to_check_count() {
            let refusal = r#"{"output":[{"content":[{"type":"refusal","refusal":"No."}]}]}"#;
            let empty =
                r#"{"output":[{"content":[{"type":"output_text","text":"{\"pairs\":[]}"}]}]}"#;

            assert!(
                parse_response(refusal)
                    .unwrap_err()
                    .to_string()
                    .contains("refused")
            );
            assert!(parse_response(empty).unwrap().is_empty());
        }
    }
}
