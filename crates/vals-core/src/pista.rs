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

use crate::boss::Nivel;
use crate::input::InputFrame;
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

/// Un baile plantado en la pista.
#[derive(Clone, Debug, PartialEq)]
pub struct Nodo {
    pub nombre: String,
    pub nivel: Nivel,
    /// Cual de los bailes de `DEFAULT_BOSS_RONS` es.
    ///
    /// **No es su sitio en la pista.** La pista los reordena por nivel, asi que
    /// el segundo nodo que ves puede ser el tercer fichero. Confundir los dos
    /// indices hacia que entrar al charleston te metiera en el tango.
    pub jefe: usize,
    pub pos: Vec2,
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
    pub tick: u64,
}

impl Pista {
    /// Una pista con un baile por cada (nombre, nivel) recibido.
    ///
    /// Los sitios salen del **nivel**, no del orden de la lista: lo facil
    /// delante y lo dificil al fondo. Asi el orden en que se escribieron los
    /// ficheros deja de decidir por donde empiezas.
    pub fn new(bailes: Vec<(String, Nivel)>) -> Self {
        let mut nodos: Vec<Nodo> = Vec::with_capacity(bailes.len());
        for nivel in Nivel::TODOS {
            let fila: Vec<usize> = (0..bailes.len())
                .filter(|i| bailes[*i].1 == nivel)
                .collect();
            for (sitio_en_fila, i) in fila.iter().enumerate() {
                nodos.push(Nodo {
                    nombre: bailes[*i].0.clone(),
                    nivel,
                    jefe: *i,
                    pos: sitio(nivel, sitio_en_fila, fila.len()),
                    vencido: false,
                });
            }
        }
        Self {
            bailarina: ENTRADA,
            prev: ENTRADA,
            vel: Vec2::ZERO,
            nodos,
            tick: 0,
        }
    }

    /// Cuanto te respeta la sala, en `[0, 1]`: la fraccion de bailes sacados.
    ///
    /// Es lo que hace que la bailarina baile mejor segun avanza. Hubo tambien
    /// publico a los lados moviendose con este mismo numero, y se quito: no
    /// aportaba lo bastante para el sitio que ocupaba. El numero se queda
    /// porque lo que si funcionaba era la bailarina.
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

    /// Si se puede entrar a un nivel.
    ///
    /// Se abre cuando estan hechos **todos** los del nivel anterior. Es lo que
    /// arregla que el tango apareciese el segundo siendo el mas duro: no se
    /// hace mas facil, deja de estar ahi delante desde el principio.
    ///
    /// Un nivel sin bailes es transparente: no bloquea nada, porque "todos" de
    /// una lista vacia se cumple solo.
    pub fn nivel_abierto(&self, nivel: Nivel) -> bool {
        self.nodos
            .iter()
            .filter(|n| n.nivel < nivel)
            .all(|n| n.vencido)
    }

