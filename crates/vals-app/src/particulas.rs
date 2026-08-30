//! Chispas, fogonazos y explosiones.
//!
//! Lo que separa "los marcianitos" de un juego que se siente caro casi nunca es
//! el dibujo: es que las cosas **reaccionen**. Disparar tiene que echar algo por
//! la boca, un impacto tiene que soltar chispas, y morir tiene que reventar.
//!
//! Todo esto vive en `vals-app` y **no toca la simulacion**. Se dispara desde el
//! `Events` que el core ya publicaba desde H7, que resulta traer exactamente los
//! diez sucesos que hacen falta. Si esto se hubiera metido en `vals-core`, el
//! replay dorado se pondria rojo, que es justo para lo que esta.
//!
//! Mismo patron que `vals_core::bullets`: arrays paralelos, hueco preasignado y
//! lista de libres. **Cero reservas de memoria mientras se juega**, que en un
//! sistema que emite en rafagas es lo que evita los tirones cuando mas cosas
//! pasan.

use macroquad::prelude::*;
use vals_core::{Events, Pcg32, World};

/// Tope de particulas vivas. Se reserva de golpe al arrancar.
pub const MAX: usize = 2048;

/// Cuanto frena una particula por segundo, como fraccion de su velocidad.
const ROZAMIENTO: f32 = 2.6;
/// Largo de la estela, en segundos de vuelo dibujados hacia atras.
const ESTELA: f32 = 0.035;

/// El sistema entero.
pub struct Particulas {
    pos: Vec<Vec2>,
    vel: Vec<Vec2>,
    /// Vida que le queda, en segundos.
    vida: Vec<f32>,
    /// La que tenia al nacer, para poder desvanecerla.
    vida_max: Vec<f32>,
    radio: Vec<f32>,
    color: Vec<Color>,
    libres: Vec<u32>,
    /// Hasta donde se ha llegado a usar. Recorrer solo esto ahorra pasear por
    /// dos mil huecos vacios cuando no hay casi nada.
    tope: usize,
    rng: Pcg32,
}

impl Particulas {
    pub fn new() -> Self {
        Self {
            pos: vec![Vec2::ZERO; MAX],
            vel: vec![Vec2::ZERO; MAX],
            vida: vec![0.0; MAX],
            vida_max: vec![1.0; MAX],
            radio: vec![0.0; MAX],
            color: vec![WHITE; MAX],
            libres: (0..MAX as u32).rev().collect(),
            tope: 0,
            rng: Pcg32::new(0x5641_4C53_5041_5254),
        }
    }

    /// Cuantas hay ahora mismo. Sale en el overlay de F1: cuando algo del zumo
    /// no se ve, lo primero que hay que saber es si es que no sale o es que no
    /// se dibuja.
    pub fn vivas(&self) -> usize {
        self.vida[..self.tope].iter().filter(|v| **v > 0.0).count()
    }

    /// Un paso. Se llama con el mismo paso fijo que la simulacion, para que las
    /// chispas no vayan mas rapido en un monitor de 144 Hz.
    pub fn step(&mut self, dt: f32) {
        for i in 0..self.tope {
            if self.vida[i] <= 0.0 {
                continue;
            }
            self.vida[i] -= dt;
            if self.vida[i] <= 0.0 {
                self.libres.push(i as u32);
                continue;
            }
            let p = self.pos[i] + self.vel[i] * dt;
            self.pos[i] = p;
            self.vel[i] *= 1.0 - (ROZAMIENTO * dt).min(1.0);
        }
    }

    /// Suelta una particula. Si no hay hueco, no pasa nada: perder una chispa
    /// no se nota, y quedarse sin memoria si.
    fn soltar(&mut self, pos: Vec2, vel: Vec2, vida: f32, radio: f32, color: Color) {
        let Some(i) = self.libres.pop() else {
            return;
        };
        let i = i as usize;
        self.pos[i] = pos;
        self.vel[i] = vel;
        self.vida[i] = vida;
        self.vida_max[i] = vida;
        self.radio[i] = radio;
        self.color[i] = color;
        self.tope = self.tope.max(i + 1);
    }

    /// Un numero aleatorio en `[-1, 1]`.
    fn azar(&mut self) -> f32 {
        self.rng.next_f32() * 2.0 - 1.0
    }

    /// Una rafaga en todas direcciones.
    fn rafaga(&mut self, pos: Vec2, n: u32, vel: f32, vida: f32, radio: f32, color: Color) {
        for _ in 0..n {
            let a = self.rng.next_f32() * std::f32::consts::TAU;
            let v = vel * (0.45 + self.rng.next_f32() * 0.55);
            let dir = vec2(a.cos(), a.sin());
            let dispersion = 0.75 + self.rng.next_f32() * 0.5;
            self.soltar(pos, dir * v, vida * dispersion, radio, color);
        }
    }

    /// Una rafaga en un cono, para lo que sale disparado hacia algun lado.
    fn cono(&mut self, pos: Vec2, dir: Vec2, n: u32, vel: f32, vida: f32, color: Color) {
        for _ in 0..n {
            let desvio = self.azar() * 0.5;
            let (s, c) = (desvio.sin(), desvio.cos());
            let d = vec2(dir.x * c - dir.y * s, dir.x * s + dir.y * c);
            let v = vel * (0.6 + self.rng.next_f32() * 0.6);
            self.soltar(pos, d * v, vida, 1.6, color);
        }
    }

