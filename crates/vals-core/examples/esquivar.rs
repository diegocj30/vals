//! Esquivar: un bot que esquiva de verdad, sin invulnerabilidad.
//!
//! Todos los demas bots del proyecto juegan invulnerables, asi que ninguno
//! dice si una figura **se puede esquivar**. Este si: cada pocos ticks clona
//! el mundo, prueba cada direccion (y quedarse quieta), con focus y sin el,
//! simula un rato hacia delante y se queda con la opcion que no la mata y la
//! deja mas lejos de la bala mas cercana. Dispara siempre y nunca suelta el
//! super ni parria: mide el esquive y nada mas.
//!
//! Juega cada figura de cada baile un rato fijo, con la vida del jefe repuesta
//! en cada tick para que no cambie de figura, y cuenta los golpes. Un golpe es
//! una bala a menos de `MARGEN` del hitbox, no solo una que lo toca: si no,
//! viendo el futuro entero, no le daria nada en ningun baile.
//!
//! ```text
//! cargo run -p vals-core --release --example esquivar
//! cargo run -p vals-core --release --example esquivar -- 12 15 b4
//! cargo test -p vals-core --release --example esquivar -- --ignored --nocapture
//! ```
//!
//! En release: cada clon del mundo copia el pool de balas entero (mas de un
//! mega), y el bot hace miles.

use vals_core::boss::BossDef;
use vals_core::bullets::BULLET_KINDS;
use vals_core::player::{PLAYER_HITBOX_RADIUS, RESPAWN_IFRAME_TICKS};
use vals_core::{ARENA_H, ARENA_W, InputFrame, Mode, World};

/// Ticks que se juega cada figura en cada intento: cuarenta segundos.
const TICKS: u32 = 2400;
/// Donde empieza cada intento. Uno solo es una tirada: el bot es caotico, un
/// paso distinto al principio cambia todo lo demas, y con un solo intento
/// una figura sacaba tres golpes o doce segun la suerte.
const SALIDAS: [f32; 3] = [200.0, 320.0, 440.0];
/// Cada cuantos ticks se vuelve a decidir.
const CADA: u32 = 5;
/// Cuanto mira hacia delante cada opcion.
const HORIZONTE: u32 = 40;
/// Por encima de esta holgura, una bala ya no preocupa y manda volver a casa.
const HOLGURA_DE_SOBRA: f32 = 32.0;
/// Lo que se le da por tocada aunque no la toque: una bala que pasa a menos
/// de esto del borde del hitbox cuenta como golpe. Es el pulso de una persona,
/// y lo que hace que el bot no sea perfecto: con margen cero ve el futuro
/// entero y no le da nada en ningun baile, ni en el flamenco de antes. Con 15
/// el bot necesita una calle de 35 entre bala y bala, lo que cita el
/// charleston para las suyas mas estrechas, y es donde los bailes empiezan a
/// separarse: el vals y el cancan casi no le dan, el tango si en los ochos.
pub const MARGEN: f32 = 15.0;

/// Lo que le falta a la bala mas cercana para tocar el hitbox.
fn holgura(w: &World) -> f32 {
    let p = w.player.pos;
    w.bullets
        .iter_live()
        .map(|b| b.pos.distance(p) - BULLET_KINDS[b.kind as usize].radius - PLAYER_HITBOX_RADIUS)
        .fold(f32::INFINITY, f32::min)
}

/// Si en este tick la ha tocado algo, de verdad o por menos del margen.
fn tocada(w: &World, margen: f32) -> bool {
    w.events.player_died || (!w.player.is_invulnerable() && holgura(w) < margen)
}

/// Los golpes que se lleva el bot en una figura, sumando los intentos.
pub fn golpes(baile: usize, fase: usize, margen: f32) -> u32 {
    std::thread::scope(|sc| {
        let hilos = SALIDAS.map(|x| sc.spawn(move || intento(baile, fase, margen, x)));
        hilos.into_iter().map(|h| h.join().unwrap()).sum()
    })
}

/// Un intento, empezando en `x`.
fn intento(baile: usize, fase: usize, margen: f32, x: f32) -> u32 {
    let mut w = World::empezar_en(7, Mode::Flight, baile);
    w.player.pos.x = x;
    while w.boss.phase < fase && !w.boss.defeated {
        let hp = w.boss.hp.max(1);
        w.boss.damage(hp);
    }
    let mut golpes = 0;
    let mut f = InputFrame::NONE;
    for t in 0..TICKS {
        if t % CADA == 0 {
            f = decidir(&w, margen);
        }
        w.boss.hp = w.boss.phase_max_hp();
        w.lives = 3;
        w.step(f);
        if tocada(&w, margen) {
            golpes += 1;
            // Lo mismo que hace el mundo al morir, si no ha muerto de verdad.
            w.bullets.clear();
            w.player.pos = World::spawn_pos();
            w.player.iframes = RESPAWN_IFRAME_TICKS;
        }
    }
    golpes
}

