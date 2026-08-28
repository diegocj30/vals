//! La pista de baile: el mapa por el que se anda entre jefe y jefe.
//!
//! Es lo que convierte una fila de jefes en un sitio. Un baile es un jefe
//!, asi que la pista es donde estan los bailes: te acercas al que
//! quieras y entras a bailarlo.
//!
//! Vive en el core y no en la app por la misma razon que todo lo demas: aqui no
//! se importa macroquad, asi que andar por la pista se puede probar sin abrir
//! una ventana. La perspectiva —que la pista se vea como un suelo y no como un
//! rectangulo— es cosa del dibujo, no de aqui: en estas coordenadas la pista es
//! plana, y son **las mismas de la arena**, para que el `Layout` que ya existe
//! valga sin tocarlo.
//!
//! Lo que todavia no hace: guardarse. El progreso vive en memoria y se pierde
//! al cerrar. Con un solo baile no compensa meter dependencias para escribir en
//! disco y en `localStorage`; cuando haya tres, si.

use glam::{Vec2, vec2};

use crate::input::InputFrame;
use crate::math::{TAU, sin_cos};
use crate::rng::Pcg32;
use crate::{ARENA_H, ARENA_W, DT};

/// Lo que corre la bailarina por la pista. Mas lento que en combate: aqui no
/// esquiva nada, y correr por un mapa se siente mal.
pub const PASEO_SPEED: f32 = 230.0;
/// Ticks que tarda en ponerse a esa velocidad, y en pararse.
pub const PASEO_ACCEL_TICKS: u32 = 6;
/// Lo cerca que hay que estar de un baile para poder entrar.
pub const RADIO_NODO: f32 = 62.0;
/// Cuanto se aparta del borde. La pista tiene paredes.
pub const MARGEN: f32 = 46.0;

/// Donde empieza la bailarina: delante del todo, mirando a la pista.
pub const ENTRADA: Vec2 = vec2(ARENA_W * 0.5, ARENA_H - 110.0);

/// Cuanta gente hay mirando. Numero par: se reparten a los dos lados.
pub const N_PUBLICO: usize = 14;

/// Alguien de pie en la pista, mirando.
///
/// No hace nada: es decorado que reacciona. Pero es el decorado que cuenta lo
/// unico que la pista no sabia decir —cuanto llevas hecho—, y lo cuenta sin
/// numeros ni barras.
#[derive(Clone, Debug, PartialEq)]
pub struct Miron {
    pub pos: Vec2,
    /// Desfase de su vaiven, para que no se muevan todos a la vez como un
    /// cuerpo de baile. Es lo unico que separa a un corro de gente de una fila
    /// de clones.
    pub desfase: f32,
    /// Lo alto que es, alrededor de 1.
    pub talla: f32,
}

/// Un baile plantado en la pista.
#[derive(Clone, Debug, PartialEq)]
pub struct Nodo {
    pub nombre: String,
    pub pos: Vec2,
    /// Si ya se ha bailado. De momento solo dura lo que dure el programa.
    pub vencido: bool,
}

/// El mapa.
#[derive(Clone, Debug)]
pub struct Pista {
    pub bailarina: Vec2,
    /// Posicion del tick anterior, para interpolar en el render igual que hace
    /// el combate.
    pub prev: Vec2,
    pub vel: Vec2,
    pub nodos: Vec<Nodo>,
    pub publico: Vec<Miron>,
    pub tick: u64,
}

impl Pista {
    /// Una pista con un baile por nombre recibido.
    pub fn new(nombres: Vec<String>) -> Self {
        let n = nombres.len();
        let nodos = nombres
            .into_iter()
            .enumerate()
            .map(|(i, nombre)| Nodo {
                nombre,
                pos: sitio(i, n),
                vencido: false,
            })
            .collect();
        Self {
            bailarina: ENTRADA,
            prev: ENTRADA,
            vel: Vec2::ZERO,
            nodos,
            publico: publico(),
            tick: 0,
        }
    }

    /// Cuanto te respeta la sala, en `[0, 1]`: la fraccion de bailes sacados.
    ///
    /// De aqui sale todo lo que cambia en la pista segun avanzas —lo bien que
    /// bailas tu y lo que te miran los demas—, para que sea **un solo numero**
    /// y no tres reglas que acaben contradiciendose. Sin bailes es 0: una sala
    /// vacia no respeta a nadie.
    pub fn respeto(&self) -> f32 {
        if self.nodos.is_empty() {
            return 0.0;
        }
        let hechos = self.nodos.iter().filter(|n| n.vencido).count();
        hechos as f32 / self.nodos.len() as f32
    }