    /// Traduce lo que ha pasado este frame a lo que se ve.
    ///
    /// Es el unico sitio donde se decide cuanto zumo lleva cada suceso, para
    /// que subirle o bajarle el volumen al juego entero sea tocar aqui.
    pub fn reaccionar(&mut self, ev: &Events, world: &World) {
        let jugador = vec2(world.player.pos.x, world.player.pos.y);
        let jefe = vec2(world.boss.pos.x, world.boss.pos.y);

        if ev.player_shot {
            // Un fogonazo corto hacia arriba: la boca del arma.
            self.cono(
                jugador + vec2(0.0, -12.0),
                vec2(0.0, -1.0),
                3,
                260.0,
                0.10,
                SHOT,
            );
        }
        if ev.boss_hit {
            self.cono(jefe, vec2(0.0, 1.0), 5, 320.0, 0.22, CHISPA);
        }
        // `grazed` y `parried` no son banderas, son cuentas: dicen a cuantas
        // balas has rozado o parriado en el tick. Se aprovecha, con tope, para
        // que parriar cinco de golpe se vea mas que parriar una.
        if ev.grazed > 0 {
            self.rafaga(jugador, 3 * ev.grazed.min(4), 150.0, 0.20, 1.4, GRAZE);
        }
        if ev.parried > 0 {
            // El parry es lo que el juego quiere que hagas: es el que mas luce.
            self.rafaga(jugador, 18 * ev.parried.min(4), 420.0, 0.42, 2.4, PARRY);
        }
        if ev.super_fired {
            self.rafaga(jugador, 90, 620.0, 0.75, 3.0, SUPER);
        }
        if ev.phase_changed {
            self.rafaga(jefe, 60, 480.0, 0.65, 2.6, FASE);
        }
        if ev.boss_down {
            self.rafaga(jefe, 140, 700.0, 1.10, 3.4, FASE);
        }
        if ev.player_died {
            self.rafaga(jugador, 70, 520.0, 0.85, 2.8, MUERTE);
        }
    }

    /// Dibuja. Cada particula es una estela corta hacia atras, no un punto: una
    /// raya en la direccion en la que va se lee como velocidad, y un circulito
    /// se lee como confeti.
    pub fn draw(&self, punto: impl Fn(Vec2) -> Vec2, escala: f32) {
        for i in 0..self.tope {
            if self.vida[i] <= 0.0 {
                continue;
            }
            let t = (self.vida[i] / self.vida_max[i]).clamp(0.0, 1.0);
            let a = self.pos[i];
            let b = a - self.vel[i] * ESTELA;
            let (pa, pb) = (punto(a), punto(b));
            let c = Color {
                a: self.color[i].a * t,
                ..self.color[i]
            };
            let grosor = (self.radio[i] * escala * t).max(1.0);
            draw_line(pa.x, pa.y, pb.x, pb.y, grosor, c);
        }
    }
}

// Los colores de cada cosa. Cada suceso tiene el suyo para que se distinga de
// un vistazo sin tener que leer nada.
const SHOT: Color = color_u8!(180, 255, 240, 220);
const CHISPA: Color = color_u8!(255, 200, 120, 235);
const GRAZE: Color = color_u8!(120, 255, 200, 200);
const PARRY: Color = color_u8!(255, 145, 210, 255);
const SUPER: Color = color_u8!(255, 235, 140, 255);
const FASE: Color = color_u8!(120, 240, 255, 245);
const MUERTE: Color = color_u8!(255, 90, 130, 245);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_reserva_memoria_mientras_se_juega() {
        // La razon de ser del pool. Si esto creciera, habria un tiron justo en
        // el frame en que mas cosas estan pasando, que es el peor momento.
        let mut p = Particulas::new();
        let capacidades = (p.pos.capacity(), p.vel.capacity(), p.color.capacity());
        for _ in 0..200 {
            p.rafaga(Vec2::ZERO, 40, 300.0, 0.5, 2.0, PARRY);
            p.step(1.0 / 60.0);
        }
        assert_eq!(
            (p.pos.capacity(), p.vel.capacity(), p.color.capacity()),
            capacidades
        );
    }

    #[test]
    fn se_llena_sin_desbordar_y_se_vacia_solo() {
        let mut p = Particulas::new();
        // Mas del doble del tope: las que no caben simplemente no salen.
        for _ in 0..200 {
            p.rafaga(Vec2::ZERO, 40, 300.0, 0.5, 2.0, PARRY);
        }
        assert!(p.vivas() <= MAX);
        // Y con el tiempo se apagan todas y devuelven su hueco.
        for _ in 0..300 {
            p.step(1.0 / 60.0);
        }
        assert_eq!(p.vivas(), 0);
        assert_eq!(p.libres.len(), MAX, "los huecos vuelven a la lista");
    }

    #[test]
    fn las_particulas_frenan_en_vez_de_irse_para_siempre() {
        let mut p = Particulas::new();
        p.soltar(Vec2::ZERO, vec2(400.0, 0.0), 1.0, 2.0, PARRY);
        let mut anterior = f32::MAX;
        for _ in 0..30 {
            p.step(1.0 / 60.0);
            let v = p.vel[0].length();
            assert!(v < anterior, "no esta frenando");
            anterior = v;
        }
    }

    #[test]
    fn es_reproducible() {
        // Sale del generador propio con semilla fija, asi que dos partidas
        // iguales echan las mismas chispas. No hace falta para jugar; hace
        // falta para que una captura de hoy valga manana.
        let correr = || {
            let mut p = Particulas::new();
            p.rafaga(vec2(10.0, 20.0), 30, 300.0, 0.5, 2.0, PARRY);
            p.pos[..30].to_vec()
        };
        assert_eq!(correr(), correr());
    }
}
