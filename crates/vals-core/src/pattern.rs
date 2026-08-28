//! El lenguaje de patrones: un DSL declarativo compilado a bytecode.
//!
//! Un patron de jefe se escribe como un arbol de [`Step`] —legible, anidado y
//! serializable a RON— y se **compila** a una lista plana de [`Op`] con saltos.
//! El interprete ejecuta esa lista un tick cada vez.
//!
//! ¿Por que compilar en vez de recorrer el arbol? Porque el interprete tiene
//! que poder **pararse a mitad y reanudar** en el tick siguiente. Recorrer un
//! arbol recursivamente obligaria a guardar la pila de recursion entre ticks;
//! con bytecode, el estado que hay que guardar es un numero: el contador de
//! programa. Eso hace ademas que la ejecucion sea trivialmente serializable, lo
//! que importara cuando lleguen los replays.
//!
//! Cada patron corre varios **hilos** logicos a la vez (no hilos del sistema
//! operativo). `Parallel` los crea. Es lo que permite que el jefe se mueva
//! mientras dispara, o que dos ritmos de disparo distintos convivan.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::bullets::Bullets;
use crate::emitter::{Aim, EmitterSpec, fire};

/// Contador de un bucle infinito.
pub const LOOP_FOREVER: u32 = u32::MAX;

/// Tope de instrucciones por hilo y por tick.
///
/// Red de seguridad contra un patron mal escrito: un `Forever` sin ningun
/// `Wait` dentro se ejecutaria eternamente sin ceder el tick y colgaria el
/// juego. Con el tope, el patron se comporta raro pero el juego sigue vivo, que
/// es infinitamente mas facil de diagnosticar que un cuelgue.
const OP_BUDGET: u32 = 10_000;

/// Curva de interpolacion para los movimientos.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Ease {
    #[default]
    Linear,
    /// Arranca y frena suave. Es la que hace que un jefe parezca que pesa.
    InOut,
    /// Arranca rapido y frena. Buena para esquivar de golpe.
    Out,
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::InOut => t * t * (3.0 - 2.0 * t),
            Ease::Out => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

/// Un paso del patron, tal y como se escribe en el RON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Step {
    /// Espera `n` ticks. `Wait(0)` no hace nada.
    Wait(u32),
    /// Dispara una tanda.
    Fire(EmitterSpec),
    /// Gira la mira de este hilo `n` radianes, acumulativo.
    ///
    /// Es lo que convierte un anillo repetido en una espiral, que es como se
    /// construyen las espirales de verdad en este genero: no son una forma
    /// aparte, son un anillo cuyo angulo base avanza. Solo afecta a los
    /// emisores con angulo fijo; los apuntados al jugador siguen apuntando al
    /// jugador.
    Turn(f32),
    /// Repite el cuerpo un numero de veces.
    Repeat { times: u32, body: Vec<Step> },
    /// Repite el cuerpo hasta que acabe la fase.
    Forever(Vec<Step>),
    /// Lanza cada rama como un hilo aparte y sigue inmediatamente.
    Parallel(Vec<Vec<Step>>),
    /// Mueve al jefe. **Bloquea el hilo** durante `dur` ticks; si quieres que
    /// dispare mientras se mueve, mete el movimiento en un `Parallel`.
    MoveTo {
        pos: Vec2,
        dur: u32,
        #[serde(default)]
        ease: Ease,
    },
}

/// Instruccion compilada.
#[derive(Clone, Debug)]
enum Op {
    Wait(u32),
    /// Indice a la tabla de emisores del patron.
    Fire(u16),
    Turn(f32),
    MoveTo {
        pos: Vec2,
        dur: u32,
        ease: Ease,
    },
    /// Entra en un bucle. `end` es la direccion de salida.
    PushLoop {
        count: u32,
        end: u32,
    },
    /// Cierra una iteracion y vuelve a `start` si quedan vueltas.
    LoopBack {
        start: u32,
    },
    /// Arranca un hilo nuevo en `addr`.
    Fork(u32),
    Jump(u32),
    /// Termina este hilo.
    End,
}