    /// Un tick de andar. Mismo paso fijo que el combate.
    pub fn step(&mut self, input: InputFrame) {
        self.prev = self.bailarina;
        self.tick += 1;

        let (dx, dy) = input.axis();
        let dir = vec2(dx, dy).normalize_or_zero();
        let deseada = dir * PASEO_SPEED;

        // Se acelera y se frena en un numero fijo de ticks, no con un factor
        // por frame: asi el tacto no depende de a cuantos hercios va el render.
        let paso = PASEO_SPEED / PASEO_ACCEL_TICKS as f32;
        self.vel = acercar(self.vel, deseada, paso);

        self.bailarina += self.vel * DT;
        self.bailarina.x = self.bailarina.x.clamp(MARGEN, ARENA_W - MARGEN);
        self.bailarina.y = self.bailarina.y.clamp(MARGEN, ARENA_H - MARGEN);
    }

    /// El baile que tiene delante, si esta lo bastante cerca para entrar.
    ///
    /// Si dos se solapasen gana el mas cercano, para que acercarse a uno nunca
    /// meta en el otro.
    pub fn nodo_cerca(&self) -> Option<usize> {
        let mut mejor: Option<(usize, f32)> = None;
        for (i, nodo) in self.nodos.iter().enumerate() {
            let d = (nodo.pos - self.bailarina).length();
            if d <= RADIO_NODO && mejor.is_none_or(|(_, m)| d < m) {
                mejor = Some((i, d));
            }
        }
        mejor.map(|(i, _)| i)
    }

    pub fn marcar_vencido(&mut self, i: usize) {
        if let Some(n) = self.nodos.get_mut(i) {
            n.vencido = true;
        }
    }

    /// Si ya no queda nada por bailar.
    pub fn todo_vencido(&self) -> bool {
        !self.nodos.is_empty() && self.nodos.iter().all(|n| n.vencido)
    }

    /// La bailarina como `Player`, para poder posarla con el mismo esqueleto
    /// que en combate.
    ///
    /// Es solo para dibujar. Se le pasa la velocidad porque de ella salen la
    /// inclinacion, la zancada y el arrastre de la falda; el resto del estado
    /// de combate —dash, disparos, invulnerabilidad— no pinta nada en la
    /// pista y se queda a cero.
    pub fn figura(&self) -> crate::Player {
        let mut f = crate::Player::new(Vec2::ZERO);
        f.vel = self.vel;
        f
    }

    /// Un miron como `Player`, para posarlo con el mismo esqueleto.
    ///
    /// La velocidad aqui no es que ande: es **hacia donde se inclina**. El
    /// esqueleto ya sabe inclinar el cuerpo en la direccion en que va, asi que
    /// darle un empujon hacia la bailarina es, gratis, girarse a mirarla. A
    /// respeto cero no se inclina nada: ni te mira.
    pub fn figura_de(&self, m: &Miron) -> crate::Player {
        let mut f = crate::Player::new(m.pos);
        f.vel = (self.bailarina - m.pos).normalize_or_zero() * (95.0 * self.respeto());
        f
    }

    /// Posicion para dibujar, interpolada entre el tick anterior y este.
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev.lerp(self.bailarina, alpha)
    }
}

/// La gente que mira, repartida a los dos lados de la pista.
///
/// A los lados y no en corro por algo practico: por el centro se anda, y ver a
/// la bailarina atravesar a alguien rompe la escena entera. Asi son dos filas
/// y el pasillo queda libre.
fn publico() -> Vec<Miron> {
    let mut rng = Pcg32::new(0x5641_4C53_5055_424C);
    let por_lado = N_PUBLICO / 2;
    (0..N_PUBLICO)
        .map(|i| {
            let lado = if i % 2 == 0 { -1.0 } else { 1.0 };
            let t = (i / 2) as f32 / (por_lado.max(2) - 1) as f32;
            // El desorden es generoso a proposito: con poco, las dos filas se
            // leen como una rejilla en vez de como gente de pie.
            let x = ARENA_W * 0.5 + lado * (ARENA_W * 0.38 + rng.next_f32() * 54.0);
            let y = ARENA_H * (0.10 + 0.80 * t) + (rng.next_f32() - 0.5) * 90.0;
            Miron {
                pos: vec2(x, y),
                desfase: rng.next_f32() * TAU,
                talla: 0.86 + rng.next_f32() * 0.28,
            }
        })
        .collect()
}

