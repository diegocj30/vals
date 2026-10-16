//! Audio **sintetizado en memoria**. Ni un fichero de sonido en el repo.
//!
//! Es la misma decision que con los graficos: en este juego no hay assets, hay
//! codigo que los genera. Aqui sale ademas especialmente a cuenta, porque
//! evita buscar sonidos CC0, comprobar licencias y cargar el repositorio con
//! binarios que luego nadie sabe de donde salieron.
//!
//! El camino es: un par de osciladores con envolvente producen muestras, las
//! muestras se empaquetan como un WAV en un `Vec<u8>`, y ese `Vec` se le pasa a
//! `macroquad::audio::load_sound_from_bytes` igual que si viniera de disco.
//!
//! Ojo con el WAV: el decodificador de macroquad **entra en panico** con una
//! cabecera mal formada, no devuelve error. Por eso el escritor esta cubierto
//! por tests.

use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
};
use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
use vals_core::Events;

use crate::music::{self, TEMAS, Tema};

/// 44.100 Hz para que el mezclador de macroquad no tenga que remuestrear.
const SAMPLE_RATE: u32 = 44_100;

/// La musica va a la mitad, y ocupa la mitad.
///
/// Con once temas, a 44,1 kHz eran unos 20 MB de memoria; a 22,05 son 10. Lo
/// unico que aliasa a esa frecuencia son los armonicos muy altos de los golpes
/// de acorde, que suenan a volumen 0,05: la melodia y el bajo son senos y no
/// tienen nada por encima de su fundamental. Era la palanca mas barata que
/// habia y estaba apuntada esperando a hacer falta.
const SAMPLE_RATE_MUSICA: u32 = 22_050;

/// Volumen general. Bajo: son efectos secos y muy repetidos.
const MASTER: f32 = 0.6;

/// La musica va por debajo de los efectos: es el suelo sobre el que pasan las
/// cosas, no una de las cosas que pasan.
const MASTER_MUSICA: f32 = 0.34;

#[derive(Clone, Copy)]
pub(crate) enum Wave {
    Sine,
    Square,
    Saw,
    Noise,
}

/// Un oscilador con barrido de frecuencia y envolvente.
///
/// Es la unidad de todo el audio del juego: un efecto son una o dos, y un tema
/// de musica son un par de cientos con distintos retardos.
#[derive(Clone, Copy)]
pub(crate) struct Voz {
    wave: Wave,
    /// Frecuencia inicial y final: el barrido es lo que da caracter.
    f0: f32,
    f1: f32,
    dur: f32,
    vol: f32,
    /// Exponente de la caida. 1 es lineal; mas alto, mas seco.
    decay: f32,
    /// Retardo desde el inicio del sonido. Sirve para encadenar notas.
    delay: f32,
}

impl Voz {
    /// Una nota: frecuencia fija, sin barrido.
    pub(crate) const fn nota(wave: Wave, freq: f32, dur: f32, vol: f32, decay: f32) -> Self {
        Self::new(wave, freq, freq, dur, vol, decay)
    }

    // Solo los usan los tests de `music`, que comprueban que las notas caen
    // donde tienen que caer y que ninguna se sale del bucle.
    #[cfg(test)]
    pub(crate) fn inicio(&self) -> f32 {
        self.delay
    }

    #[cfg(test)]
    pub(crate) fn fin(&self) -> f32 {
        self.delay + self.dur
    }

    /// Solo la usa el test que comprueba que la sala no lleva golpes secos.
    #[cfg(test)]
    pub(crate) fn forma(&self) -> Wave {
        self.wave
    }

    #[cfg(test)]
    pub(crate) fn frecuencia(&self) -> f32 {
        self.f0
    }

    #[cfg(test)]
    pub(crate) fn volumen(&self) -> f32 {
        self.vol
    }

    const fn new(wave: Wave, f0: f32, f1: f32, dur: f32, vol: f32, decay: f32) -> Self {
        Self {
            wave,
            f0,
            f1,
            dur,
            vol,
            decay,
            delay: 0.0,
        }
    }

    pub(crate) const fn tras(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }
}

/// Los sonidos del juego.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sfx {
    Disparo,
    Parry,
    Graze,
    Impacto,
    Fase,
    Muerte,
    Super,
    Victoria,
    Derrota,
    Empezar,
    /// La caida de cada jefe, en el orden de los bailes.
    CaeVals,
    CaeTango,
    CaeCharleston,
    CaeCancan,
    Ficha,
}

const TODOS: [Sfx; 15] = [
    Sfx::Disparo,
    Sfx::Parry,
    Sfx::Graze,
    Sfx::Impacto,
    Sfx::Fase,
    Sfx::Muerte,
    Sfx::Super,
    Sfx::Victoria,
    Sfx::Derrota,
    Sfx::Empezar,
    Sfx::CaeVals,
    Sfx::CaeTango,
    Sfx::CaeCharleston,
    Sfx::CaeCancan,
    Sfx::Ficha,
];