/// Un patron compilado, listo para ejecutar.
#[derive(Clone, Debug)]
pub struct Pattern {
    ops: Vec<Op>,
    emitters: Vec<EmitterSpec>,
}

impl Pattern {
    /// Compila un arbol de pasos.
    pub fn compile(steps: &[Step]) -> Self {
        let mut p = Self {
            ops: Vec::new(),
            emitters: Vec::new(),
        };
        p.emit_steps(steps);
        p.ops.push(Op::End);
        p
    }

    pub fn op_count(&self) -> usize {
        self.ops.len()
    }

    fn emit_steps(&mut self, steps: &[Step]) {
        for s in steps {
            self.emit_step(s);
        }
    }

    fn emit_step(&mut self, step: &Step) {
        match step {
            Step::Wait(n) => self.ops.push(Op::Wait(*n)),

            Step::Turn(a) => self.ops.push(Op::Turn(*a)),

            Step::Fire(spec) => {
                let idx = self.emitters.len() as u16;
                self.emitters.push(*spec);
                self.ops.push(Op::Fire(idx));
            }

            Step::MoveTo { pos, dur, ease } => self.ops.push(Op::MoveTo {
                pos: *pos,
                dur: *dur,
                ease: *ease,
            }),

            Step::Repeat { times, body } => self.emit_loop(*times, body),
            Step::Forever(body) => self.emit_loop(LOOP_FOREVER, body),

            Step::Parallel(branches) => {
                // Layout: un Fork por rama, un Jump que salta todas, y luego
                // las ramas seguidas, cada una acabada en End.
                let forks: Vec<usize> = branches
                    .iter()
                    .map(|_| {
                        self.ops.push(Op::Fork(0));
                        self.ops.len() - 1
                    })
                    .collect();
                let jump = self.ops.len();
                self.ops.push(Op::Jump(0));

                for (fork_idx, branch) in forks.iter().zip(branches) {
                    let start = self.ops.len() as u32;
                    self.ops[*fork_idx] = Op::Fork(start);
                    self.emit_steps(branch);
                    self.ops.push(Op::End);
                }

                let after = self.ops.len() as u32;
                self.ops[jump] = Op::Jump(after);
            }
        }
    }

    fn emit_loop(&mut self, count: u32, body: &[Step]) {
        let head = self.ops.len();
        self.ops.push(Op::PushLoop { count, end: 0 });
        let body_start = self.ops.len() as u32;
        self.emit_steps(body);
        self.ops.push(Op::LoopBack { start: body_start });
        let end = self.ops.len() as u32;
        self.ops[head] = Op::PushLoop { count, end };
    }
}

#[derive(Clone, Debug)]
struct LoopFrame {
    remaining: u32,
    end: u32,
}

#[derive(Clone, Debug)]
struct Movement {
    from: Vec2,
    to: Vec2,
    dur: u32,
    elapsed: u32,
    ease: Ease,
}

#[derive(Clone, Debug)]
struct Thread {
    pc: u32,
    wait: u32,
    loops: Vec<LoopFrame>,
    /// Giro acumulado de la mira de este hilo. Es por hilo y no global para
    /// que dos espirales en `Parallel` puedan girar en sentidos opuestos.
    aim: f32,
    done: bool,
}

impl Thread {
    fn new(pc: u32) -> Self {
        Self {
            pc,
            wait: 0,
            loops: Vec::new(),
            aim: 0.0,
            done: false,
        }
    }
}

/// Lo que el interprete necesita del mundo para ejecutar un tick.
pub struct RunCtx<'a> {
    pub bullets: &'a mut Bullets,
    /// Posicion del jefe. `MoveTo` la escribe.
    pub origin: &'a mut Vec2,
    pub player: Vec2,
}

/// Ejecuta un [`Pattern`] a lo largo del tiempo.
#[derive(Clone, Debug)]
pub struct PatternRunner {
    pattern: Pattern,
    threads: Vec<Thread>,
    /// Movimiento en curso. Es unico: si dos hilos hacen `MoveTo` a la vez,
    /// manda el ultimo. Que dos hilos peleen por mover al jefe es un error de
    /// quien escribe el patron, no un caso que merezca la pena modelar.
    movement: Option<Movement>,
    ticks: u64,
}

