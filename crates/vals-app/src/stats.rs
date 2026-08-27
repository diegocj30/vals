//! Medicion de frame time.
//!
//! Existe desde el primer hito a proposito: optimizar dentro de seis meses
//! solo funciona si hay contra que comparar. Lo que importa aqui es el **p99**,
//! no la media: un juego a 60 fps de media con tirones se percibe peor que uno
//! a 55 estables.

const SAMPLES: usize = 240;
/// Cada cuantos frames se reordena el buffer para recalcular percentiles.
const RECOMPUTE_EVERY: u32 = 15;
/// Peso del valor nuevo en las medias moviles. Bajo = lectura estable.
const EMA_ALPHA: f32 = 0.1;

pub struct FrameStats {
    frames_ms: [f32; SAMPLES],
    idx: usize,
    filled: usize,
    /// Buffer reutilizado para ordenar. Preasignado: cero allocations por frame.
    scratch: Vec<f32>,
    recompute_in: u32,
    p50_ms: f32,
    p99_ms: f32,
    sim_ms: f32,
    render_ms: f32,
}

impl FrameStats {
    pub fn new() -> Self {
        Self {
            frames_ms: [0.0; SAMPLES],
            idx: 0,
            filled: 0,
            scratch: Vec::with_capacity(SAMPLES),
            recompute_in: 0,
            p50_ms: 0.0,
            p99_ms: 0.0,
            sim_ms: 0.0,
            render_ms: 0.0,
        }
    }

    pub fn push_frame(&mut self, dt_secs: f32) {
        self.frames_ms[self.idx] = dt_secs * 1000.0;
        self.idx = (self.idx + 1) % SAMPLES;
        self.filled = (self.filled + 1).min(SAMPLES);

        if self.recompute_in == 0 {
            self.recompute_percentiles();
            self.recompute_in = RECOMPUTE_EVERY;
        }
        self.recompute_in -= 1;
    }

    pub fn push_sim(&mut self, secs: f32) {
        self.sim_ms += (secs * 1000.0 - self.sim_ms) * EMA_ALPHA;
    }

    pub fn push_render(&mut self, secs: f32) {
        self.render_ms += (secs * 1000.0 - self.render_ms) * EMA_ALPHA;
    }

    fn recompute_percentiles(&mut self) {
        if self.filled == 0 {
            return;
        }
        self.scratch.clear();
        self.scratch
            .extend_from_slice(&self.frames_ms[..self.filled]);
        self.scratch.sort_unstable_by(f32::total_cmp);
        self.p50_ms = self.scratch[self.filled / 2];
        self.p99_ms = self.scratch[(self.filled * 99 / 100).min(self.filled - 1)];
    }

    pub fn p50_ms(&self) -> f32 {
        self.p50_ms
    }

    /// El numero que de verdad describe la fluidez percibida.
    pub fn p99_ms(&self) -> f32 {
        self.p99_ms
    }

    pub fn sim_ms(&self) -> f32 {
        self.sim_ms
    }

    /// Tiempo de CPU emitiendo draw calls.
    ///
    /// Ojo: no incluye el trabajo de la GPU, porque macroquad no lo entrega
    /// hasta `next_frame()`. Para el coste real de GPU hay que mirar el frame
    /// time total.
    pub fn render_ms(&self) -> f32 {
        self.render_ms
    }

    pub fn fps(&self) -> f32 {
        if self.p50_ms > 0.0 {
            1000.0 / self.p50_ms
        } else {
            0.0
        }
    }
}
