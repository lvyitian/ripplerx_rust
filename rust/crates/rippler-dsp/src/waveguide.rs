//! The delay line behind the tube models.
//!
//! Port of `src/dsp/Waveguide.{h,cpp}`.

/// The length of the tube, in samples.
///
/// The C++ uses 20000, which allows for 10 Hz at a 200 kHz sample rate.
const TUBE_LEN: usize = 20000;

/// A waveguide delay line, used by the open and closed tube models.
#[derive(Clone, Debug)]
pub struct Waveguide {
    /// The tube's base frequency.
    pub base_freq: f64,
    /// Whether the tube is closed, which halves its length and inverts it.
    pub is_closed: bool,
    /// The sample rate.
    pub srate: f64,
    /// The decay time.
    pub decay: f64,
    /// The tube radius, which controls the damping.
    pub radius: f64,
    /// The release multiplier.
    pub rel: f64,
    /// How much velocity shifts the decay.
    pub vel_decay: f64,
    /// The tube's current frequency.
    pub f_k: f64,

    read_ptr_frac: f64,
    write_ptr: usize,
    tube_decay: f64,
    tube: Box<[f64]>,
    y: f64,
    y1: f64,
}

impl Waveguide {
    /// Creates a waveguide with an empty tube.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            base_freq: 1000.0,
            is_closed: false,
            srate: 0.0,
            decay: 0.0,
            radius: 0.0,
            rel: 0.0,
            vel_decay: 0.0,
            f_k: 0.0,
            read_ptr_frac: 0.0,
            write_ptr: 0,
            tube_decay: 0.0,
            tube: Box::new([0.0; TUBE_LEN]),
            y: 0.0,
            y1: 0.0,
        }
    }

    /// Retunes the tube and recomputes its decay.
    pub fn update(&mut self, f_0: f64, vel: f64, pitch_bend: f64, is_release: bool) {
        self.base_freq = f_0;
        self.f_k = self.base_freq * pitch_bend;

        let mut tlen = self.srate / self.f_k;
        if self.is_closed {
            tlen *= 0.5; // fix closed tube one octave lower
        }
        self.read_ptr_frac = self.write_ptr as f64 - tlen;
        if self.read_ptr_frac < 0.0 {
            self.read_ptr_frac += TUBE_LEN as f64;
        }

        let mut decay_k = (self.decay.ln() + vel * self.vel_decay * (100.0f64.ln() - 0.01f64.ln()))
            .exp()
            .min(100.0);
        if is_release {
            decay_k *= self.rel;
        }
        self.tube_decay = if decay_k != 0.0 {
            // 125000 set by hear so that decay approximates in seconds
            (-std::f64::consts::PI / self.base_freq / (self.srate * decay_k / 125000.0)).exp()
        } else {
            0.0
        };
    }

    /// Retunes the tube for a pitch bend, leaving its decay alone.
    pub fn apply_pitch_bend(&mut self, pitch_bend: f64) {
        self.f_k = self.base_freq * pitch_bend;

        let mut tlen = self.srate / self.f_k;
        if self.is_closed {
            tlen *= 0.5; // fix closed tube one octave lower
        }

        // Compute fractional read position
        self.read_ptr_frac = self.write_ptr as f64 - tlen;
        if self.read_ptr_frac < 0.0 {
            self.read_ptr_frac += TUBE_LEN as f64;
        }
    }

    /// Processes one sample and returns the tube's output.
    pub fn process(&mut self, input: f64) -> f64 {
        // The C++ reads `tube[(int)read_ptr_frac]`. When the tube is longer
        // than the delay line, `read_ptr_frac` is still negative after the
        // single `+= tube_len` correction above, so the C++ reads out of
        // bounds, which is undefined behaviour. The index is wrapped here
        // instead, so the waveguide stays finite rather than reading
        // whatever happens to be before the buffer. This is the one
        // deliberate difference from the C++.
        let raw = self.read_ptr_frac as i64;
        let i0 = raw.rem_euclid(TUBE_LEN as i64) as usize;
        let i1 = (i0 + 1) % TUBE_LEN;
        let frac = self.read_ptr_frac - raw as f64;
        let sample = self.tube[i0] * (1.0 - frac) + self.tube[i1] * frac;

        // Apply lowpass filter for frequency damping (tube radius)
        self.y = self.radius * sample + (1.0 - self.radius) * self.y1;
        self.y1 = self.y;

        // Apply decay to sample
        let mut dsample = self.y * self.tube_decay;
        if self.is_closed {
            dsample *= -1.0; // closed tube: only odd harmonics
        }
        self.tube[self.write_ptr] = input + dsample;

        // Increment pointers
        self.write_ptr = (self.write_ptr + 1) % TUBE_LEN;
        self.read_ptr_frac += 1.0;
        if self.read_ptr_frac >= TUBE_LEN as f64 {
            self.read_ptr_frac -= TUBE_LEN as f64;
        }

        dsample
    }

    /// Clears the tube and its history.
    pub fn clear(&mut self) {
        self.y = 0.0;
        self.y1 = 0.0;
        self.write_ptr = 0;
        self.read_ptr_frac = 0.0;
        self.tube.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRATE: f64 = 48_000.0;

    /// A waveguide tuned to `f_k`, with a radius that lets the tube sound.
    fn tuned(f_k: f64, radius: f64) -> Waveguide {
        let mut waveguide = Waveguide::new();
        waveguide.srate = SRATE;
        waveguide.decay = 1.0;
        waveguide.radius = radius;
        waveguide.update(f_k, 0.5, 1.0, false);
        waveguide
    }

    #[test]
    fn a_new_waveguide_has_the_cpp_defaults() {
        let waveguide = Waveguide::new();

        assert_eq!(waveguide.base_freq, 1000.0);
        assert!(!waveguide.is_closed);
        assert_eq!(waveguide.srate, 0.0);
        assert_eq!(waveguide.decay, 0.0);
        assert_eq!(waveguide.radius, 0.0);
        assert_eq!(waveguide.rel, 0.0);
        assert_eq!(waveguide.vel_decay, 0.0);
        assert_eq!(waveguide.f_k, 0.0);

        assert_eq!(waveguide.read_ptr_frac, 0.0);
        assert_eq!(waveguide.write_ptr, 0);
        assert_eq!(waveguide.tube_decay, 0.0);
        assert_eq!(waveguide.tube.len(), TUBE_LEN);
        assert!(waveguide.tube.iter().all(|sample| *sample == 0.0));
        assert_eq!(waveguide.y, 0.0);
        assert_eq!(waveguide.y1, 0.0);
    }

    #[test]
    fn an_impulse_returns_after_the_delay() {
        let mut waveguide = tuned(1000.0, 0.5);

        // A 1 kHz tube at 48 kHz is 48 samples long.
        let delay = (SRATE / 1000.0) as usize;

        // The impulse is written into the tube on the first sample.
        assert_eq!(waveguide.process(1.0), 0.0);

        for _ in 1..delay {
            assert_eq!(waveguide.process(0.0), 0.0, "the tube returned too early");
        }
        let returned = waveguide.process(0.0);
        assert!(returned > 0.0, "the impulse did not come back");
    }

    #[test]
    fn a_closed_tube_inverts_the_returned_signal() {
        let mut open = tuned(1000.0, 0.5);
        let mut closed = tuned(1000.0, 0.5);
        closed.is_closed = true;
        closed.update(1000.0, 0.5, 1.0, false);

        open.process(1.0);
        closed.process(1.0);

        // The open tube returns the impulse after 48 samples.
        for _ in 0..47 {
            assert_eq!(open.process(0.0), 0.0);
        }
        let open_output = open.process(0.0);

        // The closed tube is half as long, so it returns after 24 samples,
        // and it inverts what it returns.
        for _ in 0..23 {
            assert_eq!(closed.process(0.0), 0.0);
        }
        let closed_output = closed.process(0.0);

        assert!(open_output > 0.0, "the open tube did not sound");
        assert!(closed_output < 0.0, "the closed tube was not inverted");
    }

    #[test]
    fn the_radius_damps_the_high_frequencies() {
        // The radius is the feed-forward coefficient of the one-pole lowpass
        // in the tube's feedback path, `y = radius * sample + (1 - radius) *
        // y1`. A smaller radius smooths the signal more, so it damps the high
        // frequencies harder; a larger radius lets more of them through. The
        // C++ behaves this way and the port reproduces it.
        fn alternating_tube(radius: f64) -> Waveguide {
            let mut waveguide = tuned(12_000.0, radius);
            for (i, sample) in waveguide.tube.iter_mut().enumerate() {
                *sample = if i % 2 == 0 { 1.0 } else { -1.0 };
            }
            waveguide
        }

        fn peak(radius: f64) -> f64 {
            let mut waveguide = alternating_tube(radius);
            let mut peak: f64 = 0.0;
            for _ in 0..4 {
                let output = waveguide.process(0.0);
                peak = peak.max(output.abs());
            }
            peak
        }

        let narrow = peak(0.1);
        let wide = peak(0.9);

        assert!(narrow > 0.0, "the narrow tube was silent");
        assert!(
            narrow < wide,
            "the narrow tube was {narrow} and the wide tube was {wide}"
        );
    }

    #[test]
    fn the_waveguide_survives_a_frequency_below_the_tube_length() {
        // A 1 Hz tube at 48 kHz needs 48000 samples of delay, which is more
        // than the tube holds. The C++ reads out of bounds here; the port
        // wraps the index instead.
        let mut waveguide = tuned(1.0, 0.5);

        for _ in 0..1000 {
            let output = waveguide.process(0.5);
            assert!(output.is_finite(), "the waveguide produced {output}");
        }
    }

    #[test]
    fn clearing_resets_the_tube() {
        let mut waveguide = tuned(1000.0, 0.5);

        let mut loudest: f64 = 0.0;
        for n in 0..200 {
            let output =
                waveguide.process((std::f64::consts::TAU * 1000.0 * n as f64 / SRATE).sin());
            loudest = loudest.max(output.abs());
        }
        assert!(loudest > 0.0, "the tube never sounded");

        waveguide.clear();

        assert_eq!(waveguide.process(0.0), 0.0);
        assert!(waveguide.tube.iter().all(|sample| *sample == 0.0));
    }
}