impl PatternRunner {
    pub fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            threads: vec![Thread::new(0)],
            movement: None,
            ticks: 0,
        }
    }

    /// Vuelve a empezar el patron desde cero.
    pub fn restart(&mut self) {
        self.threads.clear();
        self.threads.push(Thread::new(0));
        self.movement = None;
        self.ticks = 0;
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Hilos logicos vivos. Util para depurar patrones.
    pub fn live_threads(&self) -> usize {
        self.threads.iter().filter(|t| !t.done).count()
    }

    /// `true` cuando todos los hilos han terminado.
    pub fn finished(&self) -> bool {
        self.threads.iter().all(|t| t.done)
    }

    /// Avanza el patron un tick.
    pub fn tick(&mut self, ctx: &mut RunCtx) {
        self.advance_movement(ctx.origin);

        // Indice y no iterador: un `Fork` puede anadir hilos a la lista
        // mientras la recorremos, y los nuevos deben ejecutarse ya en este
        // mismo tick.
        let mut i = 0;
        while i < self.threads.len() {
            self.run_thread(i, ctx);
            i += 1;
        }

        self.threads.retain(|t| !t.done);
        self.ticks += 1;
    }

    fn advance_movement(&mut self, origin: &mut Vec2) {
        let Some(m) = &mut self.movement else { return };
        m.elapsed += 1;
        let t = if m.dur == 0 {
            1.0
        } else {
            m.elapsed as f32 / m.dur as f32
        };
        *origin = m.from.lerp(m.to, m.ease.apply(t));
        if m.elapsed >= m.dur {
            self.movement = None;
        }
    }

    fn run_thread(&mut self, i: usize, ctx: &mut RunCtx) {
        if self.threads[i].done {
            return;
        }
        if self.threads[i].wait > 0 {
            self.threads[i].wait -= 1;
            return;
        }

        let mut budget = OP_BUDGET;
        loop {
            if budget == 0 {
                // Patron mal escrito. Se cede el tick para no colgar el juego.
                return;
            }
            budget -= 1;

            let pc = self.threads[i].pc as usize;
            let Some(op) = self.pattern.ops.get(pc).cloned() else {
                self.threads[i].done = true;
                return;
            };
            self.threads[i].pc += 1;

            match op {
                Op::End => {
                    self.threads[i].done = true;
                    return;
                }

                Op::Wait(n) => {
                    if n == 0 {
                        continue; // no-op: se sigue en el mismo tick
                    }
                    // `n - 1`: este tick ya cuenta como el primero de la
                    // espera, asi que `Wait(3)` deja exactamente 3 ticks entre
                    // la instruccion anterior y la siguiente.
                    self.threads[i].wait = n - 1;
                    return;
                }

                // Se envuelve en [0, TAU): un `Forever` con `Turn` dentro
                // acumularia angulo sin limite durante toda la partida, y la
                // precision de la reduccion de rango se degrada con el tamano.
                Op::Turn(a) => self.threads[i].aim = crate::math::wrap_tau(self.threads[i].aim + a),

                Op::Fire(idx) => {
                    if let Some(spec) = self.pattern.emitters.get(idx as usize) {
                        let mut spec = *spec;
                        if let Aim::Fixed(a) = spec.aim {
                            spec.aim = Aim::Fixed(a + self.threads[i].aim);
                        }
                        fire(ctx.bullets, *ctx.origin, &spec, ctx.player);
                    }
                }

                Op::MoveTo { pos, dur, ease } => {
                    self.movement = Some(Movement {
                        from: *ctx.origin,
                        to: pos,
                        dur,
                        elapsed: 0,
                        ease,
                    });
                    if dur == 0 {
                        *ctx.origin = pos;
                        self.movement = None;
                        continue;
                    }
                    self.threads[i].wait = dur - 1;
                    return;
                }

                Op::Jump(addr) => self.threads[i].pc = addr,

                Op::Fork(addr) => self.threads.push(Thread::new(addr)),

                Op::PushLoop { count, end } => {
                    if count == 0 {
                        self.threads[i].pc = end;
                        continue;
                    }
                    self.threads[i].loops.push(LoopFrame {
                        remaining: count,
                        end,
                    });
                }

                Op::LoopBack { start } => {
                    let Some(frame) = self.threads[i].loops.last_mut() else {
                        continue; // no deberia pasar; mejor seguir que romper
                    };
                    if frame.remaining != LOOP_FOREVER {
                        frame.remaining -= 1;
                    }
                    if frame.remaining == 0 {
                        let end = frame.end;
                        self.threads[i].loops.pop();
                        self.threads[i].pc = end;
                    } else {
                        self.threads[i].pc = start;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bullets::Bullets;
    use crate::emitter::EmitterSpec;

    fn un_disparo() -> Step {
        Step::Fire(EmitterSpec::ring(1, 100.0))
    }

    /// Ejecuta `n` ticks y devuelve en cuales salio alguna bala.
    fn ticks_con_disparo(steps: &[Step], n: u64) -> Vec<u64> {
        let pattern = Pattern::compile(steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(4096);
        let mut origin = Vec2::new(320.0, 130.0);
        let mut salida = Vec::new();

        for t in 0..n {
            let antes = bullets.live_count();
            let mut ctx = RunCtx {
                bullets: &mut bullets,
                origin: &mut origin,
                player: Vec2::new(320.0, 600.0),
            };
            runner.tick(&mut ctx);
            if bullets.live_count() > antes {
                salida.push(t);
            }
        }
        salida
    }

    #[test]
    fn wait_deja_exactamente_los_ticks_pedidos() {
        let steps = vec![
            un_disparo(),
            Step::Wait(3),
            un_disparo(),
            Step::Wait(3),
            un_disparo(),
        ];
        assert_eq!(ticks_con_disparo(&steps, 20), vec![0, 3, 6]);
    }

    #[test]
    fn wait_de_cero_no_consume_tick() {
        let steps = vec![un_disparo(), Step::Wait(0), un_disparo()];
        // Los dos disparos caen en el mismo tick, asi que solo se registra uno.
        assert_eq!(ticks_con_disparo(&steps, 5), vec![0]);
    }

    #[test]
    fn wait_de_uno_pasa_al_tick_siguiente() {
        let steps = vec![un_disparo(), Step::Wait(1), un_disparo()];
        assert_eq!(ticks_con_disparo(&steps, 5), vec![0, 1]);
    }

    #[test]
    fn repeat_ejecuta_el_cuerpo_las_veces_pedidas() {
        let steps = vec![Step::Repeat {
            times: 3,
            body: vec![un_disparo(), Step::Wait(2)],
        }];
        assert_eq!(ticks_con_disparo(&steps, 20), vec![0, 2, 4]);
    }

    #[test]
    fn repeat_de_cero_veces_se_salta_el_cuerpo() {
        let steps = vec![
            Step::Repeat {
                times: 0,
                body: vec![un_disparo()],
            },
            Step::Wait(1),
            un_disparo(),
        ];
        assert_eq!(ticks_con_disparo(&steps, 10), vec![1]);
    }

    #[test]
    fn repeat_anidado_funciona() {
        // 2 vueltas de (2 disparos separados 1 tick, y 2 de descanso).
        let steps = vec![Step::Repeat {
            times: 2,
            body: vec![
                Step::Repeat {
                    times: 2,
                    body: vec![un_disparo(), Step::Wait(1)],
                },
                Step::Wait(2),
            ],
        }];
        assert_eq!(ticks_con_disparo(&steps, 30), vec![0, 1, 4, 5]);
    }

    #[test]
    fn forever_no_para() {
        let steps = vec![Step::Forever(vec![un_disparo(), Step::Wait(5)])];
        let t = ticks_con_disparo(&steps, 31);
        assert_eq!(t, vec![0, 5, 10, 15, 20, 25, 30]);
    }

    #[test]
    fn un_forever_sin_wait_no_cuelga_el_juego() {
        // Patron mal escrito a proposito: sin `Wait` dentro del bucle.
        let steps = vec![Step::Forever(vec![un_disparo()])];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(64);
        let mut origin = Vec2::ZERO;
        let mut ctx = RunCtx {
            bullets: &mut bullets,
            origin: &mut origin,
            player: Vec2::ZERO,
        };
        // Si el presupuesto de instrucciones no existiera, esto no volveria.
        runner.tick(&mut ctx);
        assert!(runner.live_threads() > 0, "el hilo sigue vivo, solo cedio");
    }

    #[test]
    fn parallel_ejecuta_las_ramas_a_la_vez() {
        let steps = vec![Step::Parallel(vec![
            vec![Step::Forever(vec![un_disparo(), Step::Wait(2)])],
            vec![
                Step::Wait(1),
                Step::Forever(vec![un_disparo(), Step::Wait(3)]),
            ],
        ])];
        let t = ticks_con_disparo(&steps, 13);
        // Rama A en 0,2,4,6,8,10,12; rama B en 1,4,7,10. La union, sin repetir.
        assert_eq!(t, vec![0, 1, 2, 4, 6, 7, 8, 10, 12]);
    }

    #[test]
    fn parallel_arranca_las_ramas_en_el_mismo_tick() {
        let steps = vec![Step::Parallel(vec![vec![un_disparo()], vec![un_disparo()]])];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(64);
        let mut origin = Vec2::ZERO;
        let mut ctx = RunCtx {
            bullets: &mut bullets,
            origin: &mut origin,
            player: Vec2::new(0.0, 100.0),
        };
        runner.tick(&mut ctx);
        assert_eq!(bullets.live_count(), 2, "las dos ramas en el tick 0");
    }

    #[test]
    fn el_padre_sigue_despues_del_parallel() {
        let steps = vec![Step::Parallel(vec![vec![Step::Wait(10)]]), un_disparo()];
        // El padre no espera a la rama: dispara en el tick 0.
        assert_eq!(ticks_con_disparo(&steps, 5), vec![0]);
    }

    #[test]
    fn moveto_interpola_y_bloquea_su_hilo() {
        let steps = vec![
            Step::MoveTo {
                pos: Vec2::new(100.0, 100.0),
                dur: 4,
                ease: Ease::Linear,
            },
            un_disparo(),
        ];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(64);
        let mut origin = Vec2::new(0.0, 0.0);
        let mut posiciones = Vec::new();

        for _ in 0..5 {
            let mut ctx = RunCtx {
                bullets: &mut bullets,
                origin: &mut origin,
                player: Vec2::ZERO,
            };
            runner.tick(&mut ctx);
            posiciones.push(origin);
        }

        // Coherente con `Wait(4)`: el hilo se reanuda en el tick 4, y es
        // justo el tick en el que el movimiento termina. El jefe llega y
        // dispara en el mismo frame.
        assert!(
            (posiciones[4] - Vec2::new(100.0, 100.0)).length() < 0.001,
            "deberia llegar en el tick 4, esta en {}",
            posiciones[4]
        );
        assert!(
            posiciones[1].x > 0.0 && posiciones[1].x < 100.0,
            "y por el camino deberia interpolar, no teletransportarse"
        );
        assert_eq!(bullets.live_count(), 1, "dispara al llegar");
    }

    #[test]
    fn moveto_de_duracion_cero_teletransporta() {
        let steps = vec![
            Step::MoveTo {
                pos: Vec2::new(50.0, 50.0),
                dur: 0,
                ease: Ease::Linear,
            },
            un_disparo(),
        ];
        // Sin consumir tick: el disparo sale en el 0.
        assert_eq!(ticks_con_disparo(&steps, 3), vec![0]);
    }

    #[test]
    fn el_patron_termina_cuando_se_acaban_los_pasos() {
        let steps = vec![un_disparo()];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(64);
        let mut origin = Vec2::ZERO;
        for _ in 0..3 {
            let mut ctx = RunCtx {
                bullets: &mut bullets,
                origin: &mut origin,
                player: Vec2::ZERO,
            };
            runner.tick(&mut ctx);
        }
        assert!(runner.finished());
    }

    #[test]
    fn restart_vuelve_a_empezar() {
        let steps = vec![un_disparo(), Step::Wait(100), un_disparo()];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(64);
        let mut origin = Vec2::ZERO;

        let mut disparar = |runner: &mut PatternRunner, bullets: &mut Bullets| {
            let antes = bullets.live_count();
            let mut ctx = RunCtx {
                bullets,
                origin: &mut origin,
                player: Vec2::ZERO,
            };
            runner.tick(&mut ctx);
            ctx.bullets.live_count() > antes
        };

        assert!(disparar(&mut runner, &mut bullets));
        assert!(!disparar(&mut runner, &mut bullets)); // esperando
        runner.restart();
        assert!(
            disparar(&mut runner, &mut bullets),
            "tras restart, dispara ya"
        );
    }

    #[test]
    fn turn_convierte_un_anillo_repetido_en_espiral() {
        let steps = vec![Step::Repeat {
            times: 4,
            body: vec![
                Step::Fire(EmitterSpec::ring(1, 100.0).at_angle(0.0)),
                Step::Turn(0.5),
                Step::Wait(1),
            ],
        }];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(64);
        let mut origin = Vec2::ZERO;
        for _ in 0..6 {
            let mut ctx = RunCtx {
                bullets: &mut bullets,
                origin: &mut origin,
                player: Vec2::ZERO,
            };
            runner.tick(&mut ctx);
        }

        let mut angulos: Vec<f32> = bullets.iter_live().map(|v| v.vel.to_angle()).collect();
        angulos.sort_by(f32::total_cmp);
        assert_eq!(angulos.len(), 4);
        for par in angulos.windows(2) {
            assert!(
                (par[1] - par[0] - 0.5).abs() < 0.001,
                "cada disparo deberia ir 0,5 rad girado: {par:?}"
            );
        }
    }

    #[test]
    fn turn_es_por_hilo_para_poder_girar_en_sentidos_opuestos() {
        let espiral = |paso: f32| {
            vec![Step::Repeat {
                times: 3,
                body: vec![
                    Step::Fire(EmitterSpec::ring(1, 100.0).at_angle(0.0)),
                    Step::Turn(paso),
                    Step::Wait(1),
                ],
            }]
        };
        let steps = vec![Step::Parallel(vec![espiral(0.4), espiral(-0.4)])];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(64);
        let mut origin = Vec2::ZERO;
        for _ in 0..6 {
            let mut ctx = RunCtx {
                bullets: &mut bullets,
                origin: &mut origin,
                player: Vec2::ZERO,
            };
            runner.tick(&mut ctx);
        }
        let angulos: Vec<f32> = bullets.iter_live().map(|v| v.vel.to_angle()).collect();
        assert!(
            angulos.iter().any(|a| *a > 0.3),
            "una rama gira en positivo"
        );
        assert!(angulos.iter().any(|a| *a < -0.3), "y la otra en negativo");
    }

    #[test]
    fn turn_no_desvia_los_emisores_apuntados_al_jugador() {
        let steps = vec![
            Step::Turn(1.5),
            Step::Fire(EmitterSpec::fan(1, 0.0, 100.0).aimed()),
        ];
        let pattern = Pattern::compile(&steps);
        let mut runner = PatternRunner::new(pattern);
        let mut bullets = Bullets::with_capacity(16);
        let mut origin = Vec2::new(0.0, 0.0);
        let jugador = Vec2::new(0.0, 100.0);
        let mut ctx = RunCtx {
            bullets: &mut bullets,
            origin: &mut origin,
            player: jugador,
        };
        runner.tick(&mut ctx);
        let v = bullets.iter_live().next().unwrap();
        assert!(
            (v.vel.normalize() - Vec2::new(0.0, 1.0)).length() < 0.001,
            "apuntado sigue significando apuntado"
        );
    }

    #[test]
    fn las_curvas_de_easing_van_de_cero_a_uno() {
        for e in [Ease::Linear, Ease::InOut, Ease::Out] {
            assert!((e.apply(0.0)).abs() < 1e-6, "{e:?} en 0");
            assert!((e.apply(1.0) - 1.0).abs() < 1e-6, "{e:?} en 1");
            assert!(e.apply(0.5) > 0.0 && e.apply(0.5) < 1.0);
        }
        // Fuera de rango se recorta en vez de dispararse.
        assert_eq!(Ease::Linear.apply(2.0), 1.0);
        assert_eq!(Ease::Linear.apply(-1.0), 0.0);
    }
}
