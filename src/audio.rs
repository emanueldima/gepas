use {
    anyhow::{Context, Result, bail},
    clap::ValueEnum,
    shine_rs::{Mp3EncoderConfig, StereoMode, encode_pcm_to_mp3},
    std::{fs, path::Path},
};

pub const SAMPLE_RATE: u32 = 24_000;
const MP3_BITRATE: u32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum AudioFormat {
    Wav,
    Mp3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum PlaybackOrder {
    GermanFirst,
    EnglishFirst,
}

pub fn assemble(
    clips_dir: &Path,
    output_dir: &Path,
    pair_count: usize,
    order: PlaybackOrder,
    pause_ms: u64,
    format: AudioFormat,
    output_name: &str,
) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    if pair_count == 0 {
        return Ok(());
    }

    let mut pcm = Vec::new();
    for index in 1..=pair_count {
        let german_path = clips_dir.join(format!("{index:04}-de.pcm"));
        let english_path = clips_dir.join(format!("{index:04}-en.pcm"));
        let german = fs::read(&german_path)
            .with_context(|| format!("failed to read {}", german_path.display()))?;
        let english = fs::read(&english_path)
            .with_context(|| format!("failed to read {}", english_path.display()))?;
        pcm.extend(assemble_pcm(&german, &english, order, pause_ms)?);
        if index < pair_count {
            pcm.resize(pcm.len() + pause_bytes(pause_ms)?, 0);
        }
    }

    let output_path = match format {
        AudioFormat::Wav => {
            let path = output_dir.join(format!("{output_name}.wav"));
            write_wav(&path, &pcm)?;
            path
        }
        AudioFormat::Mp3 => {
            let path = output_dir.join(format!("{output_name}.mp3"));
            fs::write(&path, encode_mp3(&pcm)?)
                .with_context(|| format!("failed to write {}", path.display()))?;
            path
        }
    };
    println!("Wrote {}", output_path.display());
    Ok(())
}

pub fn assemble_pcm(
    german: &[u8],
    english: &[u8],
    order: PlaybackOrder,
    pause_ms: u64,
) -> Result<Vec<u8>> {
    if !german.len().is_multiple_of(2) || !english.len().is_multiple_of(2) {
        bail!("PCM clips must contain complete 16-bit samples");
    }

    let (first, second) = match order {
        PlaybackOrder::GermanFirst => (german, english),
        PlaybackOrder::EnglishFirst => (english, german),
    };
    let silence_bytes = pause_bytes(pause_ms)?;
    let mut output = Vec::with_capacity(first.len() + silence_bytes + second.len());
    output.extend_from_slice(first);
    output.resize(output.len() + silence_bytes, 0);
    output.extend_from_slice(second);
    Ok(output)
}

fn pause_bytes(pause_ms: u64) -> Result<usize> {
    let samples = SAMPLE_RATE as u64 * pause_ms / 1_000;
    usize::try_from(samples)?
        .checked_mul(2)
        .context("pause is too long")
}

fn write_wav(path: &Path, pcm: &[u8]) -> Result<()> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .with_context(|| format!("failed to create {}", path.display()))?;
    for sample in pcm.chunks_exact(2) {
        writer
            .write_sample(i16::from_le_bytes([sample[0], sample[1]]))
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    writer
        .finalize()
        .with_context(|| format!("failed to finalize {}", path.display()))?;
    Ok(())
}

fn encode_mp3(pcm: &[u8]) -> Result<Vec<u8>> {
    if !pcm.len().is_multiple_of(2) {
        bail!("PCM data must contain complete 16-bit samples");
    }
    let samples = pcm
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
        .collect::<Vec<_>>();
    encode_pcm_to_mp3(
        Mp3EncoderConfig::new()
            .sample_rate(SAMPLE_RATE)
            .bitrate(MP3_BITRATE)
            .channels(1)
            .stereo_mode(StereoMode::Mono),
        &samples,
    )
    .map_err(|error| anyhow::anyhow!("failed to encode MP3: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_assembly_respects_order_and_pause() {
        let german = [1, 0, 2, 0];
        let english = [3, 0];
        let german_first = assemble_pcm(&german, &english, PlaybackOrder::GermanFirst, 1).unwrap();
        let english_first =
            assemble_pcm(&german, &english, PlaybackOrder::EnglishFirst, 1).unwrap();

        assert_eq!(&german_first[..4], &german);
        assert_eq!(&german_first[german_first.len() - 2..], &english);
        assert!(
            german_first[4..german_first.len() - 2]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(&english_first[..2], &english);
        assert_eq!(&english_first[english_first.len() - 4..], &german);
        assert_eq!(german_first.len(), 4 + 48 + 2);
    }

    #[test]
    fn wav_has_expected_audio_format_and_samples() {
        let path = std::env::temp_dir().join(format!("gepas-audio-{}.wav", std::process::id()));
        write_wav(&path, &[1, 0, 255, 255]).unwrap();
        let mut reader = hound::WavReader::open(&path).unwrap();

        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, SAMPLE_RATE);
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert_eq!(
            reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            [1, -1]
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn assembly_reports_missing_clips() {
        let root = std::env::temp_dir().join(format!("gepas-missing-{}", std::process::id()));
        let result = assemble(
            &root.join("clips"),
            &root.join("output"),
            1,
            PlaybackOrder::GermanFirst,
            0,
            AudioFormat::Wav,
            "lesson",
        );

        assert!(result.unwrap_err().to_string().contains("failed to read"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn assembly_writes_all_pairs_to_one_file() {
        let root = std::env::temp_dir().join(format!("gepas-combined-{}", std::process::id()));
        let clips = root.join("clips");
        let output = root.join("output");
        fs::create_dir_all(&clips).unwrap();
        for (name, sample) in [
            ("0001-de.pcm", 1_i16),
            ("0001-en.pcm", 2),
            ("0002-de.pcm", 3),
            ("0002-en.pcm", 4),
        ] {
            fs::write(clips.join(name), sample.to_le_bytes()).unwrap();
        }

        assemble(
            &clips,
            &output,
            2,
            PlaybackOrder::GermanFirst,
            0,
            AudioFormat::Wav,
            "lesson",
        )
        .unwrap();
        let mut reader = hound::WavReader::open(output.join("lesson.wav")).unwrap();

        assert_eq!(
            reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            [1, 2, 3, 4]
        );
        assert!(!output.join("0001.wav").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mp3_encoder_accepts_our_pcm_format() {
        let pcm = vec![0; SAMPLE_RATE as usize / 10 * 2];
        let mp3 = encode_mp3(&pcm).unwrap();

        assert!(!mp3.is_empty());
        assert_eq!(mp3[0], 0xff);
        assert_eq!(mp3[1] & 0xe0, 0xe0);
    }

    #[test]
    fn external_probe_accepts_encoded_mp3() {
        if std::process::Command::new("ffprobe")
            .arg("-version")
            .output()
            .is_err()
        {
            return;
        }

        let path = std::env::temp_dir().join(format!("gepas-audio-{}.mp3", std::process::id()));
        fs::write(
            &path,
            encode_mp3(&vec![0; SAMPLE_RATE as usize / 10 * 2]).unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=sample_rate,channels",
                "-of",
                "csv=p=0",
            ])
            .arg(&path)
            .output()
            .unwrap();

        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "24000,1");
        fs::remove_file(path).unwrap();
    }
}
