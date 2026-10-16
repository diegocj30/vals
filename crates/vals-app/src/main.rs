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

mod acciones;
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
mod glifos;
mod guardado;
mod hot;
mod idioma;
mod jefes;
mod libro;
mod mando;
mod mapa;
mod modista;
mod music;
mod opciones;
mod ovacion;
mod paleta;
mod particulas;
mod pelicula;
mod prologo;
mod protagonista;
mod replay_io;
mod salon;
mod skeleton;
mod stats;
mod trazo;
mod zumo;

use acciones::{Accion, Controles, Dispositivo, Entrada, Nombres};
use audio::{Audio, Sfx};
use bullet_renderer::BulletRenderer;
use cartela::{Cartela, Fundido};
use guardado::Guardado;
use hot::HotReload;
use idioma::{Texto, nombre, tx, txf};
use music::Tema;
use opciones::{Carta, Orden, VOLUMEN_MAX};
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
/// modista es la tienda del mapa. El prologo cuenta la historia antes de la
/// primera visita a la pista, y desde el menu cuando se pida. El libro de
/// baile, las marcas, se abre desde el menu.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Escena {
    Menu,
    Prologo,
    Pista,
    Paseo,
    Combate,
    Modista,
    Libro,
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

/// Segundos que el KO tiene que estar en pantalla antes de que aceptar o
/// atras hagan algo. Se muere con el dedo en disparar o en el parry, que son
/// tambien aceptar y atras: sin esto, el mismo golpe que mata se saltaba el
/// KO sin dejar verlo. La R, el Select y ESC van sin espera, que no se
/// pulsan sin querer.
const GRACIA_FIN: f64 = 0.8;

