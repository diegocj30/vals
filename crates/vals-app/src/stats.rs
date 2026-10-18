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

/// Por encima de esto un frame cuenta como tiron, en ms. A 60 Hz un frame dura
/// 16,7 y a 144 Hz 6,9: pasar de 20 ya es perderse uno.
const UMBRAL_TIRON_MS: f64 = 20.0;

/// El registro de tirones, solo con `--pruebas`.
///
/// El p99 dice que hay tirones; esto dice **cuales**. Cada frame de mas de
/// `UMBRAL_TIRON_MS` sale por la salida de error con su escena, lo que paso en
/// el y en que se fue el tiempo: simulacion, musica, dibujo, el resto del
/// bucle y lo de fuera, que es `next_frame` (las corrutinas, el envio a la GPU
/// y la espera del vsync). Apagado no mide nada.
pub struct Tirones {
    activo: bool,
    actual: Medida,
    /// El frame anterior y cuando acabo: cuanto duro solo se sabe al empezar
    /// el siguiente.
    anterior: Option<(f64, Medida)>,
    /// Lo que paso en el frame. Se reutiliza: cero reservas por frame.
    sucesos: Vec<&'static str>,
}

/// Los tramos de un frame, en segundos.
#[derive(Clone, Copy, Default)]
struct Medida {
    inicio: f64,
    sim: f64,
    musica: f64,
    dibujo: f64,
    pasos: u32,
    escena: &'static str,
}

impl Tirones {
    pub fn new(activo: bool) -> Self {
        Self {
            activo,
            actual: Medida::default(),
            anterior: None,
            sucesos: Vec::with_capacity(16),
        }
    }

    /// Al empezar un frame, con el reloj de `get_time`. Si el anterior fue un
    /// tiron, lo cuenta.
    pub fn empezar(&mut self, ahora: f64) {
        if !self.activo {
            return;
        }
        if let Some((fin, m)) = self.anterior.take() {
            let ms = |s: f64| s * 1000.0;
            let total = ms(ahora - m.inicio);
            if total > UMBRAL_TIRON_MS {
                let resto = ms(fin - m.inicio - m.sim - m.musica - m.dibujo);
                eprintln!(
                    "[tiron] {total:5.1} ms  {:<8} pasos {}  sim {:.2}  musica {:.2}  \
                     dibujo {:.2}  resto {resto:.2}  fuera {:.2}  | {}",
                    m.escena,
                    m.pasos,
                    ms(m.sim),
                    ms(m.musica),
                    ms(m.dibujo),
                    ms(ahora - fin),
                    self.sucesos.join(", "),
                );
            }
        }
        self.sucesos.clear();
        self.actual = Medida {
            inicio: ahora,
            ..Medida::default()
        };
    }

    /// Algo que ha pasado en este frame.
    pub fn suceso(&mut self, que: &'static str) {
        if self.activo && !self.sucesos.contains(&que) {
            self.sucesos.push(que);
        }
    }

    pub fn sim(&mut self, segundos: f64, pasos: u32) {
        self.actual.sim = segundos;
        self.actual.pasos = pasos;
    }

    pub fn musica(&mut self, segundos: f64) {
        self.actual.musica = segundos;
    }

    pub fn dibujo(&mut self, segundos: f64) {
        self.actual.dibujo = segundos;
    }

    /// Al acabar el frame, justo antes de `next_frame`.
    pub fn acabar(&mut self, escena: &'static str, ahora: f64) {
        if self.activo {
            self.actual.escena = escena;
            self.anterior = Some((ahora, self.actual));
        }
    }
}
