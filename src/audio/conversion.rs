//! Host-independent conversion into Whisper's mono 16-bit PCM contract.

use cpal::{FromSample, Sample};

pub(crate) fn downmix_frame<T>(frame: &[T]) -> Option<f32>
where
    T: Sample + Copy,
    f32: FromSample<T>,
{
    if frame.is_empty() {
        return None;
    }

    Some(
        frame
            .iter()
            .copied()
            .map(|sample| sample.to_sample::<f32>())
            .sum::<f32>()
            / frame.len() as f32,
    )
}

/// Streaming deterministic sample-rate conversion into signed 16-bit PCM.
///
/// The phase is retained between input callbacks, so splitting the same input
/// across callback boundaries produces the same output as one contiguous call.
pub(crate) struct MonoPcm16Resampler {
    input_rate: u64,
    output_rate: u64,
    phase: u64,
    anti_alias: Option<[Biquad; 3]>,
}

impl MonoPcm16Resampler {
    pub(crate) fn new(input_rate: u32, output_rate: u32) -> Result<Self, &'static str> {
        if input_rate == 0 || output_rate == 0 {
            return Err("sample rates must be non-zero");
        }

        Ok(Self {
            input_rate: u64::from(input_rate),
            output_rate: u64::from(output_rate),
            phase: 0,
            anti_alias: (input_rate > output_rate).then(|| {
                // A sixth-order Butterworth response materially suppresses
                // content above the 8 kHz Nyquist limit of 16 kHz output
                // without heap allocation or convolution in the callback.
                let cutoff = f64::from(output_rate) * 0.45;
                [
                    Biquad::low_pass(input_rate, cutoff, 0.517_638_090_205_041_5),
                    Biquad::low_pass(input_rate, cutoff, std::f64::consts::FRAC_1_SQRT_2),
                    Biquad::low_pass(input_rate, cutoff, 1.931_851_652_578_136_6),
                ]
            }),
        })
    }

    /// Push one mono floating-point sample and emit zero or more output samples.
    pub(crate) fn push_sample(&mut self, sample: f32, mut emit: impl FnMut(i16)) {
        self.phase += self.output_rate;
        let mut sample = if sample.is_finite() {
            f64::from(sample.clamp(-1.0, 1.0))
        } else {
            0.0
        };
        if let Some(stages) = &mut self.anti_alias {
            for stage in stages {
                sample = stage.process(sample);
            }
        }
        let sample = float_to_pcm16(sample as f32);

        while self.phase >= self.input_rate {
            self.phase -= self.input_rate;
            emit(sample);
        }
    }

    #[cfg(test)]
    fn process_interleaved(
        &mut self,
        samples: &[f32],
        channels: usize,
        mut emit: impl FnMut(i16),
    ) -> Result<(), &'static str> {
        if channels == 0 {
            return Err("channel count must be non-zero");
        }

        for frame in samples.chunks_exact(channels) {
            let mono = downmix_frame(frame).expect("chunks are non-empty");
            self.push_sample(mono, &mut emit);
        }

        Ok(())
    }
}

/// One fixed-coefficient Direct Form II transposed low-pass stage.
///
/// Coefficients are calculated once before capture. Processing requires only
/// two state values and performs no allocation, locking or system calls.
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn low_pass(sample_rate: u32, cutoff: f64, q: f64) -> Self {
        let omega = 2.0 * std::f64::consts::PI * cutoff / f64::from(sample_rate);
        let cosine = omega.cos();
        let alpha = omega.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;

        Self {
            b0: ((1.0 - cosine) / 2.0) / a0,
            b1: (1.0 - cosine) / a0,
            b2: ((1.0 - cosine) / 2.0) / a0,
            a1: (-2.0 * cosine) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, input: f64) -> f64 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
}