/// Lo que baja la musica en el entreacto: se oye que sigue ahi, pero no
/// tapa.
const MUSICA_EN_PAUSA: f32 = 0.3;

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
    // `--exportar-sonidos <carpeta>`: lo mismo con los efectos que van en el
    // tono de cada tema, y una mezcla de cada tema con ellos encima.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().collect();
        let exportar = [
            (
                "--exportar-musica",
                "musica",
                audio::exportar_musica as fn(&_) -> _,
            ),
            ("--exportar-sonidos", "sonidos", audio::exportar_sonidos),
        ];
        for (bandera, defecto, que) in exportar {
            if let Some(i) = args.iter().position(|a| a == bandera) {
                let dir = args.get(i + 1).map_or(defecto, String::as_str);
                match que(std::path::Path::new(dir)) {
                    Ok(hechos) => hechos.iter().for_each(|r| println!("{}", r.display())),
                    Err(e) => eprintln!("no se pudo exportar: {e}"),
                }
                return;
            }
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
    let mut audio = Audio::load(ganados(&pista)).await;
    // Los navegadores no dejan sonar nada hasta que el usuario toca algo. Si la
    // musica arrancase sola, en web el menu saldria mudo y no se sabria por
    // que. Se espera a la primera tecla, que ademas es cuando el jugador esta
    // mirando.
    let mut hubo_interaccion = false;
    // Teclado y mando, con cual se toco lo ultimo. Ver `acciones.rs`.
    let mut entrada = Entrada::new();
    // El entreacto, o las opciones desde el titulo.
    let mut carta: Option<Carta> = None;
    // Desde cuando esta el KO en pantalla. Ver `GRACIA_FIN`.
    let mut fin_desde: Option<f64> = None;
    #[cfg(not(target_arch = "wasm32"))]
    let mut pantalla_puesta = false;
    let mut chispas = Particulas::new();
    let mut zumo = Zumo::new();
    let mut ovacion = ovacion::Ovacion::new();
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
    // El prologo en curso, y adonde se va al acabarlo: a la pista la primera
    // vez, y de vuelta al menu si se ha pedido desde alli.
    let mut historia = prologo::Prologo::nuevo();
    let mut tras_prologo = Escena::Pista;
    // El intento en curso, de baile o de paseo: lo que se apunta para la
    // tarjeta del final y para el libro.
    let mut intento = libro::Intento::default();

    loop {
        frames += 1;
        entrada.actualizar();
        let escena_antes = escena;
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

        hubo_interaccion |= entrada.algo() || entrada.mando.crudo() != 0;
        // Las opciones se aplican cada frame: son cuatro asignaciones, y asi
        // no hay que acordarse de aplicarlas en cada sitio que las cambia.
        let op = progreso.opciones;
        let jugando = matches!(escena, Escena::Combate | Escena::Paseo);
        let parado = jugando && carta.is_some();
        let agachada = if parado { MUSICA_EN_PAUSA } else { 1.0 };
        let volumen = |v: u8| f32::from(v) / f32::from(VOLUMEN_MAX);
        audio.volumenes(volumen(op.musica) * agachada, volumen(op.efectos));
        trazo::poner(op.hervor);
        idioma::poner(op.idioma);
        zumo.sacudir = op.sacudida;
        #[cfg(not(target_arch = "wasm32"))]
        if op.pantalla_completa != pantalla_puesta {
            set_fullscreen(op.pantalla_completa);
            pantalla_puesta = op.pantalla_completa;
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
        //
        // La pausa no la salta: se puede parar en mitad de una cartela.
        if let Some(c) = cartela.as_mut()
            && c.para_el_mundo()
            && carta.is_none()
            && entrada.algo()
            && !entrada.pulsado(Accion::Pausa)
        {
            c.saltar();
        }
        let leyendo = cartela.as_ref().is_some_and(Cartela::para_el_mundo);
        // El entreacto va primero a proposito: con `||`, el hitstop no se
        // consume mientras esta abierto, y al seguir el golpe sigue frenado.
        let congelado = jugando && (parado || zumo.congelado() || leyendo);

        // --- Las acciones de menu ---
        // Con la tarjeta abierta, solo ella escucha. `libre` se mira antes de
        // tocarla: el frame en que se cierra, la tecla que la cerro no puede
        // hacer ademas otra cosa (ESC cerraria y volveria a abrir).
        let libre = carta.is_none();
        let capturando = carta.as_ref().is_some_and(Carta::capturando);
        let acabado = match escena {
            Escena::Combate => world.is_over(),
            Escena::Paseo => paseo.as_ref().is_some_and(|(_, p)| p.derrota),
            _ => false,
        };
        fin_desde = if acabado {
            fin_desde.or(Some(get_time()))
        } else {
            None
        };
        // La victoria espera a que se pueda leer su tarjeta (ver `libro`).
        let ganado = escena == Escena::Combate && world.victory;
        let listo = if ganado {
            intento.tarjeta.as_ref().is_some_and(libro::Tarjeta::lista)
        } else {
            fin_desde.is_some_and(|t| get_time() - t > GRACIA_FIN)
        };
        let mut orden = None;
        if let Some(c) = carta.as_mut() {
            let antes = progreso.opciones;
            orden = c.entrada(&entrada, &mut progreso.opciones);
            if progreso.opciones != antes {
                progreso.guardar();
            }
            match orden {
                Some(Orden::Tic) => audio.play(Sfx::Graze, 0.7),
                Some(_) => carta = None,
                None => {}
            }
        } else if entrada.pulsado(Accion::Pausa) && (escena == Escena::Menu || jugando && !acabado)
        {
            // En un baile o un paseo, el entreacto; en el titulo, las
            // opciones.
            carta = Some(if jugando {
                Carta::entreacto()
            } else {
                Carta::opciones()
            });
            audio.play(Sfx::Graze, 0.7);
        }
        let aceptar = libre && entrada.pulsado(Accion::Aceptar);
        // Reintentar: desde el entreacto, con la R o el Select en cualquier
        // momento, y con aceptar en el KO.
        let reintentar = jugando
            && (orden == Some(Orden::Reintentar)
                || libre && entrada.pulsado(Accion::Reintentar)
                || acabado && listo && !ganado && aceptar);
        // Salir a la pista: desde el entreacto, o al acabar. ESC y Start
        // tambien al acabar: no hay entreacto que abrir en un KO.
        let salir = jugando
            && (orden == Some(Orden::Mapa)
                || libre
                    && acabado
                    && (entrada.pulsado(Accion::Pausa)
                        || listo && (entrada.pulsado(Accion::Atras) || ganado && aceptar)));

        if is_key_pressed(KeyCode::F1) {
            show_debug = !show_debug;
        }
        // La ovacion del super no se arrastra al intento nuevo.
        if reintentar || salir {
            ovacion = ovacion::Ovacion::new();
        }
        if reintentar && escena == Escena::Combate {
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
            intento = progreso.marcas.empezar(&pista.nodos[nodo_actual].nombre);
        }
        // En el paseo, reintentar vuelve a empezar la calle. Morir en el
        // paseo no te lleva al jefe ni te lo quita: se reintenta el paseo,
        // igual que morir en el jefe reintenta solo el jefe.
        if reintentar
            && escena == Escena::Paseo
            && let Some((i, p)) = paseo.as_mut()
        {
            *p = nuevo_paseo(&paseos[*i], &progreso);
            intento = progreso.marcas.empezar(&paseos[*i].nombre);
            accumulator = 0.0;
        }
        // Salir un escalon: del baile o del paseo a la pista (desde el
        // entreacto o al acabar), de la tienda a la pista y de la pista al
        // menu. Atras o pausa, que en la pista y la tienda no hay entreacto.
        // El prologo y el libro miran lo suyo mas abajo.
        let atras = libre
            && matches!(escena, Escena::Pista | Escena::Modista)
            && (entrada.pulsado(Accion::Atras) || entrada.pulsado(Accion::Pausa));
        if salir || atras {
            escena = match escena {
                Escena::Combate | Escena::Paseo | Escena::Modista => Escena::Pista,
                _ => Escena::Menu,
            };
            cartela = None;
            fundido.empezar(0.24);
        }
        // Mientras se espera una tecla nueva para un control, ninguna otra
        // hace nada: la M no deberia quitar el sonido al intentar asignarla.
        if !capturando && is_key_pressed(KeyCode::M) {
            audio.toggle_mute();
        }
        // La linea que hierve se puede parar: a quien le marea no le sirve
        // de nada que sea bonita. Es la misma opcion que la del menu, y se
        // guarda igual.
        if !capturando && is_key_pressed(KeyCode::T) {
            progreso.opciones.hervor = !progreso.opciones.hervor;
            progreso.guardar();
            let msg = tx(if progreso.opciones.hervor {
                Texto::TrazoVivo
            } else {
                Texto::TrazoQuieto
            });
            aviso = Some((msg.to_owned(), false, 120));
        }
        // En la tienda. Va antes que la pista por lo mismo que la pista va
        // antes que el menu: la Z que entra en la modista no puede comprar lo
        // primero del catalogo en el mismo frame. Se sale con atras, arriba.
        if libre
            && escena == Escena::Modista
            && let Some(sfx) = tienda.entrada(&mut progreso, &entrada)
        {
            audio.play(sfx, 0.8);
        }
        // De la pista se entra al baile que se tenga delante.
        //
        // Se mira **antes** que el menu a proposito: las dos escenas usan la Z,
        // y si el menu fuese primero, la misma pulsacion que entra a la pista
        // se leeria otra vez aqui en el mismo frame. Hoy no se notaria porque
        // se entra lejos de todo, pero deja de ser verdad en cuanto un baile
        // este cerca de la entrada.
        //
        // Y nunca con la pulsacion que acaba de sacarte a la pista: al ganar,
        // aceptar vuelve a la pista, y en el mismo frame se leia otra vez aqui
        // con la bailarina delante del baile, que volvia a empezar.
        let recien_salido = salir || atras;
        let entrar = aceptar && !recien_salido;
        // Volver a pasear: la X (cuadrado con mando) delante de un baile cuyo
        // paseo ya se anduvo. Es un boton aparte y no una pregunta al entrar
        // porque lo normal al volver a un baile es bailarlo.
        let pasear = libre && !recien_salido && entrada.pulsado(Accion::Pasear);
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
                    intento = progreso.marcas.empezar(&def.nombre);
                    cartela = Some(
                        Cartela::prologo(
                            nombre(&def.nombre),
                            nombre(&def.subtitulo),
                            nombre(Tema::de(jefe, 0).titulo()),
                        )
                        .con_lema(prologo::lema_del_paseo(jefe)),
                    );
                } else {
                    escena = Escena::Combate;
                    (world, recorder, cartela) = empezar_baile(jefe, progreso.equipo);
                    intentos += 1;
                    intento = progreso.marcas.empezar(&bailes[jefe].name);
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
        //
        // En la puerta sale su tarjeta: se sigue al aceptar, en cuanto se ha
        // podido leer, o solo al rato.
        if escena == Escena::Paseo
            && let Some((k, p)) = paseo.as_ref()
            && p.completado.is_some_and(|t| {
                p.tick >= t + EN_LA_PUERTA
                    && (p.tick >= t + libro::LEER && entrar || p.tick >= t + libro::TARJETA_PASEO)
            })
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
                aviso = Some((txf(Texto::FichasTuyas, &[&n, &p.fichas.len()]), false, 180));
            } else {
                escena = Escena::Combate;
                (world, recorder, cartela) =
                    empezar_baile(pista.nodos[nodo_actual].jefe, progreso.equipo);
                intentos += 1;
                intento = progreso.marcas.empezar(&pista.nodos[nodo_actual].nombre);
            }
            accumulator = 0.0;
            fundido.empezar(0.30);
        }

        // El libro de baile: L, o el gatillo de focus del mando (L1), que en el
        // menu no hace nada; el cuadrado ya cuenta la historia. Se cierra con
        // lo mismo, con ESC o con el circulo.
        let libro_pulsado = libre && entrada.pulsado(Accion::Libro);
        // El final se ofrece en el menu cuando ya se ha ganado y visto.
        let hay_final = progreso.final_visto && pista.todo_vencido();
        if escena == Escena::Libro && (libro_pulsado || entrada.pulsado(Accion::Atras)) {
            escena = Escena::Menu;
            fundido.empezar(0.24);
        } else if escena == Escena::Menu && libro_pulsado {
            escena = Escena::Libro;
            fundido.empezar(0.24);
        }
        // Del menu a la pista, con Z. Antes Z entraba volando y X con salto,
        // y el modo valia para todos los bailes de la visita. Ya no hay nada
        // que elegir en la puerta: cada baile trae su modo, asi
        // que la X no hace nada aqui. Dejarla como sinonimo seria un segundo
        // boton que no dice nada, y el menu tendria que explicar por que hay
        // dos.
        //
        // La primera vez, antes de la pista, la historia. Y la X (cuadrado)
        // la vuelve a contar, que es el mismo boton secundario que en la pista.
        if escena == Escena::Menu && aceptar {
            audio.play(Sfx::Empezar, 1.0);
            escena = if progreso.prologo_visto {
                Escena::Pista
            } else {
                historia = prologo::Prologo::nuevo();
                tras_prologo = Escena::Pista;
                Escena::Prologo
            };
            accumulator = 0.0;
            fundido.empezar(0.30);
        } else if escena == Escena::Menu && libre && entrada.pulsado(Accion::Historia) {
            escena = Escena::Prologo;
            historia = prologo::Prologo::nuevo();
            tras_prologo = Escena::Menu;
            fundido.empezar(0.30);
        } else if escena == Escena::Menu && libre && hay_final && entrada.pulsado(Accion::Final) {
            // El final tambien se vuelve a ver, con el boton del super.
            escena = Escena::Prologo;
            historia = prologo::Prologo::el_final(prologo::Resumen::de(&progreso.marcas, &bailes));
            tras_prologo = Escena::Menu;
            fundido.empezar(0.30);
        }
        // El final sale solo al llegar a la pista con los cuatro bailes
        // ganados: lo normal es al volver del ultimo, despues de su tarjeta,
        // y si se cerro el juego antes, al entrar desde el menu. F4 lo trae
        // desde el menu o la pista, para verlo sin ganarse nada.
        let llegando = escena == Escena::Pista && escena_antes != Escena::Pista;
        let f4 =
            libre && is_key_pressed(KeyCode::F4) && matches!(escena, Escena::Menu | Escena::Pista);
        if f4 || llegando && prologo::toca_el_final(&pista, progreso.final_visto) {
            tras_prologo = if f4 { escena } else { Escena::Pista };
            escena = Escena::Prologo;
            historia = prologo::Prologo::el_final(prologo::Resumen::de(&progreso.marcas, &bailes));
            fundido.empezar(0.30);
        }
        // El prologo va despues del menu y de la pista a proposito: la tecla
        // que entra en el no puede leerse dos veces (la gracia la tapa), y la
        // que lo acaba no puede entrar a un baile en el mismo frame.
        //
        // Y no escucha el frame en que se entra: al final se llega con la
        // tecla que cierra la tarjeta del ultimo baile, y si era ESC se lo
        // saltaba entero sin verlo.
        if escena == Escena::Prologo {
            historia.step(frame_dt);
            let escucha = escena_antes == Escena::Prologo;
            if escucha && entrada.pulsado(Accion::Aceptar) {
                historia.avanzar();
            }
            // Saltarselo entero: atras o pausa, que aqui no hay entreacto.
            if escucha && (entrada.pulsado(Accion::Atras) || entrada.pulsado(Accion::Pausa)) {
                historia.saltar();
            }
            if historia.acabado() {
                escena = tras_prologo;
                // El final visto con F4 sin haberlo ganado no cuenta: si no,
                // luego no saldria solo.
                if historia.es_el_final() {
                    progreso.final_visto |= pista.todo_vencido();
                } else {
                    progreso.prologo_visto = true;
                }
                progreso.guardar();
                accumulator = 0.0;
                fundido.empezar(0.30);
            }
        }
        // Saltarse el baile. Es una trampa y esta puesta a proposito: ver
        // como avanza el juego no deberia costar pasarse el jefe cada vez.
        // Rompe el replay en curso, asi que se vuelve a empezar la grabacion.
        // Y lo mismo con la calle.
        if libre
            && is_key_pressed(KeyCode::F3)
            && escena == Escena::Paseo
            && let Some((_, p)) = paseo.as_mut()
            && !p.terminado()
        {
            p.saltar();
            intento.trampa = true;
            aviso = Some((tx(Texto::PaseoSaltado).to_owned(), false, 120));
        }
        if libre && is_key_pressed(KeyCode::F3) && escena == Escena::Combate && !world.is_over() {
            world.saltar_el_baile();
            intento.trampa = true;
            recorder = Recorder::for_world(&world);
            zumo.golpe_gordo();
            aviso = Some((tx(Texto::BaileSaltado).to_owned(), false, 120));
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
        // Teclado y mando se suman en vez de elegir uno, cada uno con sus
        // controles (ver `acciones.rs`).
        let input = if escena == Escena::Menu {
            InputFrame::NONE
        } else {
            entrada.juego(&progreso.opciones.controles)
        };
        let t0 = get_time();
        // Congelado —hitstop, cartela o entreacto— el acumulador no suma, y
        // no sale ni un tick: ni ahora ni al seguir.
        let steps = opciones::pasos(&mut accumulator, frame_dt, congelado, MAX_STEPS_PER_FRAME);
        // Los sucesos de todos los ticks del frame se juntan: puede haber
        // varios, y reaccionar solo al ultimo se comeria sonidos.
        let mut eventos = Events::default();
        for _ in 0..steps {
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
                Escena::Modista | Escena::Prologo | Escena::Libro => {}
                Escena::Paseo => {
                    if let Some((_, p)) = paseo.as_mut() {
                        let andando = !p.terminado();
                        p.step(input);
                        if andando {
                            intento.apuntar(&mut progreso.marcas, &p.events, 0);
                        }
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
                        let baile = pista.nodos[nodo_actual].jefe;
                        chispas.reaccionar_en(&p.events, j, golpe, boca, baile);
                        if p.events.super_fired {
                            ovacion.disparar(baile, Vec::new(), true);
                        }
                        if p.events.fichas > 0 {
                            chispas.estallido(j, paleta::ORO);
                        }
                        if p.caidos > 0 {
                            let (_, ropa) = paleta::del_baile(pista.nodos[nodo_actual].jefe);
                            // Sin sonido: con veinte por calle, un sonido por
                            // cada uno era excesivo.
                            chispas.estallido(golpe, ropa);
                        }
                    }
                }
                Escena::Combate => {
                    let antes = (world.player.pos.x, world.player.pos.y);
                    // Se apunta mientras se baila: el tick que gana cuenta y
                    // los de la pantalla de victoria no.
                    let bailando = !world.is_over();
                    // La foto de las balas para la ovacion, solo con el super
                    // pulsado: el tick que lo dispara lo lleva pulsado seguro,
                    // y despues de `step` ya no queda ninguna que fotografiar.
                    let foto = if input.is_down(InputFrame::SUPER) {
                        world
                            .bullets
                            .iter_live()
                            .map(|b| vec2(b.pos.x, b.pos.y))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    world.step(input);
                    if world.events.super_fired {
                        ovacion.disparar(world.baile, foto, false);
                    }
                    if bailando {
                        intento.apuntar(&mut progreso.marcas, &world.events, world.boss.phase);
                    }
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
            ovacion.step();
        }
        // La cartela y el fundido corren con el reloj de pared, no con los
        // ticks. Es presentacion —tiene que durar igual a 60 que a 144 Hz— y,
        // sobre todo, **tiene que avanzar mientras la cartela para el mundo**,
        // que es justo cuando no hay ticks. Meterlos en el bucle de simulacion
        // dejaba la pantalla en negro para siempre al entrar a un baile.
        // En el entreacto la cartela se queda donde estaba, como el mundo.
        fundido.step(frame_dt);
        if !parado
            && let Some(c) = cartela.as_mut()
            && !c.step(frame_dt)
        {
            // Al acabar la presentacion, el grito: el "WALLOP!" de Cuphead. Es
            // lo que convierte el final de una cartela en el principio de un
            // combate.
            cartela = c.es_de_entrada().then(|| match escena {
                Escena::Paseo => Cartela::grito(tx(Texto::EnMarcha)),
                _ => Cartela::a_bailar(),
            });
        }
        stats.push_sim((get_time() - t0) as f32);
        // La musica se busca por (jefe, figura). La pista tiene la suya
        // propia: dejar sonando el vals en el mapa decia que el vals era el
        // juego, y el vals es UN jefe del juego.
        //
        // Va despues de la simulacion y antes de los sonidos a proposito: en
        // el tick que cambia de figura, el tema nuevo ya esta puesto cuando
        // suena lo que toque, asi que los efectos en tono (la ficha, la
        // victoria) salen en el tono del tema que empieza.
        if hubo_interaccion {
            audio.poner_musica(match escena {
                Escena::Combate => Tema::de(world.baile, world.boss.phase),
                // En la calle suena la primera figura de su baile: el vals se
                // oye desde fuera, y al entrar sigue sonando sin cortarse.
                Escena::Paseo => Tema::de(pista.nodos[nodo_actual].jefe, 0),
                // Y la sala, con los instrumentos que le han devuelto los
                // bailes ganados. En el final suenan los cuatro: es el salon
                // entero, que es lo que cuenta el epilogo.
                _ => Tema::Sala(ganados(&pista)),
            });
        }
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
                    cartela = Some(Cartela::figura(
                        nombre(world.boss.phase_name()),
                        world.boss.phase,
                    ));
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
        // Fin de un intento ganado: la nota, las marcas y la tarjeta, una sola
        // vez. Tras leerla, aceptar vuelve a la pista: eso lo mira `salir`,
        // arriba, con el resto de los finales.
        if intento.tarjeta.is_none() {
            if escena == Escena::Combate && world.victory {
                intento.cerrar_baile(&mut progreso.marcas);
                progreso.guardar();
            } else if escena == Escena::Paseo
                && let Some((_, p)) = paseo.as_ref()
                && p.completado.is_some()
            {
                intento.cerrar_paseo(&mut progreso.marcas, (p.fichas_cogidas(), p.fichas.len()));
            }
        }

        // Fraccion de tick pendiente. Es lo que permite que el render vaya a
        // 144 Hz con la simulacion a 60 sin que se vea a saltos.
        let alpha = (accumulator / DT).clamp(0.0, 1.0);

        // --- Render ---
        let t1 = get_time();
        // El paseo es apaisado; todo lo demas, la arena de siempre.
        let lamina = match escena {
            Escena::Paseo | Escena::Prologo => draw::Layout::para(VISTA_W, VISTA_H),
            _ => draw::Layout::compute(),
        };
        let layout = lamina.sacudido(zumo.desplazamiento());
        // Todos los nombres de teclas y botones de este frame, del aparato que
        // se toco lo ultimo.
        let n = entrada.nombres(&progreso.opciones.controles);
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
            let teclas = [&n.aceptar[..], &n.pasear, &n.mover, &n.atras];
            let notas: Vec<_> = pista
                .nodos
                .iter()
                .map(|n| progreso.marcas.nota(&n.nombre))
                .collect();
            mapa::pista(&pista, alpha, &layout, &fichas, &notas, teclas);
        } else if escena == Escena::Prologo {
            historia.dibujar(&layout, &n);
        } else if escena == Escena::Libro {
            libro::pagina(&layout, &progreso, &bailes, &paseos, &n);
        } else if escena == Escena::Modista {
            tienda.dibujar(&progreso, &layout, &n);
        } else if escena == Escena::Paseo
            && let Some((_, p)) = paseo.as_ref()
        {
            let jefe = pista.nodos[nodo_actual].jefe;
            let (pulso, _) = salon::latido(Tema::de(jefe, 0), p.tick as f32 + alpha);
            calle::dibujar(p, alpha, &layout, pulso, jefe);
            let camara = layout.sacudido(-vec2(calle::camara(p, alpha), calle::camara_y(p, alpha)));
            let yo = p.jugadora.render_pos(alpha);
            ovacion.dibujar(
                &layout,
                vec2(VISTA_W, VISTA_H),
                camara.to_screen(yo.x, yo.y),
                alpha,
            );
            draw::particulas(&chispas, &camara);
            calle::encima(p, &layout, &n);
            if let Some(t) = intento.tarjeta.as_ref() {
                libro::tarjeta_de_paseo(t, nombre(&p.def.nombre), &lamina, &n);
            }
        } else {
            let mostrado = match (escena, attract.as_ref()) {
                (Escena::Menu, Some(a)) => &a.world,
                _ => &world,
            };
            let cayendo = match escena {
                Escena::Combate => jefes::caida(caida.unwrap_or(0.0)),
                _ => 1.0,
            };
            draw::frame(mostrado, alpha, &layout, bullets_gpu.as_mut(), cayendo, &n);
            if escena == Escena::Combate {
                let yo = world.player.render_pos(alpha);
                let tam = vec2(vals_core::ARENA_W, vals_core::ARENA_H);
                ovacion.dibujar(&layout, tam, layout.to_screen(yo.x, yo.y), alpha);
            }
            draw::particulas(&chispas, &layout);
            if let Some((donde, cuando)) = fantasma
                && escena == Escena::Combate
            {
                draw::fantasma(&world, donde, (get_time() - cuando) as f32, &layout);
            }
            if escena == Escena::Menu {
                draw::menu(&layout, mostrado, intentos, &n);
                libro::aviso_en_el_menu(&layout, &n);
                prologo::en_el_menu(&layout, &n, hay_final);
            } else if world.is_over() {
                draw::fin_de_partida(
                    &layout,
                    &world,
                    jefes::caida(caida.unwrap_or(0.0)),
                    intento.tarjeta.as_ref(),
                    &n,
                );
            }
        }
        if show_debug && escena == Escena::Combate {
            draw::debug_overlay(&world, &stats, steps, chispas.vivas(), &n);
            draw::recording_badge(recorder.ticks());
        }
        if let Some(c) = cartela.as_ref() {
            match escena {
                Escena::Paseo => c.dibujar_en(&layout, vec2(VISTA_W, VISTA_H)),
                _ => c.dibujar(&layout),
            }
        }
        if let Some(c) = carta.as_ref() {
            c.dibujar(&progreso.opciones, &n, &entrada);
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
        // Al cambiar de escena se guarda: el tiempo jugado y los golpes de un
        // intento que se deja a medias no tienen otro momento.
        if escena != escena_antes {
            progreso.guardar();
        }

        next_frame().await;
    }
}

/// Los bailes ganados, un bit por jefe: lo que la sala necesita para saber
/// que instrumentos le han devuelto.
fn ganados(pista: &Pista) -> u8 {
    pista
        .nodos
        .iter()
        .filter(|n| n.vencido)
        .fold(0, |m, n| m | 1u8.checked_shl(n.jefe as u32).unwrap_or(0))
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
        nombre(&world.boss.name),
        nombre(world.boss.phase_name()),
        nombre(Tema::de(world.baile, 0).titulo()),
    )
    .con_lema(prologo::lema_del_baile(world.baile));
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
        let n = Nombres::de(Dispositivo::Teclado, false, &Controles::default());
        draw::frame(&world, alpha, &layout, bullets_gpu.as_mut(), 1.0, &n);
        draw::debug_overlay(&world, &stats, steps, 0, &n);
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
