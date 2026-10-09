//! Punto de entrada de VALS.
//!
//! Este crate se ocupa de ventana, input, render y reloj. La simulacion vive
//! entera en `vals-core` y no sabe que esto existe.

use macroquad::prelude::*;
use vals_core::bench::Stress;
use vals_core::boss::BossDef;
use vals_core::paseo::{Paseo, PaseoDef, VISTA_H, VISTA_W};
use vals_core::pista::Pista;
use vals_core::replay::{GOLDEN_REPLAY, Replay};
use vals_core::{DT, Equipo, Events, InputFrame, MAX_BULLETS, Mode, Recorder, World};

mod audio;
mod bailarines;
mod bullet_renderer;
mod calle;
mod calle_arrabal;
mod calle_chicago;
mod calle_montmartre;
mod cartela;
mod draw;
mod escenarios;
mod fuentes;
mod guardado;
mod hot;
mod jefes;
mod mando;
mod mapa;
mod modista;
mod music;
mod paleta;
mod particulas;
mod pelicula;
mod protagonista;
mod replay_io;
mod salon;
mod skeleton;
mod stats;
mod trazo;
mod zumo;

use audio::{Audio, Sfx};
use bullet_renderer::BulletRenderer;
use cartela::{Cartela, Fundido};
use guardado::Guardado;
use hot::HotReload;
use mando::Mando;
use music::Tema;
use particulas::Particulas;
use stats::FrameStats;
use zumo::Zumo;

/// En que parte del juego estamos.
///
/// Antes esto era un `bool` —menu o partida— porque no habia mas sitios. La
/// pista mete un tercero, y es el que convierte una fila de jefes en un lugar:
/// del menu se entra a la pista, y de la pista a cada baile.
///
/// El paseo va entre la pista y el combate: la calle que se anda antes del
/// jefe, si su baile tiene una y no se ha andado ya. La
/// modista es la tienda del mapa.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Escena {
    Menu,
    Pista,
    Paseo,
    Combate,
    Modista,
}

/// "VALS" en ASCII. Semilla por defecto.
const SEED: u64 = 0x5641_4C53;

/// Cada cuantos frames se mira si cambio el fichero del patron.
///
/// Tres veces por segundo: imperceptible al guardar, e irrelevante para el
/// frame time comparado con dibujar un millar de balas.
const HOT_RELOAD_EVERY: u32 = 20;

/// Balas de la escena de stress si no se pide otra cosa.
const BENCH_DEFAULT: usize = 10_000;