fn float_to_pcm16(sample: f32) -> i16 {
    let sample = if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    };

    if sample >= 0.0 {
        (sample * f32::from(i16::MAX)).round() as i16
    } else {
        (sample * -(f32::from(i16::MIN))).round() as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resample_sine(input_rate: u32, frequency: f64) -> Vec<i16> {
        let input = (0..input_rate)
            .map(|sample| {
                (2.0 * std::f64::consts::PI * frequency * f64::from(sample) / f64::from(input_rate))
                    .sin() as f32
            })
            .collect::<Vec<_>>();
        let mut converter = MonoPcm16Resampler::new(input_rate, 16_000).unwrap();
        let mut output = Vec::new();
        converter
            .process_interleaved(&input, 1, |sample| output.push(sample))
            .unwrap();
        output
    }

    fn normalised_rms(samples: &[i16]) -> f64 {
        let mean_square = samples
            .iter()
            .map(|sample| (f64::from(*sample) / f64::from(i16::MAX)).powi(2))
            .sum::<f64>()
            / samples.len() as f64;
        mean_square.sqrt()
    }

    #[test]
    fn downmixes_stereo_and_resamples_to_16khz() {
        let mut converter = MonoPcm16Resampler::new(48_000, 16_000).unwrap();
        let input = (0..300)
            .flat_map(|_| [0.75_f32, 0.25_f32])
            .collect::<Vec<_>>();
        let mut output = Vec::new();

        converter
            .process_interleaved(&input, 2, |sample| output.push(sample))
            .unwrap();

        assert_eq!(downmix_frame(&[0.75_f32, 0.25]), Some(0.5));
        assert_eq!(output.len(), 100);
        assert!((i32::from(*output.last().unwrap()) - 16_384).abs() <= 2);
    }

    #[test]
    fn downmix_supports_every_cpal_sample_format() {
        macro_rules! assert_extremes {
            ($minimum:expr, $maximum:expr) => {{
                let minimum = downmix_frame(&[$minimum]).unwrap();
                let maximum = downmix_frame(&[$maximum]).unwrap();
                assert!(minimum <= -0.9, "minimum converted to {minimum}");
                assert!(maximum >= 0.9, "maximum converted to {maximum}");
            }};
        }

        assert_extremes!(i8::MIN, i8::MAX);
        assert_extremes!(i16::MIN, i16::MAX);
        assert_extremes!(i32::MIN, i32::MAX);
        assert_extremes!(i64::MIN, i64::MAX);
        assert_extremes!(u8::MIN, u8::MAX);
        assert_extremes!(u16::MIN, u16::MAX);
        assert_extremes!(u32::MIN, u32::MAX);
        assert_extremes!(u64::MIN, u64::MAX);
        assert_extremes!(-1.0_f32, 1.0_f32);
        assert_extremes!(-1.0_f64, 1.0_f64);
        assert_eq!(downmix_frame::<f32>(&[]), None);
    }

    #[test]
    fn callback_boundaries_do_not_reset_resampling_phase() {
        let input = [0.1_f32, 0.2, 0.3, 0.4, 0.5, 0.6];
        let mut contiguous = MonoPcm16Resampler::new(48_000, 16_000).unwrap();
        let mut split = MonoPcm16Resampler::new(48_000, 16_000).unwrap();
        let mut contiguous_output = Vec::new();
        let mut split_output = Vec::new();

        contiguous
            .process_interleaved(&input, 1, |sample| contiguous_output.push(sample))
            .unwrap();
        split
            .process_interleaved(&input[..2], 1, |sample| split_output.push(sample))
            .unwrap();
        split
            .process_interleaved(&input[2..], 1, |sample| split_output.push(sample))
            .unwrap();

        assert_eq!(split_output, contiguous_output);
        assert_eq!(split_output.len(), 2);
    }

    #[test]
    fn downsampling_materially_attenuates_out_of_band_audio() {
        // At 48 kHz, 12 kHz would alias to 4 kHz after unfiltered 16 kHz
        // decimation. Ignore the bounded filter startup transient and require
        // more than 18 dB attenuation relative to a full-scale sine's RMS.
        let output = resample_sine(48_000, 12_000.0);

        assert_eq!(output.len(), 16_000);
        assert!(normalised_rms(&output[200..]) < 0.085);
    }

    #[test]
    fn arbitrary_rate_downsampling_preserves_voice_band_and_count() {
        let output = resample_sine(44_100, 1_000.0);
        let rms = normalised_rms(&output[200..]);

        assert_eq!(output.len(), 16_000);
        assert!((0.68..0.73).contains(&rms), "unexpected RMS {rms}");
    }

    #[test]
    fn upsampling_is_deterministic() {
        let mut converter = MonoPcm16Resampler::new(8_000, 16_000).unwrap();
        let mut output = Vec::new();

        converter
            .process_interleaved(&[0.5, -0.5], 1, |sample| output.push(sample))
            .unwrap();

        assert_eq!(output, vec![16_384, 16_384, -16_384, -16_384]);
    }

    #[test]
    fn clamps_extremes_and_silences_non_finite_samples() {
        assert_eq!(float_to_pcm16(1.5), i16::MAX);
        assert_eq!(float_to_pcm16(-1.5), i16::MIN);
        assert_eq!(float_to_pcm16(f32::NAN), 0);
        assert_eq!(float_to_pcm16(f32::INFINITY), 0);
    }

    #[test]
    fn rejects_invalid_rates_and_channel_counts() {
        assert!(MonoPcm16Resampler::new(0, 16_000).is_err());
        assert!(MonoPcm16Resampler::new(48_000, 0).is_err());

        let mut converter = MonoPcm16Resampler::new(48_000, 16_000).unwrap();
        assert!(converter.process_interleaved(&[0.0], 0, |_| {}).is_err());
    }
}