/// Las opciones: ocho direcciones y quieta, con focus y sin el. Cada una se
/// mantiene todo el horizonte o solo un paso corto y luego quieta: solo con
/// las largas, huir siempre parecia lo mas seguro y el bot acababa
/// arrinconado contra la pared de abajo.
fn opciones() -> Vec<(InputFrame, u32)> {
    let mut v = Vec::new();
    for focus in [false, true] {
        for (dx, dy) in [
            (0, 0),
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            let mut f = InputFrame::NONE;
            f.set(InputFrame::SHOOT, true);
            f.set(InputFrame::FOCUS, focus);
            f.set(InputFrame::LEFT, dx < 0);
            f.set(InputFrame::RIGHT, dx > 0);
            f.set(InputFrame::UP, dy < 0);
            f.set(InputFrame::DOWN, dy > 0);
            v.push((f, HORIZONTE));
            if (dx, dy) != (0, 0) {
                v.push((f, 2 * CADA));
            }
        }
    }
    v
}

/// La mejor opcion: la que aguanta mas sin golpe; entre las que aguantan
/// igual, la de mas holgura (redondeada, para que no decida el decimal); y
/// entre esas, la que acaba mas cerca de abajo en el centro.
fn decidir(w: &World, margen: f32) -> InputFrame {
    let casa = World::spawn_pos();
    let mut mejor = (InputFrame::NONE, (0, 0, f32::NEG_INFINITY));
    let quieta = InputFrame::from_bits(InputFrame::SHOOT);
    for (f, dura) in opciones() {
        let mut s = w.clone();
        let mut cerca = HOLGURA_DE_SOBRA;
        let mut aguanta = HORIZONTE;
        for t in 0..HORIZONTE {
            s.boss.hp = s.boss.phase_max_hp();
            s.step(if t < dura { f } else { quieta });
            if tocada(&s, margen) {
                aguanta = t;
                break;
            }
            // Las paredes cuentan como balas: sin esto el bot huia a una
            // esquina de abajo, donde todo llega lo ultimo, y alli lo
            // acorralaba el primer abanico que barria la esquina.
            let p = s.player.pos;
            let pared = p.x.min(ARENA_W - p.x).min(p.y).min(ARENA_H - p.y);
            cerca = cerca.min(holgura(&s)).min(pared);
        }
        let nota = (
            aguanta,
            (cerca / 4.0).floor() as i32,
            -s.player.pos.distance(casa),
        );
        if nota.partial_cmp(&mejor.1) == Some(std::cmp::Ordering::Greater) {
            mejor = (f, nota);
        }
    }
    mejor.0
}

/// Los golpes de todas las figuras de los bailes que se pidan (todos si no se
/// pide ninguno), en paralelo.
pub fn tabla(margen: f32, bailes: &[usize]) -> Vec<(String, Vec<(String, u32)>)> {
    let defs = BossDef::default_bosses();
    std::thread::scope(|sc| {
        let hilos: Vec<_> = defs
            .iter()
            .enumerate()
            .filter(|(i, _)| bailes.is_empty() || bailes.contains(i))
            .map(|(i, d)| {
                let figuras: Vec<_> = (0..d.phases.len())
                    .map(|fase| sc.spawn(move || golpes(i, fase, margen)))
                    .collect();
                (d, figuras)
            })
            .collect();
        hilos
            .into_iter()
            .map(|(d, figuras)| {
                let n = figuras.into_iter().map(|h| h.join().unwrap());
                let filas = d.phases.iter().map(|f| f.name.clone()).zip(n).collect();
                (d.name.clone(), filas)
            })
            .collect()
    })
}

/// Una columna por margen: `esquivar 9 12 15`; sin ninguno, solo `MARGEN`.
/// Y `b4` (o `b1 b3`) para jugar solo esos bailes, contando desde cero.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut margenes: Vec<f32> = args.iter().filter_map(|a| a.parse().ok()).collect();
    if margenes.is_empty() {
        margenes.push(MARGEN);
    }
    let bailes: Vec<usize> = args
        .iter()
        .filter_map(|a| a.strip_prefix('b')?.parse().ok())
        .collect();
    let tablas: Vec<_> = margenes.iter().map(|m| tabla(*m, &bailes)).collect();
    println!(
        "golpes del bot en {} intentos de {} s de cada figura\n",
        SALIDAS.len(),
        TICKS / 60
    );
    print!("{:<28}", "margen");
    for m in &margenes {
        print!("{m:>5}");
    }
    println!();
    for (i, (baile, filas)) in tablas[0].iter().enumerate() {
        println!("{baile}");
        for (j, (figura, _)) in filas.iter().enumerate() {
            print!("  {figura:<26}");
            for t in &tablas {
                print!("{:>5}", t[i].1[j].1);
            }
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El flamenco se esquiva: ninguna figura le cuesta al bot mas golpes que
    /// la peor del tango o del cancan, que son los dos mas duros de los justos.
    #[test]
    #[ignore]
    fn el_flamenco_se_esquiva() {
        let t = tabla(MARGEN, &[1, 3, 4]);
        let peor = |i: usize| t[i].1.iter().map(|(_, n)| *n).max().unwrap();
        let tope = peor(0).max(peor(1));
        for (figura, n) in &t[2].1 {
            assert!(*n <= tope, "{figura}: {n} golpes, el tope es {tope}");
        }
    }
}
