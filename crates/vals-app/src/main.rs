//! Punto de entrada de VALS.
//!
//! Este crate se ocupa de ventana, input, render y reloj. La simulacion vive
//! entera en `vals-core` y no sabe que esto existe.

use macroquad::prelude::*;
use vals_core::bench::Stress;
use vals_core::replay::{GOLDEN_REPLAY, Replay};
use vals_core::{DT, Events, InputFrame, MAX_BULLETS, Mode, Recorder, World};

mod audio;
mod bullet_renderer;
mod draw;
mod hot;
mod music;
mod replay_io;
mod skeleton;
mod stats;

use audio::{Audio, Sfx};
use bullet_renderer::BulletRenderer;
use hot::HotReload;
use music::Baile;
use stats::FrameStats;

/// "VALS" en ASCII. Semilla por defecto.
const SEED: u64 = 0x5641_4C53;

/// Cada cuantos frames se mira si cambio el fichero del patron.
///
/// Tres veces por segundo: imperceptible al guardar, e irrelevante para el
/// frame time comparado con dibujar un millar de balas.
const HOT_RELOAD_EVERY: u32 = 20;

/// Balas de la escena de stress si no se pide otra cosa.
const BENCH_DEFAULT: usize = 10_000;

/// Maximo de ticks de simulacion por frame.
///
/// Evita la espiral de la muerte: si un frame tarda muchisimo (un breakpoint,
/// la ventana minimizada, el navegador en otra pestana), preferimos que el
/// tiempo del juego se ralentice antes que encadenar simulaciones cada vez mas
/// largas hasta no recuperarnos nunca.
const MAX_STEPS_PER_FRAME: u32 = 5;

/// Techo del delta que aceptamos del sistema, en segundos.
const MAX_FRAME_DT: f32 = 0.25;

fn window_conf() -> Conf {
    Conf {
        window_title: "VALS".to_owned(),
        window_width: 1100,
        window_height: 860,
        high_dpi: true,
        ..Default::default()
    }
}

/// Si se usa el render instanciado o el viejo de macroquad.
///
/// El viejo se conserva a proposito tras `--legacy-render`: es lo que hace que
/// la comparacion de `docs/PERF.md` se pueda repetir dentro de un ano en vez de
/// ser una cifra que hay que creerse.
fn legacy_render() -> bool {
    std::env::args().any(|a| a == "--legacy-render")
}

fn nuevo_renderer() -> Option<BulletRenderer> {
    (!legacy_render()).then(|| BulletRenderer::new(MAX_BULLETS))
}

/// Como arranca la app desde la linea de comandos.
///
/// Se llama `Arranque` y no `Mode` para no chocar con `vals_core::Mode`, que es
/// el modo de juego (vuelo o plataformas) y es el concepto de dominio.
enum Arranque {
    Game,
    /// Escena de stress. `frames` mide y sale, para que las cifras de PERF.md
    /// salgan de un comando repetible.
    Bench {
        target: usize,
        frames: Option<u32>,
    },
    /// Reproduce un replay guardado.
    Replay(String),
}