/// Mueve `v` hacia `objetivo` como mucho `paso` en cada eje.
fn acercar(v: Vec2, objetivo: Vec2, paso: f32) -> Vec2 {
    let d = objetivo - v;
    if d.length() <= paso {
        objetivo
    } else {
        v + d.normalize_or_zero() * paso
    }
}

/// Donde se planta cada baile.
///
/// Uno solo va al centro; varios se reparten por una elipse, que en
/// perspectiva se lee como un corro en mitad de la pista. Es una funcion y no
/// una tabla para que anadir el tango no sea tambien anadirle un sitio.
fn sitio(i: usize, n: usize) -> Vec2 {
    let centro = vec2(ARENA_W * 0.5, ARENA_H * 0.40);
    if n <= 1 {
        return centro;
    }
    // Mas ancha que alta: el fondo de la pista queda mas lejos que los lados.
    let (rx, ry) = (ARENA_W * 0.30, ARENA_H * 0.20);
    // Empezando por delante, que es por donde se entra: asi el primer baile es
    // con el que te tropiezas primero. Al reves te encontrabas antes el tango,
    // que es el dificil, y el orden en que estan escritos dejaba de significar
    // nada.
    let angulo = TAU * i as f32 / n as f32;
    let (s, c) = sin_cos(angulo);
    centro + vec2(s * rx, c * ry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pista_de(n: usize) -> Pista {
        Pista::new((1..=n).map(|i| format!("Baile {i}")).collect())
    }

    const DERECHA: InputFrame = InputFrame::from_bits(InputFrame::RIGHT);
    const ARRIBA: InputFrame = InputFrame::from_bits(InputFrame::UP);

    #[test]
    fn se_empieza_en_la_entrada_y_quieta() {
        let p = pista_de(1);
        assert_eq!(p.bailarina, ENTRADA);
        assert_eq!(p.vel, Vec2::ZERO);
    }

    #[test]
    fn andar_mueve_y_soltar_para() {
        let mut p = pista_de(1);
        for _ in 0..30 {
            p.step(DERECHA);
        }
        assert!(p.bailarina.x > ENTRADA.x, "no se ha movido");
        for _ in 0..PASEO_ACCEL_TICKS + 2 {
            p.step(InputFrame::NONE);
        }
        assert_eq!(p.vel, Vec2::ZERO, "deberia haberse parado del todo");
    }

    #[test]
    fn la_pista_tiene_paredes() {
        let mut p = pista_de(1);
        for _ in 0..600 {
            p.step(DERECHA);
        }
        assert!((p.bailarina.x - (ARENA_W - MARGEN)).abs() < 0.01);
        assert!(p.bailarina.y <= ARENA_H - MARGEN);
    }

    #[test]
    fn el_primer_baile_es_el_que_pilla_mas_cerca() {
        // El orden de `DEFAULT_BOSS_RONS` es el orden en que estan pensados
        // para jugarse, y la pista tiene que respetarlo: si no, te tropiezas
        // primero con el ultimo.
        let p = pista_de(3);
        let cerca = |n: &Nodo| (n.pos - ENTRADA).length();
        for otro in &p.nodos[1..] {
            assert!(
                cerca(&p.nodos[0]) < cerca(otro),
                "el primer baile no es el mas cercano a la entrada"
            );
        }
    }

    #[test]
    fn un_solo_baile_se_planta_en_medio() {
        let p = pista_de(1);
        assert_eq!(p.nodos.len(), 1);
        assert!((p.nodos[0].pos.x - ARENA_W * 0.5).abs() < 0.01);
    }

    #[test]
    fn varios_bailes_no_se_pisan_y_caben_en_la_pista() {
        for n in 2..=6 {
            let p = pista_de(n);
            for (i, a) in p.nodos.iter().enumerate() {
                assert!(
                    a.pos.x > MARGEN && a.pos.x < ARENA_W - MARGEN,
                    "{n} bailes: el {i} se sale de ancho"
                );
                assert!(
                    a.pos.y > MARGEN && a.pos.y < ARENA_H - MARGEN,
                    "{n} bailes: el {i} se sale de alto"
                );
                for b in &p.nodos[i + 1..] {
                    assert!(
                        (a.pos - b.pos).length() > RADIO_NODO * 2.0,
                        "{n} bailes: dos se solapan y no se podria elegir"
                    );
                }
            }
        }
    }

    #[test]
    fn se_entra_al_baile_acercandose() {
        let mut p = pista_de(1);
        assert_eq!(p.nodo_cerca(), None, "desde la entrada no se alcanza");
        for _ in 0..600 {
            p.step(ARRIBA);
            if p.nodo_cerca().is_some() {
                break;
            }
        }
        assert_eq!(p.nodo_cerca(), Some(0), "andando hacia el se llega");
    }

    #[test]
    fn ganar_marca_el_baile_y_termina_la_pista() {
        let mut p = pista_de(2);
        assert!(!p.todo_vencido());
        p.marcar_vencido(0);
        assert!(p.nodos[0].vencido);
        assert!(!p.todo_vencido(), "aun queda uno");
        p.marcar_vencido(1);
        assert!(p.todo_vencido());
    }

    #[test]
    fn una_pista_vacia_no_esta_terminada() {
        // Si no hay bailes no se ha ganado nada; sin esto, un RON que no cargue
        // daria la partida por completada.
        let p = Pista::new(Vec::new());
        assert!(!p.todo_vencido());
        assert_eq!(p.nodo_cerca(), None);
    }

    #[test]
    fn el_respeto_va_de_cero_a_uno_segun_lo_bailado() {
        let mut p = pista_de(4);
        assert_eq!(p.respeto(), 0.0, "nadie te ha visto bailar todavia");
        p.marcar_vencido(0);
        p.marcar_vencido(1);
        assert!((p.respeto() - 0.5).abs() < 1e-6);
        p.marcar_vencido(2);
        p.marcar_vencido(3);
        assert_eq!(p.respeto(), 1.0);
        // Una pista sin bailes no puede dar respeto, ni dividir por cero.
        assert_eq!(Pista::new(Vec::new()).respeto(), 0.0);
    }

    #[test]
    fn el_publico_se_vuelve_hacia_ti_cuando_te_respeta() {
        let mut p = pista_de(1);
        let m = p.publico[0].clone();
        assert_eq!(p.figura_de(&m).vel, Vec2::ZERO, "sin respeto no te mira");

        p.marcar_vencido(0);
        let vel = p.figura_de(&m).vel;
        assert!(vel.length() > 1.0, "deberia haberse vuelto");
        // Y vuelto hacia ella, no hacia cualquier lado.
        let hacia = (p.bailarina - m.pos).normalize_or_zero();
        assert!(vel.normalize_or_zero().dot(hacia) > 0.99);
    }

    #[test]
    fn el_publico_se_queda_a_los_lados() {
        // Si alguien se planta en el pasillo, la bailarina lo atraviesa al
        // andar hacia el fondo y se rompe la escena.
        let p = pista_de(1);
        assert_eq!(p.publico.len(), N_PUBLICO);
        for (i, m) in p.publico.iter().enumerate() {
            let desvio = (m.pos.x - ARENA_W * 0.5).abs();
            assert!(
                desvio > ARENA_W * 0.35,
                "el miron {i} esta en medio del paso"
            );
            assert!(m.talla > 0.5 && m.talla < 1.5, "talla rara: {}", m.talla);
        }
        // Y no todos con el mismo vaiven, que se veria como un cuerpo de baile.
        let primero = p.publico[0].desfase;
        assert!(p.publico.iter().any(|m| (m.desfase - primero).abs() > 0.5));
    }

    #[test]
    fn la_sala_se_monta_siempre_igual() {
        // Sale de un generador propio con semilla fija: dos partidas tienen la
        // misma sala, y las capturas de un dia valen para el siguiente.
        assert_eq!(pista_de(2).publico, pista_de(2).publico);
    }

    #[test]
    fn andar_es_determinista() {
        let guion = [DERECHA, ARRIBA, InputFrame::NONE, ARRIBA, DERECHA];
        let correr = || {
            let mut p = pista_de(3);
            for i in 0..200 {
                p.step(guion[i % guion.len()]);
            }
            p.bailarina
        };
        assert_eq!(correr(), correr());
    }
}