/// Ticks que se queda la jugadora en la puerta antes de pasar al baile: lo
/// justo para ver abrirse la luz del salon.
const EN_LA_PUERTA: u64 = 50;

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
    // `--exportar-musica <carpeta>`: escribe los temas como .wav y sale.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(i) = args.iter().position(|a| a == "--exportar-musica") {
            let dir = args.get(i + 1).map_or("musica", String::as_str);
            match audio::exportar_musica(std::path::Path::new(dir)) {
                Ok(hechos) => hechos.iter().for_each(|r| println!("{}", r.display())),
                Err(e) => eprintln!("no se pudo exportar la musica: {e}"),
            }
            return;
        }
    }
    // Antes de dibujar nada: las tres formas de arrancar escriben texto.
    fuentes::cargar();
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
    // Apagado de serie: es una herramienta, no parte del juego. F1 lo trae.
    let mut show_debug = false;
    let mut hot = HotReload::new();
    let mut frames: u32 = 0;
    let mut aviso: Option<(String, bool, u32)> = None;
    let mut bullets_gpu = nuevo_renderer();
    let mut attract = Attract::new();
    let mut escena = Escena::Menu;
    // La pista se monta con un nodo por jefe: anadir el tango sera anadir su
    // RON, y aparecera solo.
    let bailes = BossDef::default_bosses();
    let mut pista = Pista::new(bailes.iter().map(|d| (d.name.clone(), d.nivel)).collect());
    // Y cual se baila en el suelo, que el mapa lo ensena.
    for n in pista.nodos.iter_mut() {
        n.suelo = bailes[n.jefe].suelo;
    }
    // El progreso de partidas anteriores. Se aplica por nombre, asi que un
    // guardado de cuando habia dos bailes sigue valiendo con cuatro.
    let mut progreso = Guardado::cargar();
    for i in 0..pista.nodos.len() {
        if progreso.tiene(&pista.nodos[i].nombre) {
            pista.marcar_vencido(i);
        }
    }
    let mut nodo_actual = 0usize;
    let mut marcado = false;
    let mut intentos: u32 = 0;
    let mut audio = Audio::load().await;
    // Los navegadores no dejan sonar nada hasta que el usuario toca algo. Si la
    // musica arrancase sola, en web el menu saldria mudo y no se sabria por
    // que. Se espera a la primera tecla, que ademas es cuando el jugador esta
    // mirando.
    let mut hubo_interaccion = false;
    let mut mando = Mando::new();
    let mut chispas = Particulas::new();
    let mut zumo = Zumo::new();
    let mut cartela: Option<Cartela> = None;
    // Donde murio la jugadora y cuando: de ahi sube su fantasma. Hay que
    // guardarlo antes del paso, porque `die` la devuelve a la salida en el
    // mismo tick en que muere.
    let mut fantasma: Option<((f32, f32), f64)> = None;
    let mut fundido = Fundido::nuevo();
    // Los paseos de serie, y el que se esta andando. Cada uno dice delante de
    // que baile va, por nombre.
    let paseos = PaseoDef::de_serie();
    // El paseo que se anda, y si es un repaso: uno ya andado al que se vuelve
    // desde el mapa a por las fichas que quedaron. Un repaso acaba en la
    // pista, no en el baile.
    let mut paseo: Option<(usize, Paseo)> = None;
    // Segundos que lleva cayendo el jefe; `None` si no ha
    // caido.
    let mut caida: Option<f32> = None;
    let mut repaso = false;
    let mut tienda = modista::Modista::new();

    loop {
        frames += 1;
        mando.actualizar();
        if frames.is_multiple_of(HOT_RELOAD_EVERY)
            && let Some(defs) = hot.poll()
        {
            // El mundo solo tiene el baile en curso, asi que se le recarga
            // solo ese: pasarle la lista entera le devolveria los demas.
            if let Some(def) = defs.get(world.baile) {
                world.reload_bosses(vec![def.clone()]);
            }
            // El replay en curso ya no reproduce nada: el jefe ha cambiado.
            recorder = Recorder::for_world(&world);
        }
        if frames.is_multiple_of(HOT_RELOAD_EVERY)
            && escena == Escena::Paseo
            && let Some((i, p)) = paseo.as_mut()
            && let Some(def) = hot.poll_paseo(*i)
        {
            p.recargar(def);
        }

        hubo_interaccion |= get_last_key_pressed().is_some() || mando.frame().bits() != 0;
        if hubo_interaccion {
            // La musica se busca por (jefe, figura). La pista tiene la suya
            // propia: dejar sonando el vals en el mapa decia que el vals era
            // el juego, y el vals es UN jefe del juego.
            audio.poner_musica(match escena {
                Escena::Combate => Tema::de(world.baile, world.boss.phase),
                // En la calle suena la primera figura de su baile: el vals se
                // oye desde fuera, y al entrar sigue sonando sin cortarse.
                Escena::Paseo => Tema::de(pista.nodos[nodo_actual].jefe, 0),
                _ => Tema::Sala,
            });
        }

        let frame_dt = get_frame_time().min(MAX_FRAME_DT);
        stats.push_frame(frame_dt);

        // Hitstop: unos frames sin avanzar el mundo al parriar, al cambiar de
        // figura y al morir. Congela el reloj de pared, no los ticks, asi que
        // el replay no se entera: el mundo hace los mismos ticks, mas tarde.
        // La cartela de entrada para el mundo mientras se lee: una cartela que
        // hay que leer esquivando no se lee. Y se salta con cualquier tecla,
        // porque la segunda vez ya te la sabes.
        //
        // Ojo con el mando: hay que mirar el **flanco**, no si hay algo
        // apretado. Se entra a un baile pulsando un boton y ese boton sigue
        // pulsado despues, asi que mirando el estado la cartela se saltaba sola
        // y no daba tiempo a leerla.
        if let Some(c) = cartela.as_mut()
            && c.para_el_mundo()
            && (get_last_key_pressed().is_some() || mando.algo_pulsado())
        {
            c.saltar();
        }
        let leyendo = cartela.as_ref().is_some_and(Cartela::para_el_mundo);
        let jugando = matches!(escena, Escena::Combate | Escena::Paseo);
        let congelado = jugando && (zumo.congelado() || leyendo);

        if is_key_pressed(KeyCode::F1) {
            show_debug = !show_debug;
        }
        if (is_key_pressed(KeyCode::R) || mando.pulsado(InputFrame::PARRY) && world.is_over())
            && escena == Escena::Combate
        {
            // Tras perder se reintenta **este** jefe con las vidas llenas; en
            // cualquier otro momento, partida nueva desde el principio.
            //
            // "Desde el principio" es el principio de ESTE baile y en su modo:
            // `with_mode` montaba el vals con los cuatro detras, asi que R en
            // mitad del cancan te mandaba a bailar el vals, y ahora ademas en
            // el suelo.
            if world.defeat {
                world.retry_current_boss();
            } else {
                world = World::empezar_con(SEED, world.mode, world.baile, world.equipo);
            }
            recorder = Recorder::for_world(&world);
            accumulator = 0.0;
            intentos += 1;
        }
        // En el paseo, R (o el parry tras el KO) vuelve a empezar la calle.
        // Morir en el paseo no te lleva al jefe ni te lo quita: se reintenta
        // el paseo, igual que morir en el jefe reintenta solo el jefe.
        if escena == Escena::Paseo
            && let Some((i, p)) = paseo.as_mut()
            && (is_key_pressed(KeyCode::R) || mando.pulsado(InputFrame::PARRY) && p.derrota)
        {
            *p = nuevo_paseo(&paseos[*i], &progreso);
            accumulator = 0.0;
        }
        // Start/Select del mando hacen de ESC: salir un escalon. En la calle
        // no, que alli el super es el super.
        if is_key_pressed(KeyCode::Escape) || mando.pulsado(InputFrame::SUPER) && !jugando {
            // Se sale un escalon cada vez: del baile o del paseo a la pista,
            // de la pista al menu.
            escena = match escena {
                Escena::Combate | Escena::Paseo | Escena::Modista => Escena::Pista,
                _ => Escena::Menu,
            };
            cartela = None;
            fundido.empezar(0.24);
        }
        if is_key_pressed(KeyCode::M) {
            audio.toggle_mute();
        }
        // La linea que hierve se puede parar: a quien le marea no le sirve
        // de nada que sea bonita.
        if is_key_pressed(KeyCode::T) {
            let vivo = trazo::alternar();
            let msg = if vivo { "trazo vivo" } else { "trazo quieto" };
            aviso = Some((msg.to_owned(), false, 120));
        }
        // En la tienda. Va antes que la pista por lo mismo que la pista va
        // antes que el menu: la Z que entra en la modista no puede comprar lo
        // primero del catalogo en el mismo frame. Con mando, el circulo sale,
        // como en el KO.
        if escena == Escena::Modista {
            if mando.pulsado(InputFrame::PARRY) {
                escena = Escena::Pista;
                fundido.empezar(0.24);
            } else if let Some(sfx) = tienda.entrada(&mut progreso, &mando) {
                audio.play(sfx, 0.8);
            }
        }
        // De la pista se entra al baile que se tenga delante.
        //
        // Se mira **antes** que el menu a proposito: las dos escenas usan la Z,
        // y si el menu fuese primero, la misma pulsacion que entra a la pista
        // se leeria otra vez aqui en el mismo frame. Hoy no se notaria porque
        // se entra lejos de todo, pero deja de ser verdad en cuanto un baile
        // este cerca de la entrada.
        let entrar = is_key_pressed(KeyCode::Z) || mando.pulsado(InputFrame::JUMP);
        // Volver a pasear: la X (el dash; cuadrado con mando) delante de un
        // baile cuyo paseo ya se anduvo. Es un boton aparte y no una pregunta
        // al entrar porque lo normal al volver a un baile es bailarlo.
        let pasear = is_key_pressed(KeyCode::X) || mando.pulsado(InputFrame::DASH);
        if escena == Escena::Pista && entrar && pista.en_la_modista() {
            audio.play(Sfx::Empezar, 1.0);
            escena = Escena::Modista;
            tienda = modista::Modista::new();
            fundido.empezar(0.30);
        }
        if escena == Escena::Pista
            && (entrar || pasear)
            && let Some(i) = pista.nodo_cerca()
            && pista.abierto(i)
        {
            // Ojo: el jefe sale del NODO, no del sitio en la pista. La pista
            // reordena por nivel.
            let jefe = pista.nodos[i].jefe;
            // Antes del baile, su paseo, si lo tiene y no se ha andado ya: una
            // vez hecha la calle, volver a un baile es volver a bailarlo, no a
            // andar hasta el. Se busca por nombre, como el guardado. Con la X
            // se vuelve a el aunque ya se haya andado.
            let suyo = paseos.iter().position(|d| d.jefe == bailes[jefe].name);
            let andado = suyo.is_some_and(|k| progreso.tiene(&paseos[k].nombre));
            let calle = suyo.filter(|_| !andado || pasear && !entrar);
            if entrar || calle.is_some() {
                audio.play(Sfx::Empezar, 1.0);
                nodo_actual = i;
                marcado = false;
                repaso = andado;
                if let Some(k) = calle {
                    let def = &paseos[k];
                    escena = Escena::Paseo;
                    paseo = Some((k, nuevo_paseo(def, &progreso)));
                    cartela = Some(Cartela::prologo(
                        &def.nombre,
                        &def.subtitulo,
                        Tema::de(jefe, 0).titulo(),
                    ));
                } else {
                    escena = Escena::Combate;
                    (world, recorder, cartela) = empezar_baile(jefe, progreso.equipo);
                    intentos += 1;
                }
                accumulator = 0.0;
                fundido.empezar(0.30);
            }
        }
        // Llegar a la puerta: el paseo queda andado para siempre, **y sus
        // fichas pasan a ser tuyas**, y se entra al baile con su cartela, como
        // si se hubiera entrado desde la pista. Las fichas se quedan aqui y no
        // al cogerlas, como en Cuphead: una ficha en un sitio dificil cuesta
        // arriesgarse **y** acabar la calle, y morir despues de cogerla la
        // devuelve a su sitio. Un repaso vuelve a la pista, que el baile ya se
        // puede entrar directo.
        if escena == Escena::Paseo
            && let Some((k, p)) = paseo.as_ref()
            && p.completado.is_some_and(|t| p.tick >= t + EN_LA_PUERTA)
        {
            let def = &paseos[*k];
            progreso.marcar(&def.nombre);
            for (i, _) in p.fichas.iter().enumerate().filter(|(_, f)| **f) {
                progreso.marcar_ficha(&def.nombre, i);
            }
            progreso.guardar();
            if repaso {
                escena = Escena::Pista;
                let n = p.fichas_cogidas();
                aviso = Some((
                    format!("{n} de {} fichas del paseo son tuyas", p.fichas.len()),
                    false,
                    180,
                ));
            } else {
                escena = Escena::Combate;
                (world, recorder, cartela) =
                    empezar_baile(pista.nodos[nodo_actual].jefe, progreso.equipo);
                intentos += 1;
            }
            accumulator = 0.0;
            fundido.empezar(0.30);
        }

        // Del menu a la pista, con Z. Antes Z entraba volando y X con salto,
        // y el modo valia para todos los bailes de la visita. Ya no hay nada
        // que elegir en la puerta: cada baile trae su modo, asi
        // que la X no hace nada aqui. Dejarla como sinonimo seria un segundo
        // boton que no dice nada, y el menu tendria que explicar por que hay
        // dos.
        if escena == Escena::Menu && (is_key_pressed(KeyCode::Z) || mando.pulsado(InputFrame::JUMP))
        {
            audio.play(Sfx::Empezar, 1.0);
            escena = Escena::Pista;
            accumulator = 0.0;
            fundido.empezar(0.30);
        }
        // Saltarse el baile. Es una trampa y esta puesta a proposito: ver
        // como avanza el juego no deberia costar pasarse el jefe cada vez.
        // Rompe el replay en curso, asi que se vuelve a empezar la grabacion.
        // Y lo mismo con la calle.
        if is_key_pressed(KeyCode::F3)
            && escena == Escena::Paseo
            && let Some((_, p)) = paseo.as_mut()
            && !p.terminado()
        {
            p.saltar();
            aviso = Some(("paseo saltado".to_owned(), false, 120));
        }
        if is_key_pressed(KeyCode::F3) && escena == Escena::Combate && !world.is_over() {
            world.saltar_el_baile();
            recorder = Recorder::for_world(&world);
            // El sonido se pide a mano: los eventos del mundo se recogen
            // dentro del bucle de simulacion, y el proximo tick ya los habra
            // vaciado.
            audio.play(Sfx::Fase, 1.0);
            zumo.golpe_gordo();
            aviso = Some(("baile saltado".to_owned(), false, 120));
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
        // Teclado y mando se suman en vez de elegir uno: no hay que anunciar
        // con cual se juega, y soltar el mando a media partida no rompe nada.
        let input = if escena == Escena::Menu {
            InputFrame::NONE
        } else {
            InputFrame::from_bits(read_input().bits() | mando.frame().bits())
        };
        let t0 = get_time();
        if !congelado {
            accumulator += frame_dt;
        }
        let mut steps = 0;
        // Los sucesos de todos los ticks del frame se juntan: puede haber
        // varios, y reaccionar solo al ultimo se comeria sonidos.
        let mut eventos = Events::default();
        while accumulator >= DT {
            match escena {
                Escena::Menu => {
                    if let Some(a) = attract.as_mut() {
                        a.step();
                        // El atractor tambien echa chispas: es lo primero que
                        // se ve del juego y merece estar vivo.
                        eventos.merge(&a.world.events);
                    }
                }
                Escena::Pista => pista.step(input),
                Escena::Modista => {}
                Escena::Paseo => {
                    if let Some((_, p)) = paseo.as_mut() {
                        p.step(input);
                        eventos.merge(&p.events);
                        // Las chispas del paseo salen tick a tick: cada una
                        // tiene que salir de donde paso, y al final del frame
                        // solo queda el sitio del ultimo impacto.
                        // Conversion explicita en la frontera: el core usa su
                        // glam y macroquad el suyo.
                        let (b, a) = (p.boca(), p.apunta);
                        let boca = (vec2(b.x, b.y), vec2(a.x, a.y));
                        let j = vec2(p.jugadora.pos.x, p.jugadora.pos.y);
                        let golpe = vec2(p.impacto.x, p.impacto.y);
                        chispas.reaccionar_en(&p.events, j, golpe, boca);
                        if p.events.fichas > 0 {
                            chispas.estallido(j, paleta::ORO);
                        }
                        if p.caidos > 0 {
                            let (_, ropa) = paleta::del_baile(pista.nodos[nodo_actual].jefe);
                            chispas.estallido(golpe, ropa);
                            audio.play(Sfx::Fase, 0.55);
                        }
                    }
                }
                Escena::Combate => {
                    let antes = (world.player.pos.x, world.player.pos.y);
                    world.step(input);
                    if world.events.player_died {
                        fantasma = Some((antes, get_time()));
                    }
                    recorder.record(input, &world);
                    eventos.merge(&world.events);
                }
            }
            // Las chispas van al mismo paso fijo que la simulacion: si fueran
            // por frame, en un monitor de 144 Hz volarian al doble.
            chispas.step(DT);
            zumo.step();
            accumulator -= DT;
            steps += 1;
            if steps >= MAX_STEPS_PER_FRAME {
                accumulator = 0.0;
                break;
            }
        }
        // La cartela y el fundido corren con el reloj de pared, no con los
        // ticks. Es presentacion —tiene que durar igual a 60 que a 144 Hz— y,
        // sobre todo, **tiene que avanzar mientras la cartela para el mundo**,
        // que es justo cuando no hay ticks. Meterlos en el bucle de simulacion
        // dejaba la pantalla en negro para siempre al entrar a un baile.
        fundido.step(frame_dt);
        if let Some(c) = cartela.as_mut()
            && !c.step(frame_dt)
        {
            // Al acabar la presentacion, el grito: el "WALLOP!" de Cuphead. Es
            // lo que convierte el final de una cartela en el principio de un
            // combate.
            cartela = c.es_de_entrada().then(|| match escena {
                Escena::Paseo => Cartela::grito("¡EN MARCHA!"),
                _ => Cartela::a_bailar(),
            });
        }
        stats.push_sim((get_time() - t0) as f32);
        // En el menu no suena nada ni sacude nada: el atractor es un fondo, no
        // una partida. Las chispas si, porque son lo que lo hace parecer vivo.
        match (escena, attract.as_ref()) {
            (Escena::Combate, _) => {
                // La fanfarria de la victoria espera al sello, al final de
                // la caida del jefe.
                audio.play_events(&Events {
                    victory: false,
                    ..eventos
                });
                chispas.reaccionar(&eventos, &world);
                zumo.reaccionar(&eventos);
                if eventos.phase_changed && !world.boss.defeated {
                    cartela = Some(Cartela::figura(world.boss.phase_name(), world.boss.phase));
                }
            }
            (Escena::Paseo, _) => {
                audio.play_events(&eventos);
                zumo.reaccionar(&eventos);
            }
            (Escena::Menu, Some(a)) => chispas.reaccionar(&eventos, &a.world),
            _ => {}
        }

        // La caida del jefe corre con el reloj de pared, como la cartela, y
        // no durante el hitstop del golpe: el frame del golpe solo la arma,
        // y empieza a contar cuando se descongela. Con R o ESC la victoria se
        // acaba y la caida se desarma sola.
        let antes = caida.unwrap_or(0.0);
        caida = match caida {
            _ if escena != Escena::Combate || !world.victory => None,
            None => Some(0.0),
            Some(s) => Some(s + if congelado { 0.0 } else { frame_dt }),
        };
        let ahora = caida.unwrap_or(0.0);
        if antes == 0.0 && ahora > 0.0 {
            audio.play(Sfx::caida(world.baile), 1.0);
        }
        if jefes::caida(antes) < 1.0 && jefes::caida(ahora) >= 1.0 {
            audio.play(Sfx::Victoria, 1.0);
        }

        // Ganar un baile lo tacha en la pista. Una sola vez, que la pantalla
        // de victoria se queda puesta muchos frames.
        if escena == Escena::Combate && world.victory && !marcado {
            pista.marcar_vencido(nodo_actual);
            progreso.marcar(&pista.nodos[nodo_actual].nombre);
            progreso.guardar();
            marcado = true;
        }

        // Fraccion de tick pendiente. Es lo que permite que el render vaya a
        // 144 Hz con la simulacion a 60 sin que se vea a saltos.
        let alpha = (accumulator / DT).clamp(0.0, 1.0);

        // --- Render ---
        let t1 = get_time();
        // El paseo es apaisado; todo lo demas, la arena de siempre.
        let lamina = match escena {
            Escena::Paseo => draw::Layout::para(VISTA_W, VISTA_H),
            _ => draw::Layout::compute(),
        };
        let layout = lamina.sacudido(zumo.desplazamiento());
        if escena == Escena::Pista {
            // Lo que el mapa dice de las fichas: el bolsillo y, de cada baile
            // cuyo paseo ya se anduvo, cuantas de las suyas son tuyas.
            let fichas = mapa::Fichas {
                bolsillo: modista::saldo(&progreso),
                paseos: pista
                    .nodos
                    .iter()
                    .map(|n| {
                        let def = paseos.iter().find(|d| d.jefe == n.nombre)?;
                        progreso.tiene(&def.nombre).then(|| {
                            let total = def.fichas.len();
                            let tuyas = (0..total)
                                .filter(|i| progreso.tiene_ficha(&def.nombre, *i))
                                .count();
                            (tuyas, total)
                        })
                    })
                    .collect(),
            };
            let teclas = match mando.conectado().then(|| mando.botones()) {
                Some(b) => [b[4], b[1]],
                None => ["Z", "X"],
            };
            mapa::pista(&pista, alpha, &layout, &fichas, teclas);
        } else if escena == Escena::Modista {
            tienda.dibujar(
                &progreso,
                &layout,
                mando.conectado().then(|| mando.botones()),
            );
        } else if escena == Escena::Paseo
            && let Some((_, p)) = paseo.as_ref()
        {
            let jefe = pista.nodos[nodo_actual].jefe;
            let (pulso, _) = salon::latido(Tema::de(jefe, 0), p.tick as f32 + alpha);
            calle::dibujar(p, alpha, &layout, pulso, jefe);
            let camara = layout.sacudido(-vec2(calle::camara(p, alpha), calle::camara_y(p, alpha)));
            draw::particulas(&chispas, &camara);
            calle::encima(p, &layout);
        } else {
            let mostrado = match (escena, attract.as_ref()) {
                (Escena::Menu, Some(a)) => &a.world,
                _ => &world,
            };
            let cayendo = match escena {
                Escena::Combate => jefes::caida(caida.unwrap_or(0.0)),
                _ => 1.0,
            };
            draw::frame(mostrado, alpha, &layout, bullets_gpu.as_mut(), cayendo);
            draw::particulas(&chispas, &layout);
            if let Some((donde, cuando)) = fantasma
                && escena == Escena::Combate
            {
                draw::fantasma(&world, donde, (get_time() - cuando) as f32, &layout);
            }
            if escena == Escena::Menu {
                draw::menu(
                    &layout,
                    mostrado,
                    intentos,
                    mando.conectado().then(|| mando.botones()),
                );
            } else if world.is_over() {
                draw::fin_de_partida(&layout, &world, jefes::caida(caida.unwrap_or(0.0)));
            }
        }
        if show_debug && escena == Escena::Combate {
            draw::debug_overlay(
                &world,
                &stats,
                steps,
                chispas.vivas(),
                mando.conectado().then(|| mando.botones()),
            );
            draw::recording_badge(recorder.ticks());
        }
        if let Some(c) = cartela.as_ref() {
            match escena {
                Escena::Paseo => c.dibujar_en(&layout, vec2(VISTA_W, VISTA_H)),
                _ => c.dibujar(&layout),
            }
        }
        fundido.dibujar();
        // La pelicula va encima de todo: es la copia la que tiene grano.
        pelicula::dibujar();
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

/// Todo lo que hace falta para empezar el baile `jefe`: su mundo, su
/// grabacion y su cartela. Se entra desde dos sitios —la pista y la puerta del
/// paseo— y los dos tienen que entrar igual.
///
/// El modo lo pone el baile: el charleston y el cancan se bailan
/// en el suelo y el resto en el aire. El vuelo que se pasa aqui es solo para
/// los que no dicen nada.
///
/// Con el equipo que se lleve puesto, que entra en el mundo y
/// en la grabacion.
fn empezar_baile(jefe: usize, equipo: Equipo) -> (World, Recorder, Option<Cartela>) {
    let world = World::empezar_con(SEED, Mode::Flight, jefe, equipo);
    let recorder = Recorder::for_world(&world);
    let cartela = Cartela::entrada(
        &world.boss.name,
        world.boss.phase_name(),
        Tema::de(world.baile, 0).titulo(),
    );
    (world, recorder, Some(cartela))
}

/// Un paseo recien empezado con lo que se lleva puesto, y sin las fichas que
/// ya son tuyas: esas no vuelven a estar en la calle.
fn nuevo_paseo(def: &PaseoDef, progreso: &Guardado) -> Paseo {
    let mut p = Paseo::con_equipo(def.clone(), SEED, progreso.equipo);
    for (i, f) in p.fichas.iter_mut().enumerate() {
        *f = progreso.tiene_ficha(&def.nombre, i);
    }
    p
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

    let mut world = replay.mundo();
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
            world = replay.mundo();
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
        draw::frame(&world, alpha, &layout, bullets_gpu.as_mut(), 1.0);
        draw::debug_overlay(&world, &stats, steps, 0, None);
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