fn parse_mode() -> Arranque {
    let args: Vec<String> = std::env::args().collect();
    let valor = |flag: &str| -> Option<&String> {
        let i = args.iter().position(|a| a == flag)?;
        args.get(i + 1)
    };

    if let Some(p) = valor("--replay") {
        return Arranque::Replay(p.clone());
    }
    if let Some(i) = args.iter().position(|a| a == "--bench-scene") {
        return Arranque::Bench {
            target: args
                .get(i + 1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(BENCH_DEFAULT),
            frames: valor("--frames").and_then(|s| s.parse().ok()),
        };
    }
    Arranque::Game
}

#[macroquad::main(window_conf)]
async fn main() {
    match parse_mode() {
        Arranque::Game => run_game().await,
        Arranque::Bench { target, frames } => run_bench(target, frames).await,
        Arranque::Replay(path) => run_replay(path).await,
    }
}

/// Modo atractor: el replay dorado corriendo de fondo en el menu.
///
/// Sale casi gratis. El replay va embebido en el binario, asi que funciona
/// tambien en web, y reproducirlo es exactamente el mismo bucle que jugar.
struct Attract {
    world: World,
    replay: Replay,
    cursor: usize,
}

impl Attract {
    fn new() -> Option<Self> {
        let replay = Replay::from_bytes(GOLDEN_REPLAY).ok()?;
        Some(Self {
            world: World::new(replay.seed),
            replay,
            cursor: 0,
        })
    }

    fn step(&mut self) {
        match self.replay.inputs.get(self.cursor) {
            Some(input) => {
                self.world.step(*input);
                self.cursor += 1;
            }
            // Al acabarse, vuelve a empezar: es un fondo, no una partida.
            None => {
                self.world = World::new(self.replay.seed);
                self.cursor = 0;
            }
        }
    }
}

async fn run_game() {
    let mut world = World::new(SEED);
    // Se graba siempre. Cuesta dos bytes por tick, asi que no hay ningun motivo
    // para pedirlo: cuando pasa algo digno de guardar, ya es tarde para
    // haberle dado a grabar.
    let mut recorder = Recorder::for_world(&world);
    let mut stats = FrameStats::new();
    let mut accumulator = 0.0f32;
    let mut show_debug = true;
    let mut hot = HotReload::new();
    let mut frames: u32 = 0;
    let mut aviso: Option<(String, bool, u32)> = None;
    let mut bullets_gpu = nuevo_renderer();
    let mut attract = Attract::new();
    let mut en_menu = true;
    let mut intentos: u32 = 0;
    let mut audio = Audio::load().await;
    // Los navegadores no dejan sonar nada hasta que el usuario toca algo. Si la
    // musica arrancase sola, en web el menu saldria mudo y no se sabria por
    // que. Se espera a la primera tecla, que ademas es cuando el jugador esta
    // mirando.
    let mut hubo_interaccion = false;

    loop {
        frames += 1;
        if frames.is_multiple_of(HOT_RELOAD_EVERY)
            && let Some(defs) = hot.poll()
        {
            world.reload_bosses(defs);
            // El replay en curso ya no reproduce nada: el jefe ha cambiado.
            recorder = Recorder::for_world(&world);
        }

        hubo_interaccion |= get_last_key_pressed().is_some();
        if hubo_interaccion {
            // El tema lo decide el jefe que toca. En el menu suena el del
            // primero, que es el que corre de fondo en el atractor.
            audio.poner_musica(Baile::del_jefe(if en_menu { 0 } else { world.boss_index }));
        }

        let frame_dt = get_frame_time().min(MAX_FRAME_DT);
        stats.push_frame(frame_dt);

        if is_key_pressed(KeyCode::F1) {
            show_debug = !show_debug;
        }
        if is_key_pressed(KeyCode::R) && !en_menu {
            // Tras perder se reintenta **este** jefe con las vidas llenas; en
            // cualquier otro momento, partida nueva desde el principio.
            if world.defeat {
                world.retry_current_boss();
            } else {
                world = World::with_mode(SEED, world.mode);
            }
            recorder = Recorder::for_world(&world);
            accumulator = 0.0;
            intentos += 1;
        }
        if is_key_pressed(KeyCode::Escape) && !en_menu {
            en_menu = true;
        }
        if is_key_pressed(KeyCode::M) {
            audio.toggle_mute();
        }
        if en_menu {
            let modo = if is_key_pressed(KeyCode::Z) {
                Some(Mode::Flight)
            } else if is_key_pressed(KeyCode::X) {
                Some(Mode::Platform)
            } else {
                None
            };
            if let Some(modo) = modo {
                audio.play(Sfx::Empezar, 1.0);
                en_menu = false;
                world = World::with_mode(SEED, modo);
                recorder = Recorder::for_world(&world);
                accumulator = 0.0;
                intentos += 1;
            }
        }
        if is_key_pressed(KeyCode::F2) {
            aviso = Some(match guardar_replay(&recorder) {
                Ok(m) => (m, false, 240),
                Err(m) => (m, true, 240),
            });
        }

        // --- Simulacion: paso fijo, desacoplada del render ---
        // En el menu se avanza el modo atractor en vez de la partida; el
        // acumulador es el mismo, asi que el replay va al ritmo correcto.
        let input = if en_menu {
            InputFrame::NONE
        } else {
            read_input()
        };
        let t0 = get_time();
        accumulator += frame_dt;
        let mut steps = 0;
        // Los sucesos de todos los ticks del frame se juntan: puede haber
        // varios, y reaccionar solo al ultimo se comeria sonidos.
        let mut eventos = Events::default();
        while accumulator >= DT {
            if en_menu {
                if let Some(a) = attract.as_mut() {
                    a.step();
                }
            } else {
                world.step(input);
                recorder.record(input, &world);
                eventos.merge(&world.events);
            }
            accumulator -= DT;
            steps += 1;
            if steps >= MAX_STEPS_PER_FRAME {
                accumulator = 0.0;
                break;
            }
        }
        stats.push_sim((get_time() - t0) as f32);
        // En el menu no suena nada: el atractor es un fondo, no una partida.
        if !en_menu {
            audio.play_events(&eventos);
        }

        // Fraccion de tick pendiente. Es lo que permite que el render vaya a
        // 144 Hz con la simulacion a 60 sin que se vea a saltos.
        let alpha = (accumulator / DT).clamp(0.0, 1.0);

        // --- Render ---
        let t1 = get_time();
        let layout = draw::Layout::compute();
        let mostrado = match (en_menu, attract.as_ref()) {
            (true, Some(a)) => &a.world,
            _ => &world,
        };
        draw::frame(mostrado, alpha, &layout, bullets_gpu.as_mut());
        if en_menu {
            draw::menu(&layout, intentos);
        } else if world.is_over() {
            draw::fin_de_partida(&layout, &world);
        }
        if show_debug && !en_menu {
            draw::debug_overlay(&world, &stats, steps);
            draw::recording_badge(recorder.ticks());
        }
        if let Some((msg, error)) = hot.aviso() {
            draw::banner(msg, error);
        } else if let Some((msg, error, restantes)) = &mut aviso {
            draw::banner(msg, *error);
            *restantes = restantes.saturating_sub(1);
            if *restantes == 0 {
                aviso = None;
            }
        }
        stats.push_render((get_time() - t1) as f32);

        next_frame().await;
    }
}

fn guardar_replay(recorder: &Recorder) -> Result<String, String> {
    if recorder.is_empty() {
        return Err("todavia no hay nada grabado".to_owned());
    }
    let bytes = recorder.replay().to_bytes();
    replay_io::save(&bytes).map(|ruta| format!("replay guardado en {ruta} ({} bytes)", bytes.len()))
}

/// Reproduce un replay guardado.
///
/// Ademas de verlo, comprueba las huellas sobre la marcha: si la simulacion se
/// sale del guion, se dice en pantalla y en que tick. Es la version visual de
/// lo que hace el test del replay dorado.
async fn run_replay(path: String) {
    let replay = match replay_io::load(&path)
        .and_then(|b| Replay::from_bytes(&b).map_err(|e| format!("{path}: {e}")))
    {
        Ok(r) => r,
        Err(e) => {
            println!("{e}");
            // Sin replay no hay nada que ensenar; se sale en vez de dejar una
            // ventana negra sin explicacion.
            std::process::exit(1);
        }
    };

    println!(
        "reproduciendo {path}: {} ticks ({:.1} s), semilla {:#018x}",
        replay.ticks(),
        replay.seconds(),
        replay.seed
    );

    let mut world = World::new(replay.seed);
    let mut stats = FrameStats::new();
    let mut accumulator = 0.0f32;
    let mut cursor = 0usize;
    let mut siguiente_huella = 0usize;
    let mut divergencia: Option<u64> = None;
    let mut bullets_gpu = nuevo_renderer();

    loop {
        let frame_dt = get_frame_time().min(MAX_FRAME_DT);
        stats.push_frame(frame_dt);

        if is_key_pressed(KeyCode::R) {
            world = World::new(replay.seed);
            cursor = 0;
            siguiente_huella = 0;
            divergencia = None;
            accumulator = 0.0;
        }

        let t0 = get_time();
        accumulator += frame_dt;
        let mut steps = 0;
        while accumulator >= DT {
            match replay.inputs.get(cursor) {
                Some(input) => {
                    world.step(*input);
                    cursor += 1;
                    if let Some(cp) = replay.checkpoints.get(siguiente_huella)
                        && cp.tick == world.tick
                    {
                        if world.state_hash() != cp.hash && divergencia.is_none() {
                            divergencia = Some(world.tick);
                            println!("DIVERGENCIA en el tick {}", world.tick);
                        }
                        siguiente_huella += 1;
                    }
                }
                // Se acabo el replay: se congela en el ultimo frame.
                None => accumulator = 0.0,
            }
            accumulator -= DT;
            steps += 1;
            if steps >= MAX_STEPS_PER_FRAME {
                accumulator = 0.0;
                break;
            }
        }
        stats.push_sim((get_time() - t0) as f32);
        let alpha = (accumulator / DT).clamp(0.0, 1.0);

        let t1 = get_time();
        let layout = draw::Layout::compute();
        draw::frame(&world, alpha, &layout, bullets_gpu.as_mut());
        draw::debug_overlay(&world, &stats, steps);
        draw::replay_badge(cursor, replay.inputs.len(), divergencia);
        stats.push_render((get_time() - t1) as f32);

        next_frame().await;
    }
}

/// Escena de stress: la misma que miden los benchmarks, pero con render.
///
/// No hay jugador ni input. Sirve para ver a ojo cuanto aguanta el render y
/// para sacar las cifras que van a `docs/PERF.md`.
async fn run_bench(target: usize, limite: Option<u32>) {
    let mut stress = Stress::new(target);
    let mut stats = FrameStats::new();
    let mut accumulator = 0.0f32;
    let mut frames: u32 = 0;
    let mut bullets_gpu = nuevo_renderer();
    // Frames que no se cuentan: los primeros siempre incluyen la compilacion
    // de shaders y el llenado inicial del pool, y falsearian el p99.
    let calentamiento = 60;

    loop {
        let frame_dt = get_frame_time().min(MAX_FRAME_DT);
        stats.push_frame(frame_dt);

        let t0 = get_time();
        accumulator += frame_dt;
        let mut steps = 0;
        while accumulator >= DT {
            stress.step(DT);
            accumulator -= DT;
            steps += 1;
            if steps >= MAX_STEPS_PER_FRAME {
                accumulator = 0.0;
                break;
            }
        }
        stats.push_sim((get_time() - t0) as f32);

        let t1 = get_time();
        let layout = draw::Layout::compute();
        clear_background(BLACK);
        match bullets_gpu.as_mut() {
            Some(r) => r.draw(
                &stress.bullets,
                &layout,
                draw::pulse(frames as f32),
                draw::bullet_style,
            ),
            None => draw::draw_bullets(&stress.bullets, &layout),
        }
        draw::bench_overlay(
            &stats,
            &stress.bullets,
            stress.target(),
            steps,
            bullets_gpu.as_ref(),
        );
        stats.push_render((get_time() - t1) as f32);

        frames += 1;
        if let Some(limite) = limite
            && frames >= limite + calentamiento
        {
            println!(
                "balas={} objetivo={target} render={}",
                stress.live_count(),
                if legacy_render() {
                    "macroquad"
                } else {
                    "instanciado"
                }
            );
            println!(
                "p50={:.3}ms p99={:.3}ms sim={:.3}ms draw={:.3}ms fps={:.1}",
                stats.p50_ms(),
                stats.p99_ms(),
                stats.sim_ms(),
                stats.render_ms(),
                stats.fps()
            );
            std::process::exit(0);
        }
        if frames == calentamiento {
            stats = FrameStats::new();
        }

        next_frame().await;
    }
}

/// Traduce el teclado a un `InputFrame`.
///
/// Es el unico sitio donde se mapean teclas: la simulacion solo ve bits, asi
/// que anadir mando o rebindeo mas adelante no toca nada del juego.
fn read_input() -> InputFrame {
    let mut f = InputFrame::NONE;
    f.set(
        InputFrame::UP,
        is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
    );
    f.set(
        InputFrame::DOWN,
        is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
    );
    f.set(
        InputFrame::LEFT,
        is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
    );
    f.set(
        InputFrame::RIGHT,
        is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
    );
    f.set(
        InputFrame::FOCUS,
        is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
    );
    // El salto reutiliza arriba: en el modo con gravedad el eje vertical no
    // mueve, asi que la tecla queda libre para lo que toca en un plataformas.
    f.set(
        InputFrame::JUMP,
        is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) || is_key_down(KeyCode::K),
    );
    f.set(InputFrame::SHOOT, is_key_down(KeyCode::Z));
    f.set(InputFrame::DASH, is_key_down(KeyCode::X));
    f.set(InputFrame::PARRY, is_key_down(KeyCode::C));
    f.set(
        InputFrame::SUPER,
        is_key_down(KeyCode::Space) || is_key_down(KeyCode::V),
    );
    f
}
