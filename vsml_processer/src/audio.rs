use log::{info, debug};
use std::collections::HashMap;
use std::time::Instant;
use vsml_common_audio::Audio as VsmlAudio;
use vsml_core::schemas::{ObjectProcessor, RectSize};

pub struct AudioProcessor;

impl<I> ObjectProcessor<I, VsmlAudio> for AudioProcessor {
    fn name(&self) -> &str {
        "audio"
    }

    fn default_duration(&self, attributes: &HashMap<String, String>) -> f64 {
        let src_path = attributes.get("src").unwrap();
        let reader = hound::WavReader::open(src_path).unwrap();
        reader.duration() as f64 / reader.spec().sample_rate as f64
    }

    fn default_image_size(&self, _attributes: &HashMap<String, String>) -> RectSize {
        RectSize::ZERO
    }

    fn process_image(
        &self,
        _: f64,
        _attributes: &HashMap<String, String>,
        _: Option<I>,
    ) -> Option<I> {
        None
    }

    fn process_audio(
        &self,
        attributes: &HashMap<String, String>,
        _audio: Option<VsmlAudio>,
    ) -> Option<VsmlAudio> {
        let audio_load_start = Instant::now();
        let src_path = attributes.get("src").unwrap();
        info!("音声ファイル読み込み開始: {}", src_path);

        let mut reader = hound::WavReader::open(src_path).unwrap();
        let spec = reader.spec();
        info!("音声ファイル読み込み完了: {:?}, サンプルレート: {}, チャンネル: {}, 全サンプル数: {}",
              audio_load_start.elapsed(), spec.sample_rate, spec.channels, reader.duration());

        // attributesから時間情報を取得
        let effective_duration = attributes
            .get("_effective_duration")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(10.0); // デフォルトは10秒（互換性のため）
        
        info!("音声読み込み: 実効継続時間={:.2}秒", effective_duration);
        
        // 実際に必要な長さだけ読み込む（少し余裕を持たせて+1秒）
        let max_samples = ((effective_duration + 1.0) * spec.sample_rate as f64 * spec.channels as f64) as u32;
        let samples_to_read = reader.duration().min(max_samples);

        info!("読み込み予定サンプル数: {} (元: {})", samples_to_read, reader.duration());

        let sample_process_start = Instant::now();
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .take(samples_to_read as usize)  // 必要な分だけ読み込み
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            hound::SampleFormat::Int => reader
                .samples::<i32>()
                .take(samples_to_read as usize)  // 必要な分だけ読み込み
                .map(|s| (s.unwrap() as f64 / (1i64 << (spec.bits_per_sample - 1)) as f64) as f32)
                .collect(),
        };

        let samples = samples
            .chunks(spec.channels as usize)
            .map(|chunk| match chunk {
                &[left, right, ..] => [left, right],
                &[mono] => [mono, mono],
                [] => unreachable!("channels must be greater than 0"),
            })
            .collect::<Vec<[f32; 2]>>();

        info!("音声サンプル処理完了: {:?}, 実際のサンプル数: {}",
              sample_process_start.elapsed(), samples.len());
        info!("音声ファイル処理総時間: {:?}", audio_load_start.elapsed());

        Some(VsmlAudio {
            samples,
            sampling_rate: spec.sample_rate,
        })
    }
}