/// Los efectos que suenan **dentro** de la musica, y no por encima de ella.
///
/// Son los tres que son notas y no ruidos, y por eso los unicos que pueden
/// desafinar: una ficha en si y mi sobre un tango en la menor rompe la frase
/// aunque suene bajito. Se sintetizan una vez por tema, en su tonalidad y con
/// el seno de su melodia, y al sonar se coge el del tema que esta puesto.
///
/// El resto se queda como estaba a proposito. El disparo, el roce y el
/// impacto son ruidos y chasquidos, sin tono que chocar; el parry y el super
/// son barridos, que el oido toma por gesto y no por nota; y la muerte TIENE
/// que sonar mal, que es lo que la distingue de pasar de figura.
const TEMATICOS: [Sfx; 3] = [Sfx::Fase, Sfx::Ficha, Sfx::Victoria];

/// Una nota de celesta: el seno de la melodia y su octava, que se apaga
/// enseguida.
///
/// La octava es el macillo, el "ting" que la separa de la melodia; sin ella
/// seria la melodia tocando una nota de mas, y con mas armonicos ya no seria
/// la misma banda. Del mismo seno y la misma caida que las notas cortas de la
/// melodia (`music::tema`), solo que mas arriba.
fn celesta(semitonos: i32, dur: f32, vol: f32, decay: f32, cuando: f32) -> [Voz; 2] {
    let f = music::frecuencia(semitonos);
    [
        Voz::nota(Wave::Sine, f, dur, vol, decay).tras(cuando),
        Voz::nota(Wave::Sine, f * 2.0, dur * 0.5, vol * 0.25, 5.0).tras(cuando),
    ]
}

/// La tonica del tema, en la octava que empieza en `desde` (semitonos sobre
/// La2). Asi cada efecto vive en un registro fijo sea cual sea el tono: si
/// solo se transportara, la ficha del tango (la, arriba del todo) chillaria y
/// la del charleston (fa) se perderia en la melodia.
fn tonica_desde(t: Tema, desde: i32) -> i32 {
    desde + (music::triada_tonica(t)[0] - desde).rem_euclid(12)
}

/// Lo que dura un tiempo del tema, en segundos. Solo lo usa la mezcla de
/// `--exportar-sonidos`, que en la web no existe.
#[cfg(not(target_arch = "wasm32"))]
fn negra(t: Tema) -> f32 {
    music::compas(t).0 / vals_core::TICK_HZ as f32
}

/// Volumen de cada nota de los efectos en tono. Por debajo de lo que suena
/// una nota de la melodia una vez aplicados los dos masters: la musica va a
/// 0,34 y los efectos a 0,6, asi que 0,11 aqui son unos 0,066 al oido, frente
/// a los 0,075 de la melodia. Asoman por encima del acompanamiento y no tapan
/// la frase.
const VOL_EN_TONO: f32 = 0.11;

impl Sfx {
    fn tematico(self) -> bool {
        TEMATICOS.contains(&self)
    }

    /// Lo que suena cuando cae el jefe del baile `baile`.
    pub fn caida(baile: usize) -> Self {
        [
            Sfx::CaeVals,
            Sfx::CaeTango,
            Sfx::CaeCharleston,
            Sfx::CaeCancan,
        ][baile.min(3)]
    }