    /// Si se puede entrar a este baile.
    pub fn abierto(&self, i: usize) -> bool {
        self.nodos
            .get(i)
            .is_some_and(|n| self.nivel_abierto(n.nivel))
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

    /// Posicion para dibujar, interpolada entre el tick anterior y este.
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev.lerp(self.bailarina, alpha)
    }
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

/// A que profundidad de la pista se planta cada nivel.
///
/// Lo facil delante y lo dificil al fondo: la sala se lee de un vistazo y andar
/// hacia dentro es andar hacia lo duro.
fn fondo(nivel: Nivel) -> f32 {
    ARENA_H
        * match nivel {
            Nivel::Facil => 0.70,
            Nivel::Media => 0.52,
            Nivel::Dificil => 0.34,
            Nivel::Final => 0.17,
        }
}

/// Donde se planta un baile dentro de la fila de su nivel.
///
/// Los de indice impar se apartan un poco hacia el fondo: en fila recta se leen
/// como una lista, y en zigzag se leen como gente en una sala. Ademas separa lo
/// justo para que no se solapen cuando la fila se llena.
fn sitio(nivel: Nivel, i: usize, n: usize) -> Vec2 {
    let y = fondo(nivel);
    if n <= 1 {
        return vec2(ARENA_W * 0.5, y);
    }
    let hueco = ARENA_W - 2.0 * MARGEN - 2.0 * RADIO_NODO;
    // El tope era 160 cuando cada baile era un emblema de sesenta de ancho.
    // Desde que es un monumento (`mapa.rs` en la app) hace falta mas aire entre
    // los dos de una fila, o se tapan uno al otro con la perspectiva.
    let paso = (hueco / (n - 1) as f32).min(220.0);
    let x = ARENA_W * 0.5 + (i as f32 - (n - 1) as f32 * 0.5) * paso;
    let zigzag = if i % 2 == 1 { -40.0 } else { 0.0 };
    vec2(x, y + zigzag)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una pista con `n` bailes repartidos por los cuatro niveles, que es como
    /// se van a repartir de verdad.
    fn pista_de(n: usize) -> Pista {
        Pista::new(
            (0..n)
                .map(|i| {
                    let nivel = Nivel::TODOS[i * Nivel::TODOS.len() / n.max(1)];
                    (format!("Baile {}", i + 1), nivel)
                })
                .collect(),
        )
    }

    /// Y una con todos en el mismo nivel, para probar que una fila llena cabe.
    fn fila_de(n: usize) -> Pista {
        Pista::new(
            (0..n)
                .map(|i| (format!("Baile {}", i + 1), Nivel::Facil))
                .collect(),
        )
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
    fn lo_facil_esta_delante_y_lo_dificil_al_fondo() {
        // La pista se lee de un vistazo: andar hacia dentro es andar hacia lo
        // duro. Antes esto salia del orden del fichero, que no queria decir
        // nada; ahora sale del nivel que declara cada baile.
        let p = pista_de(8);
        for a in &p.nodos {
            for b in &p.nodos {
                if a.nivel < b.nivel {
                    assert!(
                        a.pos.y > b.pos.y,
                        "{} ({}) deberia estar mas cerca que {} ({})",
                        a.nombre,
                        a.nivel.nombre(),
                        b.nombre,
                        b.nivel.nombre()
                    );
                }
            }
        }
        // Y lo mas facil es lo que pillas al entrar.
        let cerca = |n: &Nodo| (n.pos - ENTRADA).length();
        let primero = p.nodos.iter().min_by(|a, b| cerca(a).total_cmp(&cerca(b)));
        assert_eq!(primero.unwrap().nivel, Nivel::Facil);
    }

    #[test]
    fn cada_nodo_sabe_que_jefe_es() {
        // La pista reordena por nivel, asi que el sitio en la pista y el sitio
        // en la lista de ficheros son cosas distintas. Confundirlos hacia que
        // entrar al charleston te metiera en el tango.
        let p = Pista::new(vec![
            ("Vals".into(), Nivel::Facil),
            ("Tango".into(), Nivel::Dificil),
            ("Charleston".into(), Nivel::Media),
        ]);
        let por_nombre = |n: &str| p.nodos.iter().find(|x| x.nombre == n).unwrap();
        assert_eq!(por_nombre("Vals").jefe, 0);
        assert_eq!(por_nombre("Tango").jefe, 1);
        assert_eq!(por_nombre("Charleston").jefe, 2);
        // Y el orden en la pista NO es el de los ficheros.
        assert_eq!(
            p.nodos[1].nombre, "Charleston",
            "el segundo nodo es el medio"
        );
        assert_ne!(p.nodos[1].jefe, 1, "pero no es el segundo fichero");
    }

    #[test]
    fn un_nivel_se_abre_cuando_esta_hecho_el_anterior() {
        // Es lo que arregla que el tango, siendo el mas duro, apareciese el
        // segundo. No se hace mas facil: deja de estar ahi desde el principio.
        let mut p = Pista::new(vec![
            ("Vals".into(), Nivel::Facil),
            ("Charleston".into(), Nivel::Media),
            ("Tango".into(), Nivel::Dificil),
        ]);
        assert!(
            p.nivel_abierto(Nivel::Facil),
            "lo facil siempre esta abierto"
        );
        assert!(!p.nivel_abierto(Nivel::Media));
        assert!(!p.nivel_abierto(Nivel::Dificil));

        let vals = p.nodos.iter().position(|n| n.nombre == "Vals").unwrap();
        p.marcar_vencido(vals);
        assert!(
            p.nivel_abierto(Nivel::Media),
            "cayo el vals, entra el medio"
        );
        assert!(!p.nivel_abierto(Nivel::Dificil));

        let ch = p
            .nodos
            .iter()
            .position(|n| n.nombre == "Charleston")
            .unwrap();
        p.marcar_vencido(ch);
        assert!(p.nivel_abierto(Nivel::Dificil));
        assert!(p.nodos.iter().enumerate().all(|(i, _)| p.abierto(i)));
    }

    #[test]
    fn un_nivel_vacio_no_bloquea_nada() {
        // Si un dia hay bailes dificiles y ninguno medio, el hueco no puede
        // dejar el juego cerrado para siempre.
        let mut p = Pista::new(vec![
            ("Vals".into(), Nivel::Facil),
            ("Tango".into(), Nivel::Dificil),
        ]);
        assert!(!p.nivel_abierto(Nivel::Dificil));
        p.marcar_vencido(0);
        assert!(p.nivel_abierto(Nivel::Dificil), "el nivel medio esta vacio");
    }

    #[test]
    fn un_solo_baile_se_planta_en_medio() {
        let p = fila_de(1);
        assert_eq!(p.nodos.len(), 1);
        assert!((p.nodos[0].pos.x - ARENA_W * 0.5).abs() < 0.01);
    }

    #[test]
    fn varios_bailes_no_se_pisan_y_caben_en_la_pista() {
        // Cuatro por fila es lo que se espera como mucho: con ocho bailes en
        // cuatro niveles salen a dos por fila.
        for n in 2..=4 {
            let p = fila_de(n);
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
