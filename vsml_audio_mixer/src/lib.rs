use dasp::{Signal, interpolate::sinc::Sinc, ring_buffer, signal, slice::add_in_place};
use log::{info, debug};
use std::time::Instant;
use vsml_common_audio::Audio as VsmlAudio;
use vsml_core::AudioEffectStyle;

pub struct MixerImpl {
    audio: VsmlAudio,
}

pub struct MixingContextImpl {}

impl vsml_core::Mixer for MixerImpl {
    type Audio = VsmlAudio;

    fn mix_audio(&mut self, audio: Self::Audio, offset_time: f64, duration: f64) {
        let mix_start = Instant::now();
        info!("ミキシング開始: オフセット={:.2}s, 継続時間={:.2}s, 入力サンプル数={}",
            offset_time, duration, audio.samples.len());

        // サンプルレートが同じ場合はリサンプリングをスキップ
        let resampled_samples = if audio.sampling_rate == self.audio.sampling_rate {
            info!("サンプルレートが同じためリサンプリングをスキップ");
            audio.samples
        } else {
            info!("リサンプリング開始: {}Hz → {}Hz", audio.sampling_rate, self.audio.sampling_rate);
            let resample_start = Instant::now();

            let signal = signal::from_iter(audio.samples);
            let ring_buffer = ring_buffer::Fixed::from([[0.0, 0.0]; 100]);
            let sinc = Sinc::new(ring_buffer);

            let new_signal = signal.from_hz_to_hz(
                sinc,
                audio.sampling_rate as f64,
                self.audio.sampling_rate as f64,
            );
            let resampled: Vec<_> = new_signal.until_exhausted().collect();

            info!("リサンプリング完了: {:?}, 出力サンプル数: {}",
                resample_start.elapsed(), resampled.len());
            resampled
        };

        let sampling_rate = self.audio.sampling_rate as f64;
        let offset_sample = (offset_time * sampling_rate) as usize;
        let duration_sample = (duration * sampling_rate) as usize;

        if offset_sample + duration_sample + 1 > self.audio.samples.len() {
            self.audio
                .samples
                .resize(offset_sample + duration_sample + 1, [0.0, 0.0]);
        }

        let end_sample = (offset_sample + duration_sample).min(
            offset_sample + resampled_samples.len()
        );
        let copy_length = end_sample - offset_sample;

        if copy_length > 0 && copy_length <= resampled_samples.len() {
            add_in_place(
                &mut self.audio.samples[offset_sample..offset_sample + copy_length],
                &resampled_samples[..copy_length],
            );
        }

        info!("ミキシング完了: {:?}", mix_start.elapsed());
    }

    fn mix(mut self, duration: f64) -> Self::Audio {
        let final_mix_start = Instant::now();
        info!("最終ミキシング開始: 継続時間={:.2}s", duration);

        self.audio.samples.resize(
            (duration * self.audio.sampling_rate as f64) as usize,
            [0.0, 0.0],
        );

        info!("最終ミキシング完了: {:?}", final_mix_start.elapsed());
        self.audio
    }
}

impl MixingContextImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MixingContextImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl vsml_core::MixingContext for MixingContextImpl {
    type Audio = VsmlAudio;
    type Mixer = MixerImpl;

    fn create_mixer(&mut self, sampling_rate: u32) -> Self::Mixer {
        info!("ミキサー作成: サンプリングレート={}Hz", sampling_rate);
        MixerImpl {
            audio: VsmlAudio {
                samples: Vec::new(),
                sampling_rate,
            },
        }
    }

    fn apply_style(&mut self, audio: Self::Audio, _style: AudioEffectStyle) -> Self::Audio {
        audio
    }
}