    fn indice(self) -> usize {
        TODOS.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// La receta de cada sonido, en el tono del tema `t`. Los que no son
    /// `TEMATICOS` no lo miran.
    ///
    /// Devuelve un `Vec` y no un slice estatico porque `Voz::new` es una
    /// llamada a `const fn`, y Rust no promociona esas a `'static`. Da igual:
    /// esto se construye una vez al cargar, no en el bucle de juego.
    fn receta(self, t: Tema) -> Vec<Voz> {
        use Wave::*;
        let [r, iii, _] = music::triada_tonica(t);
        let tercera = iii - r;
        match self {
            // Suena quince veces por segundo: corto y bajito o es insufrible.
            Sfx::Disparo => vec![Voz::new(Square, 780.0, 520.0, 0.055, 0.10, 3.0)],
            // Brillante y ascendente: es la recompensa, tiene que apetecer.
            Sfx::Parry => vec![
                Voz::new(Sine, 720.0, 1500.0, 0.20, 0.35, 2.2),
                Voz::new(Sine, 1440.0, 3000.0, 0.18, 0.12, 2.6),
            ],
            // Casi un susurro: pasa muchas veces seguidas.
            Sfx::Graze => vec![Voz::new(Sine, 2200.0, 2600.0, 0.03, 0.06, 4.0)],
            Sfx::Impacto => vec![Voz::new(Noise, 1.0, 1.0, 0.045, 0.11, 5.0)],
            // Pasar de figura: el acorde de la tonica del tema nuevo, GRAVE y
            // suave, tocado a la vez, como la orquesta que marca el primer
            // tiempo de la pieza siguiente. Se probo con campanillas agudas
            // (estridentes, rompian la melodia) y con nada (raro: la musica
            // cambiaba sin que nada lo subrayase). Grave para que no compita
            // con la melodia, y en el tono del tema que entra, que suena en
            // el mismo frame.
            Sfx::Fase => {
                // Triada cerrada con la raiz entre re3 y do#4: por debajo de
                // donde va la melodia, y por encima de lo que un altavoz de
                // portatil ya no da.
                let raiz = tonica_desde(t, 5);
                [raiz, raiz + tercera, raiz + 7]
                    .iter()
                    .map(|s| Voz::nota(Sine, music::frecuencia(*s), 1.1, VOL_EN_TONO * 0.5, 1.1))
                    .collect()
            }
            Sfx::Muerte => vec![
                Voz::new(Noise, 1.0, 1.0, 0.45, 0.28, 2.0),
                Voz::new(Square, 320.0, 50.0, 0.50, 0.26, 1.4),
            ],
            Sfx::Super => vec![
                Voz::new(Sine, 220.0, 1400.0, 0.55, 0.32, 0.8),
                Voz::new(Noise, 1.0, 1.0, 0.30, 0.10, 2.5),
            ],
            // Tres notas que suben y acaban en casa: tercera, quinta y
            // tonica del tema que estaba sonando. Era do, mi, la, que es la
            // menor y solo cuadraba con la sala; la forma es la misma.
            Sfx::Victoria => {
                let arriba = tonica_desde(t, 27);
                let v = VOL_EN_TONO * 1.4;
                [
                    celesta(arriba - 12 + tercera, 0.18, v, 2.0, 0.0),
                    celesta(arriba - 5, 0.18, v, 2.0, 0.14),
                    celesta(arriba, 0.45, v * 1.1, 1.4, 0.28),
                ]
                .concat()
            }
            Sfx::Derrota => vec![
                Voz::new(Saw, 200.0, 70.0, 0.90, 0.28, 1.2),
                Voz::new(Sine, 100.0, 40.0, 0.90, 0.20, 1.2),
            ],
            Sfx::Empezar => vec![Voz::new(Sine, 440.0, 880.0, 0.15, 0.25, 2.0)],
            // La caja sin cuerda: la melodia bajando, cada nota mas tarde y
            // un pelo mas desafinada que la anterior, y el portazo de la tapa.
            Sfx::CaeVals => {
                let mut v: Vec<Voz> = [1568.0, 1319.0, 1175.0, 988.0, 880.0, 659.0, 587.0]
                    .iter()
                    .scan(0.0f32, |t, &f| {
                        let n = Voz::new(Sine, f, f * 0.97, 0.3, 0.16, 3.0).tras(*t);
                        *t += 0.09 + *t * 0.25;
                        Some(n)
                    })
                    .collect();
                v.push(Voz::new(Noise, 1.0, 1.0, 0.09, 0.3, 4.0).tras(1.32));
                v.push(Voz::new(Square, 120.0, 55.0, 0.12, 0.18, 3.0).tras(1.32));
                v
            }
            // El fuelle: coge aire y lo suelta en un resoplido largo que se
            // va quedando grave, como una lengueta sin aire.
            Sfx::CaeTango => vec![
                Voz::new(Noise, 1.0, 1.0, 0.25, 0.05, 0.6),
                Voz::new(Saw, 330.0, 60.0, 1.5, 0.12, 0.7).tras(0.26),
                Voz::new(Saw, 336.0, 64.0, 1.5, 0.08, 0.7).tras(0.26),
                Voz::new(Noise, 1.0, 1.0, 1.4, 0.07, 0.8).tras(0.26),
            ],
            // El disco rayado: tres rascadas, el plato frenando y el disco
            // silbando al salir volando.
            Sfx::CaeCharleston => {
                let mut v = Vec::new();
                for t in [0.0, 0.12, 0.22] {
                    v.push(Voz::new(Noise, 1.0, 1.0, 0.07, 0.28, 3.0).tras(t));
                    v.push(Voz::new(Saw, 900.0, 300.0, 0.1, 0.1, 2.0).tras(t));
                }
                v.push(Voz::new(Saw, 260.0, 40.0, 1.0, 0.13, 1.0).tras(0.35));
                v.push(Voz::new(Sine, 600.0, 1500.0, 0.45, 0.08, 1.2).tras(0.8));
                v
            }
            // La fila cayendo: un pito de dibujo animado bajando y un golpe
            // sordo por cada corista que llega al suelo.
            Sfx::CaeCancan => {
                let mut v = vec![Voz::new(Sine, 1400.0, 500.0, 0.5, 0.07, 1.2).tras(0.05)];
                for i in 0..5 {
                    let t = 0.37 + 0.22 * i as f32;
                    v.push(Voz::new(Sine, 190.0, 70.0, 0.14, 0.26, 3.0).tras(t));
                    v.push(Voz::new(Noise, 1.0, 1.0, 0.04, 0.1, 3.0).tras(t));
                }
                v
            }
            // Dominante y tonica, la segunda encima de la primera: el
            // tintineo de una moneda de toda la vida (era si y mi, que es lo
            // mismo en mi), pero en el tono del tema y en celesta.
            Sfx::Ficha => {
                let arriba = tonica_desde(t, 32);
                [
                    celesta(arriba - 5, 0.08, VOL_EN_TONO, 3.0, 0.0),
                    celesta(arriba, 0.3, VOL_EN_TONO * 1.1, 2.2, 0.07),
                ]
                .concat()
            }
        }
    }
}

/// Los sonidos ya cargados.
pub struct Audio {
    /// Por sonido, sus variantes: una por tema si es de los `TEMATICOS`, y
    /// una sola si no.
    sonidos: Vec<Vec<Option<Sound>>>,
    /// Un tema por baile.
    musica: Vec<Option<Sound>>,
    /// Que sala hay en su hueco, el primero: los bailes ganados con que se
    /// sintetizo. Hay dieciseis y solo se guarda una.
    sala: u8,
    /// La sala que se esta sintetizando porque ha cambiado lo ganado.
    sala_nueva: Option<(u8, Coroutine<Option<Sound>>)>,
    /// Cual esta sonando ahora.
    sonando: Option<usize>,
    pub muted: bool,
    /// Lo que dicen las opciones, de 0 a 1. La musica ya viene agachada si
    /// esta abierto el entreacto.
    volumen_musica: f32,
    volumen_efectos: f32,
}

impl Audio {
    /// Sintetiza y carga todos los sonidos, con la sala de los bailes que ya
    /// se hayan ganado.
    ///
    /// Si alguno falla se queda a `None` y el juego sigue sin el: quedarse sin
    /// audio es un incordio, no un motivo para no poder jugar.
    pub async fn load(sala: u8) -> Self {
        let mut sonidos = Vec::with_capacity(TODOS.len());
        let mut bytes_en_tono = 0usize;
        for sfx in TODOS {
            let temas: &[Tema] = if sfx.tematico() {
                &TEMAS
            } else {
                &[Tema::Sala(0)]
            };
            let mut variantes = Vec::with_capacity(temas.len());
            for t in temas {
                let bytes = wav_sfx(sfx, *t);
                if sfx.tematico() {
                    bytes_en_tono += bytes.len();
                }
                match load_sound_from_bytes(&bytes).await {
                    Ok(s) => variantes.push(Some(s)),
                    Err(e) => {
                        println!("[audio] no se pudo cargar un sonido: {e}");
                        variantes.push(None);
                    }
                }
            }
            sonidos.push(variantes);
        }
        let cargados = sonidos.iter().flatten().filter(|s| s.is_some()).count();
        let total = sonidos.iter().map(Vec::len).sum::<usize>();

        let mut musica = Vec::with_capacity(TEMAS.len());
        let mut bytes_musica = 0usize;
        for b in TEMAS {
            let b = match b {
                Tema::Sala(_) => Tema::Sala(sala),
                b => b,
            };
            let bytes = wav_tema(b);
            bytes_musica += bytes.len();
            match load_sound_from_bytes(&bytes).await {
                Ok(s) => musica.push(Some(s)),
                Err(e) => {
                    println!("[audio] no se pudo cargar la musica de {b:?}: {e}");
                    musica.push(None);
                }
            }
        }
        let temas = musica.iter().filter(|s| s.is_some()).count();
        println!(
            "[audio] {cargados}/{total} sonidos y {temas}/{} temas sintetizados \
             ({:.1} MB de musica, {:.2} MB de efectos en tono)",
            TEMAS.len(),
            bytes_musica as f32 / (1024.0 * 1024.0),
            bytes_en_tono as f32 / (1024.0 * 1024.0)
        );

        Self {
            sonidos,
            musica,
            sala,
            sala_nueva: None,
            sonando: None,
            muted: false,
            volumen_musica: 1.0,
            volumen_efectos: 1.0,
        }
    }

