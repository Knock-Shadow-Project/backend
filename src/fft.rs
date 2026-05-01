//! Procesador FFT con ventana deslizante.
//!
//! La ventana y el paso se configuran en segundos junto con la frecuencia de
//! muestreo. El numero de muestras necesarias se deriva automaticamente:
//!
//!   window_samples = (window_secs * sample_rate_hz).round()
//!   step_samples   = (step_secs   * sample_rate_hz).round()
//!
//! Bins utiles: indices 0..window_samples/2, con resolucion Fs/window_samples Hz/bin.

use rustfft::{FftPlanner, num_complex::Complex};
use std::collections::VecDeque;

/// Resultado de una FFT sobre la ultima ventana de muestras.
/// Cada vec contiene `window_samples / 2` magnitudes.
pub struct FftSnapshot {
    pub magnitudes_x: Vec<f32>,
    pub magnitudes_y: Vec<f32>,
    pub magnitudes_z: Vec<f32>,
}

/// Procesador con ventana deslizante configurado por tiempo.
pub struct FftProcessor {
    window: VecDeque<(f32, f32, f32)>,
    window_samples: usize,
    step_samples: usize,
    samples_since_last_fft: usize,
    planner: FftPlanner<f32>,
    hann: Vec<f32>,
}

impl FftProcessor {
    /// Crea un procesador FFT configurado por tiempo.
    ///
    /// # Parametros
    /// - `sample_rate_hz`: frecuencia de muestreo del sensor (ej. 60.0)
    /// - `window_secs`:    duracion de la ventana de analisis (ej. 1.0)
    /// - `step_secs`:      intervalo entre FFTs consecutivas (ej. 0.5)
    pub fn new(sample_rate_hz: f32, window_secs: f32, step_secs: f32) -> Self {
        let window_samples = (window_secs * sample_rate_hz).round() as usize;
        let step_samples = (step_secs * sample_rate_hz).round() as usize;

        assert!(window_samples >= 2, "window must cover at least 2 samples");
        assert!(step_samples >= 1, "step must cover at least 1 sample");

        let hann = (0..window_samples)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (window_samples - 1) as f32).cos())
            })
            .collect();

        Self {
            window: VecDeque::with_capacity(window_samples),
            window_samples,
            step_samples,
            samples_since_last_fft: 0,
            planner: FftPlanner::new(),
            hann,
        }
    }

    /// Introduce una muestra (x, y, z).
    /// Devuelve `Some(FftSnapshot)` cada `step_secs` una vez la ventana esta llena.
    pub fn push(&mut self, x: f32, y: f32, z: f32) -> Option<FftSnapshot> {
        if self.window.len() == self.window_samples {
            self.window.pop_front();
        }
        self.window.push_back((x, y, z));
        self.samples_since_last_fft += 1;

        if self.window.len() == self.window_samples
            && self.samples_since_last_fft >= self.step_samples
        {
            self.samples_since_last_fft = 0;
            Some(self.compute())
        } else {
            None
        }
    }

    fn compute(&mut self) -> FftSnapshot {
        let fft = self.planner.plan_fft_forward(self.window_samples);

        let mut buf_x: Vec<Complex<f32>> = self
            .window
            .iter()
            .enumerate()
            .map(|(i, &(x, _, _))| Complex {
                re: x * self.hann[i],
                im: 0.0,
            })
            .collect();
        let mut buf_y: Vec<Complex<f32>> = self
            .window
            .iter()
            .enumerate()
            .map(|(i, &(_, y, _))| Complex {
                re: y * self.hann[i],
                im: 0.0,
            })
            .collect();
        let mut buf_z: Vec<Complex<f32>> = self
            .window
            .iter()
            .enumerate()
            .map(|(i, &(_, _, z))| Complex {
                re: z * self.hann[i],
                im: 0.0,
            })
            .collect();

        fft.process(&mut buf_x);
        fft.process(&mut buf_y);
        fft.process(&mut buf_z);

        // Solo la mitad util del espectro.
        // Bin i → frecuencia = i * sample_rate_hz / window_samples Hz.
        let half = self.window_samples / 2;
        FftSnapshot {
            magnitudes_x: buf_x[..half].iter().map(|c| c.norm()).collect(),
            magnitudes_y: buf_y[..half].iter().map(|c| c.norm()).collect(),
            magnitudes_z: buf_z[..half].iter().map(|c| c.norm()).collect(),
        }
    }
}