    /// Pone el tema de un baile, en bucle. No hace nada si ya sonaba.
    pub fn poner_musica(&mut self, b: Tema) {
        if let Tema::Sala(bailes) = b {
            self.cambiar_sala(bailes);
            // Si se llega a la sala y la que hay ya no vale, se espera a la
            // nueva en silencio: un frame de la vieja suena a error.
            if self.sala_nueva.is_some() && self.sonando != Some(0) {
                self.parar_musica();
                self.sonando = None;
                return;
            }
        }
        self.poner(b.indice());
    }

    /// La sala que toca con lo ganado. Si no es la que hay, se sintetiza
    /// aparte, sin parar el juego, y entra en cuanto esta: al volver de ganar
    /// un baile, la pista suena con lo que ese baile ha devuelto.
    fn cambiar_sala(&mut self, bailes: u8) {
        if let Some((nueva, hilo)) = self.sala_nueva.take() {
            if !hilo.is_done() {
                self.sala_nueva = Some((nueva, hilo));
                return;
            }
            if self.sonando == Some(0) {
                self.parar_musica();
                self.sonando = None;
            }
            self.musica[0] = hilo.retrieve().flatten();
            self.sala = nueva;
        }
        if bailes != self.sala {
            let t = Tema::Sala(bailes);
            let hilo =
                start_coroutine(async move { load_sound_from_bytes(&wav_tema(t)).await.ok() });
            self.sala_nueva = Some((bailes, hilo));
        }
    }

    fn poner(&mut self, idx: usize) {
        if self.sonando == Some(idx) {
            return;
        }
        self.parar_musica();
        if self.muted {
            // Se anota igual cual toca, para que al quitar el mudo entre sola.
            self.sonando = Some(idx);
            return;
        }
        if let Some(Some(s)) = self.musica.get(idx) {
            play_sound(
                s,
                PlaySoundParams {
                    looped: true,
                    volume: MASTER_MUSICA * self.volumen_musica,
                },
            );
        }
        self.sonando = Some(idx);
    }

    fn parar_musica(&mut self) {
        if let Some(i) = self.sonando
            && let Some(Some(s)) = self.musica.get(i)
        {
            stop_sound(s);
        }
    }

    /// Los volumenes, de 0 a 1. Se llama cada frame y solo toca el tema que
    /// suena si ha cambiado: asi el entreacto agacha la musica en el sitio, sin
    /// cortarla ni volver a empezarla.
    pub fn volumenes(&mut self, musica: f32, efectos: f32) {
        self.volumen_efectos = efectos;
        if musica == self.volumen_musica {
            return;
        }
        self.volumen_musica = musica;
        if !self.muted
            && let Some(i) = self.sonando
            && let Some(Some(s)) = self.musica.get(i)
        {
            set_sound_volume(s, MASTER_MUSICA * musica);
        }
    }

    /// Silencia o devuelve el sonido, musica incluida.
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        let actual = self.sonando;
        if self.muted {
            self.parar_musica();
            self.sonando = actual;
        } else if let Some(i) = actual {
            self.sonando = None;
            self.poner(i);
        }
    }

    pub fn play(&self, sfx: Sfx, vol: f32) {
        if self.muted {
            return;
        }
        // Los efectos en tono cogen la variante del tema puesto. Antes de que
        // suene ninguno (o si el sonido no tiene variantes) vale la primera,
        // que es la de la sala.
        let tema = self.sonando.unwrap_or(0);
        if let Some(v) = self.sonidos.get(sfx.indice())
            && let Some(Some(s)) = v.get(tema).or(v.first())
        {
            play_sound(
                s,
                PlaySoundParams {
                    looped: false,
                    volume: (vol * MASTER * self.volumen_efectos).clamp(0.0, 1.0),
                },
            );
        }
    }

    /// Traduce lo que ha pasado en el tick a sonidos.
    ///
    /// Es el unico sitio que junta las dos capas, y va en un solo sentido: el
    /// core publica lo que ha ocurrido y aqui se decide como suena.
    pub fn play_events(&self, ev: &Events) {
        if ev.player_shot {
            self.play(Sfx::Disparo, 1.0);
        }
        if ev.parried > 0 {
            // Un solo sonido aunque caigan cinco balas de golpe: encadenar
            // cinco copias solo produce un chasquido saturado.
            self.play(Sfx::Parry, 1.0);
        }
        if ev.grazed > 0 {
            self.play(Sfx::Graze, 1.0);
        }
        if ev.boss_hit {
            self.play(Sfx::Impacto, 1.0);
        }
        if ev.phase_changed {
            self.play(Sfx::Fase, 1.0);
        }
        if ev.super_fired {
            self.play(Sfx::Super, 1.0);
        }
        if ev.fichas > 0 {
            self.play(Sfx::Ficha, 1.0);
        }
        if ev.player_died {
            self.play(Sfx::Muerte, 1.0);
        }
        if ev.victory {
            self.play(Sfx::Victoria, 1.0);
        }
        if ev.defeat {
            self.play(Sfx::Derrota, 1.0);
        }
    }
}

/// Un tema sintetizado y empaquetado como WAV, tal cual se carga.
fn wav_tema(t: Tema) -> Vec<u8> {
    wav_a(
        &render_len_a(&music::tema(t), music::duracion(t), SAMPLE_RATE_MUSICA),
        SAMPLE_RATE_MUSICA,
    )
}

/// Escribe cada tema como un `.wav` en `dir`, para escucharlos fuera del
/// juego. La musica no existe como fichero: se sintetiza al arrancar desde
/// las partituras de `music.rs`, y esto es la misma sintesis puesta en disco.
#[cfg(not(target_arch = "wasm32"))]
pub fn exportar_musica(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut hechos = Vec::with_capacity(TEMAS.len());
    for (i, b) in TEMAS.into_iter().enumerate() {
        let ruta = dir.join(format!("{i:02} {}.wav", fichero(b)));
        std::fs::write(&ruta, wav_tema(b))?;
        hechos.push(ruta);
    }
    // Y las otras quince salas, con los bailes que las llenan en el nombre.
    for bailes in 1..16u8 {
        let quien: Vec<&str> = ["vals", "tango", "charleston", "cancan"]
            .into_iter()
            .enumerate()
            .filter(|(i, _)| bailes & (1 << i) != 0)
            .map(|(_, b)| b)
            .collect();
        let ruta = dir.join(format!("00 la sala {bailes:02} - {}.wav", quien.join(" ")));
        std::fs::write(&ruta, wav_tema(Tema::Sala(bailes)))?;
        hechos.push(ruta);
    }
    Ok(hechos)
}

/// El titulo de un tema sin lo que un sistema de ficheros no traga.
#[cfg(not(target_arch = "wasm32"))]
fn fichero(t: Tema) -> String {
    t.titulo()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Escribe cada efecto en tono de cada tema, tal cual lo carga el juego, y
/// una mezcla por tema para juzgarlos donde suenan: los primeros compases
/// con la fase en el uno (como al entrar la figura), dos fichas sueltas y la
/// victoria al final, cada cosa con el master con que suena en el juego.
#[cfg(not(target_arch = "wasm32"))]
pub fn exportar_sonidos(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut hechos = Vec::new();
    for (i, t) in TEMAS.into_iter().enumerate() {
        let nombre = fichero(t);
        for (sfx, que) in TEMATICOS.into_iter().zip(["fase", "ficha", "victoria"]) {
            let ruta = dir.join(format!("{i:02} {nombre} - {que}.wav"));
            std::fs::write(&ruta, wav_sfx(sfx, t))?;
            hechos.push(ruta);
        }
        let ruta = dir.join(format!("{i:02} {nombre} - mezcla.wav"));
        std::fs::write(&ruta, wav_a(&mezcla(t), SAMPLE_RATE_MUSICA))?;
        hechos.push(ruta);
    }
    Ok(hechos)
}

/// Cinco compases del tema con los efectos en tono encima, normalizado al
/// final: lo que importa es la proporcion entre musica y efectos, que es la
/// del juego, no el volumen absoluto.
#[cfg(not(target_arch = "wasm32"))]
fn mezcla(t: Tema) -> Vec<i16> {
    let r = SAMPLE_RATE_MUSICA;
    let compas = negra(t) * music::compas(t).1 as f32;
    let largo = (compas * 5.0).min(music::duracion(t));
    let mut m: Vec<f32> = render_len_a(&music::tema(t), largo, r)
        .iter()
        .map(|x| *x as f32 * MASTER_MUSICA)
        .collect();
    let encima = |m: &mut Vec<f32>, sfx: Sfx, cuando: f32| {
        let i0 = (cuando * r as f32) as usize;
        for (k, x) in render_a(&sfx.receta(t), r).iter().enumerate() {
            if let Some(y) = m.get_mut(i0 + k) {
                *y += *x as f32 * MASTER;
            }
        }
    };
    encima(&mut m, Sfx::Fase, 0.0);
    encima(&mut m, Sfx::Ficha, compas + negra(t));
    encima(&mut m, Sfx::Ficha, compas * 2.0 + negra(t) * 1.5);
    encima(&mut m, Sfx::Victoria, compas * 3.0);
    let pico = m.iter().fold(1.0f32, |a, x| a.max(x.abs()));
    m.iter()
        .map(|x| (x / pico * 0.9 * i16::MAX as f32) as i16)
        .collect()
}

/// Un efecto ya empaquetado. Los de tono van a la frecuencia de la musica,
/// como ella: son senos por debajo de 3 kHz, no pierden nada, y catorce
/// variantes de cada uno ocupan la mitad.
fn wav_sfx(sfx: Sfx, t: Tema) -> Vec<u8> {
    let rate = if sfx.tematico() {
        SAMPLE_RATE_MUSICA
    } else {
        SAMPLE_RATE
    };
    wav_a(&render_a(&sfx.receta(t), rate), rate)
}

/// Mezcla las voces en muestras de 16 bits.
#[cfg(test)]
fn render(voces: &[Voz]) -> Vec<i16> {
    render_a(voces, SAMPLE_RATE)
}

/// Igual, a la frecuencia de muestreo que se pida.
fn render_a(voces: &[Voz], rate: u32) -> Vec<i16> {
    let total = voces
        .iter()
        .map(|v| v.delay + v.dur)
        .fold(0.0f32, f32::max)
        .max(0.001);
    render_len_a(voces, total, rate)
}

/// Igual, pero con la duracion exacta del buffer, y a la frecuencia de
/// muestreo que se pida.
///
/// La musica lo necesita: el bucle tiene que durar los compases justos. Si se
/// dejara terminar en la ultima nota, el bucle daria un salto cada vuelta.
fn render_len_a(voces: &[Voz], segundos: f32, rate: u32) -> Vec<i16> {
    let n = (segundos.max(0.001) * rate as f32) as usize;
    let mut acc = vec![0.0f32; n];

    for v in voces {
        let inicio = (v.delay * rate as f32) as usize;
        let largo = ((v.dur * rate as f32) as usize).max(1);
        let mut fase = 0.0f32;
        // Ruido con un generador propio: nada de aleatoriedad del sistema, para
        // que el mismo sonido salga igual en cada arranque.
        let mut rnd: u32 = 0x5641_4C53;

        for i in 0..largo {
            let idx = inicio + i;
            if idx >= n {
                break;
            }
            let t = i as f32 / largo as f32;
            let f = v.f0 + (v.f1 - v.f0) * t;
            fase += f / rate as f32;
            if fase >= 1.0 {
                fase -= 1.0;
            }

            let onda = match v.wave {
                Wave::Sine => vals_core::math::sin(fase * vals_core::math::TAU),
                Wave::Square => {
                    if fase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Saw => fase * 2.0 - 1.0,
                Wave::Noise => {
                    rnd = rnd.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (rnd >> 8) as f32 / 8_388_608.0 - 1.0
                }
            };

            // Ataque muy corto para que no chasquee al empezar, y caida
            // exponencial: es lo que hace que suene a golpe y no a pitido.
            let ataque = (t * 60.0).min(1.0);
            let env = ataque * (1.0 - t).powf(v.decay);
            acc[idx] += onda * env * v.vol;
        }
    }

    acc.iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// Empaqueta muestras mono de 16 bits en un WAV.
#[cfg(test)]
fn wav(samples: &[i16]) -> Vec<u8> {
    wav_a(samples, SAMPLE_RATE)
}

/// Igual, declarando la frecuencia de muestreo que toque.
fn wav_a(samples: &[i16], rate: u32) -> Vec<u8> {
    let datos = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + datos);

    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + datos as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");

    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // tamano del bloque fmt
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM sin comprimir
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // bytes por segundo
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes por muestra
    out.extend_from_slice(&16u16.to_le_bytes()); // bits por muestra

    out.extend_from_slice(b"data");
    out.extend_from_slice(&(datos as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cada sonido con cada tema en que se carga: los de tono, con los
    /// catorce; los demas, con la sala.
    fn todas_las_variantes() -> Vec<(Sfx, Tema)> {
        TODOS
            .into_iter()
            .flat_map(|s| {
                let temas: &[Tema] = if s.tematico() {
                    &TEMAS
                } else {
                    &[Tema::Sala(0)]
                };
                temas.iter().map(move |t| (s, *t))
            })
            .collect()
    }

    /// Los temas tal cual se cargan, y la sala con cada combinacion de
    /// bailes ganados, que se sintetiza cuando cambia.
    fn musicas() -> Vec<Tema> {
        TEMAS.into_iter().chain((1..16).map(Tema::Sala)).collect()
    }

    fn pico(m: &[i16]) -> f32 {
        m.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0) as f32 / i16::MAX as f32
    }

    fn u32_en(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
    }
    fn u16_en(b: &[u8], i: usize) -> u16 {
        u16::from_le_bytes(b[i..i + 2].try_into().unwrap())
    }

    #[test]
    fn la_cabecera_wav_esta_bien_formada() {
        // Importa de verdad: el decodificador de macroquad entra en panico con
        // una cabecera mala, no devuelve error.
        let bytes = wav(&[0i16; 100]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");

        assert_eq!(u32_en(&bytes, 16), 16, "tamano del bloque fmt");
        assert_eq!(u16_en(&bytes, 20), 1, "PCM");
        assert_eq!(u16_en(&bytes, 22), 1, "mono");
        assert_eq!(u32_en(&bytes, 24), SAMPLE_RATE);
        assert_eq!(u32_en(&bytes, 28), SAMPLE_RATE * 2, "bytes por segundo");
        assert_eq!(u16_en(&bytes, 32), 2, "bytes por muestra");
        assert_eq!(u16_en(&bytes, 34), 16, "bits por muestra");

        assert_eq!(u32_en(&bytes, 40), 200, "tamano de los datos");
        assert_eq!(u32_en(&bytes, 4), 36 + 200, "tamano declarado del RIFF");
        assert_eq!(bytes.len(), 44 + 200);
    }

    #[test]
    fn todos_los_sonidos_producen_un_wav_plausible() {
        for (sfx, t) in todas_las_variantes() {
            let bytes = wav_sfx(sfx, t);
            assert!(bytes.len() > 44, "sonido vacio");
            assert_eq!(&bytes[0..4], b"RIFF");
            // Coherencia interna: lo que declara y lo que ocupa.
            assert_eq!(u32_en(&bytes, 40) as usize, bytes.len() - 44);
            assert_eq!(u32_en(&bytes, 4) as usize, bytes.len() - 8);
            // Y que suene a algo: alguna muestra lejos del silencio.
            let pico = bytes[44..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| i16::from_le_bytes(*c).unsigned_abs())
                .max()
                .unwrap_or(0);
            assert!(pico > 1000, "el sonido es practicamente silencio: {pico}");
        }
    }

    #[test]
    fn ninguna_muestra_satura() {
        for (sfx, t) in todas_las_variantes() {
            let s = render(&sfx.receta(t));
            // Se recorta a i16::MAX por construccion; esto vigila que las
            // recetas no vivan pegadas al techo, que es donde distorsiona.
            let pico = s.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
            assert!(
                pico < i16::MAX as u16,
                "una receta llega al tope y va a distorsionar"
            );
        }
    }

    #[test]
    fn los_temas_de_musica_duran_lo_que_dicen() {
        for b in musicas() {
            let muestras = render_len_a(&music::tema(b), music::duracion(b), SAMPLE_RATE_MUSICA);
            let esperado = (music::duracion(b) * SAMPLE_RATE_MUSICA as f32) as usize;
            assert_eq!(muestras.len(), esperado, "{b:?}");
            // Y suenan: hay senal de verdad, no silencio.
            let pico = muestras.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
            assert!(pico > 3000, "{b:?} suena a casi nada: {pico}");
        }
    }

    #[test]
    fn los_temas_no_saturan() {
        for b in musicas() {
            let muestras = render_len_a(&music::tema(b), music::duracion(b), SAMPLE_RATE_MUSICA);
            let pico = muestras.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
            assert!(pico < i16::MAX as u16, "{b:?} llega al tope y distorsiona");
        }
    }

    #[test]
    fn el_bucle_no_da_un_salto_al_volver_a_empezar() {
        // El final y el principio tienen que ser silencio o casi: si el bucle
        // corta una nota a media vibracion, se oye un chasquido cada vuelta.
        for b in musicas() {
            let m = render_len_a(&music::tema(b), music::duracion(b), SAMPLE_RATE_MUSICA);
            let cola: i32 = m[m.len() - 200..]
                .iter()
                .map(|v| (*v as i32).abs())
                .max()
                .unwrap();
            assert!(cola < 2500, "{b:?} termina con senal viva: {cola}");
        }
    }

    #[test]
    fn las_notas_retrasadas_alargan_el_sonido() {
        let corta = render(&[Voz::new(Wave::Sine, 440.0, 440.0, 0.1, 0.5, 2.0)]);
        let larga = render(&[
            Voz::new(Wave::Sine, 440.0, 440.0, 0.1, 0.5, 2.0),
            Voz::new(Wave::Sine, 440.0, 440.0, 0.1, 0.5, 2.0).tras(0.2),
        ]);
        assert!(larga.len() > corta.len() * 2);
    }

    #[test]
    fn pasar_de_figura_suena_grave() {
        // Nada por encima de un la4: es un acorde de acompanamiento, no un
        // aviso que se oiga por encima de la melodia.
        for t in TEMAS {
            for v in Sfx::Fase.receta(t) {
                assert!(v.frecuencia() < 445.0, "{t:?}: {} Hz", v.frecuencia());
            }
        }
    }

    #[test]
    fn la_sintesis_es_reproducible() {
        // El ruido usa un generador propio, asi que el mismo sonido sale igual
        // en cada arranque.
        let muerte = Sfx::Muerte.receta(Tema::Sala(0));
        assert_eq!(render(&muerte), render(&muerte));
        // Y las variantes en tono, igual: misma receta, mismas muestras.
        for t in TEMAS {
            assert_eq!(wav_sfx(Sfx::Ficha, t), wav_sfx(Sfx::Ficha, t), "{t:?}");
        }
    }

    #[test]
    fn los_efectos_en_tono_tocan_la_triada_de_su_tema() {
        // Todas las notas, la octava del macillo incluida, son tonica, tercera
        // o quinta del tema. Una sola fuera y la ficha vuelve a desafinar.
        for sfx in TEMATICOS {
            for t in TEMAS {
                let triada = music::triada_tonica(t).map(|s| s.rem_euclid(12));
                for v in sfx.receta(t) {
                    let f = v.frecuencia();
                    let suya = (0..72).any(|s| {
                        triada.contains(&(s % 12)) && (music::frecuencia(s) - f).abs() < f * 0.003
                    });
                    assert!(suya, "{t:?}: {f} Hz no es de la triada de la tonica");
                }
            }
        }
    }

    #[test]
    fn los_efectos_en_tono_no_tapan_la_musica() {
        // Cada uno, con su master, a menos de tres cuartos de lo que llega a
        // sonar el tema con el suyo: tienen que asomar, no ponerse delante (la
        // fase de antes pasaba del tema entero). Y sin chillar: ninguna
        // fundamental por encima de 1,4 kHz.
        for sfx in TEMATICOS {
            for t in TEMAS {
                let tema = render_len_a(&music::tema(t), music::duracion(t), SAMPLE_RATE_MUSICA);
                let techo = pico(&tema) * MASTER_MUSICA * 0.75;
                let suyo = pico(&render(&sfx.receta(t))) * MASTER;
                assert!(
                    suyo < techo,
                    "{t:?}: {suyo} por encima de la musica, {techo}"
                );
                for v in sfx.receta(t).iter().filter(|v| v.vol >= VOL_EN_TONO) {
                    assert!(v.f0 < 1400.0, "{t:?}: {} Hz chilla", v.f0);
                }
            }
        }
    }
}
