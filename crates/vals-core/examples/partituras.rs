//! Partituras: la foto de larga exposicion de cada figura.
//!
//! Simula cada figura de cada baile con una jugadora quieta e invulnerable
//! abajo en el centro, sin disparar, y deja el obturador abierto unos segundos:
//! cada bala que sale en ese rato deja su trayectoria entera, con un punto cada
//! pocos ticks, y al cerrar se imprime encima la foto fija del ultimo instante.
//! Sale un SVG por figura en `docs/media/partituras/<baile>-<n>.svg`.
//!
//! ```text
//! cargo run -p vals-core --example partituras
//! ```
//!
//! Es determinista: la misma semilla y los mismos RON dan los mismos bytes, asi
//! que relanzarlo sin tocar los patrones no cambia nada en git, y despues de
//! reafinar un baile el diff de los SVG es el del baile.
//!
//! Como se lee:
//!
//! - Las trazas mas vivas son las balas mas nuevas. En un baile que gira, el
//!   brillo da la vuelta alrededor del jefe: eso es el `Turn` del vals.
//! - Los puntos van a intervalos fijos de tiempo, asi que donde se apelotonan
//!   la bala iba despacio: la frenada del tango se ve ahi.
//! - Una traza que se tuerce es una bala que curva (el `spin` del charleston),
//!   y una que se corta en seco, una que se desvanecio (el `ttl` del cancan).

use std::fmt::Write as _;
use std::path::Path;

use glam::Vec2;
use vals_core::boss::BossDef;
use vals_core::bullets::{BULLET_KINDS, FLAG_PARRYABLE, KIND_NEEDLE};
use vals_core::{ARENA_H, ARENA_W, DT, InputFrame, MAX_BULLETS, Mode, World};

const SALIDA: &str = "docs/media/partituras";

/// Ticks de figura antes de abrir el obturador: que la sala se llene y el
/// jefe haya dado al menos una vuelta a su camino.
const CALENTAR: u32 = 240;
/// Cada cuantos ticks deja un punto cada bala.
const CADA: u32 = 4;
/// Opacidad de las trazas por antiguedad, de la bala mas vieja a la mas nueva.
const OPACIDAD: [f32; 4] = [0.2, 0.32, 0.48, 0.7];
/// Y la de la foto fija, con las mismas tandas.
const OPACIDAD_FOTO: [f32; 4] = [0.4, 0.6, 0.8, 1.0];

/// Donde se planta la jugadora. Las balas apuntadas van aqui.
const JUGADORA: Vec2 = Vec2::new(320.0, 680.0);

/// Margen de papel alrededor de la arena, cabecera y pie.
const MARGEN: f32 = 40.0;
const CABECERA: f32 = 84.0;
const PIE: f32 = 56.0;

// La paleta del cartel, la misma que `vals-app/src/paleta.rs` y `draw.rs`.
// Copiada y no importada: este crate no puede depender de la app.
const PAPEL: &str = "#e0d3b6";
const TINTA: &str = "#1a1418";
const TINTA_TENUE: &str = "#706054";
const TARIMA: &str = "#362521";
const LUZ: &str = "#ffd494";

/// Color de cada tipo de bala (pequena, media, grande, aguja) y el rosa de las
/// que se parrean, que va el ultimo.
const COLOR_BALA: [&str; 5] = ["#f0e4c4", "#f0963e", "#60b0a8", "#fff0aa", "#ee709e"];
/// Grosor de la traza de cada clase. Los puntos miden 2,4 veces esto.
const GROSOR_TRAZA: [f32; 5] = [1.1, 1.6, 2.4, 0.9, 1.6];
/// Orden de pintado: lo fino debajo, lo gordo y lo rosa encima.
const ORDEN: [usize; 5] = [3, 0, 1, 2, 4];

struct Baile {
    slug: &'static str,
    /// Su tinta, la primera de `paleta::TINTAS`.
    tinta: &'static str,
    /// El verbo del motor que es suyo y de nadie mas.
    verbo: &'static str,
    /// Ticks con el obturador abierto. El vals llena la sala de agujas enseguida
    /// y con mas tiempo la foto seria niebla; el tango necesita que la frenada
    /// llegue a volver.
    exponer: u32,
}

const BAILES: [Baile; 4] = [
    Baile {
        slug: "vals",
        tinta: "#5884be",
        verbo: "Turn \u{b7} la mira gira",
        exponer: 110,
    },
    Baile {
        slug: "tango",
        tinta: "#c43a42",
        verbo: "accel negativa \u{b7} frena y vuelve",
        exponer: 150,
    },
    Baile {
        slug: "charleston",
        tinta: "#e2b242",
        verbo: "spin \u{b7} la bala curva",
        exponer: 150,
    },
    Baile {
        slug: "cancan",
        tinta: "#e8608e",
        verbo: "ttl corto \u{b7} la patada se desvanece",
        exponer: 120,
    },
];

fn main() -> std::io::Result<()> {
    let dir = Path::new(SALIDA);
    std::fs::create_dir_all(dir)?;
    for (i, def) in BossDef::default_bosses().iter().enumerate() {
        let baile = &BAILES[i.min(BAILES.len() - 1)];
        for (fase, f) in def.phases.iter().enumerate() {
            let svg = partitura(i, fase, baile, &def.name, &f.name, def.phases.len());
            let ruta = dir.join(format!("{}-{}.svg", baile.slug, fase + 1));
            std::fs::write(&ruta, &svg)?;
            println!("{} ({} KB)", ruta.display(), svg.len() / 1024);
        }
    }
    Ok(())
}

/// Clase de pintado de una bala: su tipo, o 4 si es rosa.
fn clase(kind: u8, flags: u8) -> usize {
    if flags & FLAG_PARRYABLE != 0 {
        4
    } else {
        usize::from(kind).min(3)
    }
}

/// Redondeo a entero, sin el "-0" de los flotantes.
fn r(x: f32) -> i32 {
    x.round() as i32
}

/// La trayectoria de una bala, ya como trozo de `d` de un `<path>`.
struct Traza {
    clase: usize,
    nacio: u32,
    d: String,
    ultima: Vec2,
    vel: Vec2,
    kind: u8,
    apuntada: bool,
}

impl Traza {
    fn punto(&mut self, p: Vec2) {
        let _ = write!(self.d, " {} {}", r(p.x), r(p.y));
        self.apuntada = true;
    }
}

fn partitura(baile: usize, fase: usize, b: &Baile, jefe: &str, figura: &str, n: usize) -> String {
    let mut w = World::empezar_en(7, Mode::Flight, baile);
    while w.boss.phase < fase && !w.boss.defeated {
        let hp = w.boss.hp.max(1);
        w.boss.damage(hp);
    }

    // Un hueco del pool vivo en dos ticks seguidos es la misma bala: asi se
    // sigue a cada una sin que el nucleo tenga que saber nada de esto.
    let mut viva_antes = vec![false; MAX_BULLETS];
    let mut traza_de: Vec<Option<usize>> = vec![None; MAX_BULLETS];
    let mut trazas: Vec<Traza> = Vec::new();
    let mut camino = String::new();

    for t in 0..CALENTAR + b.exponer {
        w.player.pos = JUGADORA;
        w.player.iframes = 10;
        w.step(InputFrame::default());

        if t >= 60 && t.is_multiple_of(2) {
            let p = w.boss.pos;
            let sep = if camino.is_empty() { "M" } else { " " };
            let _ = write!(camino, "{sep}{} {}", r(p.x), r(p.y));
        }

        let mut viva = vec![false; MAX_BULLETS];
        for (i, v) in w.bullets.iter_live_slots() {
            let i = i as usize;
            viva[i] = true;
            match traza_de[i] {
                Some(k) => {
                    let tr = &mut trazas[k];
                    tr.ultima = v.pos;
                    tr.vel = v.vel;
                    tr.apuntada = false;
                    if (t - tr.nacio).is_multiple_of(CADA) {
                        tr.punto(v.pos);
                    }
                }
                // Solo las que nacen con el obturador abierto.
                None if !viva_antes[i] && t >= CALENTAR => {
                    traza_de[i] = Some(trazas.len());
                    trazas.push(Traza {
                        clase: clase(v.kind, v.flags),
                        nacio: t,
                        d: format!("M{} {}", r(v.pos.x), r(v.pos.y)),
                        ultima: v.pos,
                        vel: v.vel,
                        kind: v.kind,
                        apuntada: true,
                    });
                }
                None => {}
            }
        }
        // Las que se han ido este tick: se cierra su traza donde se las vio.
        for (i, k) in traza_de.iter_mut().enumerate() {
            if !viva[i]
                && let Some(j) = k.take()
            {
                let tr = &mut trazas[j];
                if !tr.apuntada {
                    let p = tr.ultima;
                    tr.punto(p);
                }
            }
        }
        viva_antes = viva;
    }
    for tr in trazas.iter_mut().filter(|tr| !tr.apuntada) {
        let p = tr.ultima;
        tr.punto(p);
    }

    // Un <path> por antiguedad y clase: lo que deja el fichero en decenas de
    // KB y no en megas.
    let edad = |tr: &Traza| {
        let e = ((tr.nacio - CALENTAR) as usize * OPACIDAD.len()) / b.exponer as usize;
        e.min(OPACIDAD.len() - 1)
    };
    let mut capas = vec![vec![String::new(); 5]; OPACIDAD.len()];
    for tr in &trazas {
        // Una traza por linea: al reafinar un baile, el diff va bala a bala.
        let capa = &mut capas[edad(tr)][tr.clase];
        capa.push_str(&tr.d);
        capa.push('\n');
    }

    // El fogonazo al cerrar el obturador: las balas de la foto que siguen
    // vivas, donde estan, tambien mas tenues cuanto mas viejas. Las de antes de
    // abrir no salen, para que se lea.
    let mut foto = vec![vec![String::new(); 5]; OPACIDAD.len()];
    for tr in traza_de.iter().flatten().map(|&k| &trazas[k]) {
        let (p, s) = (tr.ultima, &mut foto[edad(tr)][tr.clase]);
        if tr.kind == KIND_NEEDLE {
            let d = tr.vel.normalize_or_zero() * -10.0;
            let _ = write!(s, "M{} {}l{} {}", r(p.x), r(p.y), r(d.x), r(d.y));
        } else {
            let _ = write!(s, "M{} {}h0", r(p.x), r(p.y));
        }
    }

    let (ancho, alto) = (ARENA_W + 2.0 * MARGEN, CABECERA + ARENA_H + PIE + MARGEN);
    let (ax, ay) = (MARGEN, CABECERA + MARGEN);
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 {ancho} {alto}" width="{ancho}" height="{alto}" font-family="Georgia, 'Times New Roman', serif">"#
    );
    let _ = write!(
        s,
        r#"<defs><clipPath id="arena"><rect width="{ARENA_W}" height="{ARENA_H}" rx="6"/></clipPath><radialGradient id="foco" cx="0.5" cy="0.12" r="0.75"><stop offset="0" stop-color="{LUZ}" stop-opacity="0.16"/><stop offset="1" stop-color="{LUZ}" stop-opacity="0"/></radialGradient>"#
    );
    // Los puntos de las trazas, uno por clase y en su color.
    for (c, color) in COLOR_BALA.iter().enumerate() {
        let _ = write!(
            s,
            r#"<marker id="p{c}" viewBox="-1 -1 2 2" markerWidth="2.4" markerHeight="2.4"><circle r="1" fill="{color}"/></marker>"#
        );
    }
    let _ = writeln!(s, "</defs>");
    // El papel, con el doble filete de un cartel.
    let _ = writeln!(
        s,
        r#"<rect width="{ancho}" height="{alto}" fill="{PAPEL}"/>"#
    );
    let _ = writeln!(
        s,
        r#"<rect x="12" y="12" width="{}" height="{}" fill="none" stroke="{TINTA}" stroke-width="2.5"/><rect x="18" y="18" width="{}" height="{}" fill="none" stroke="{TINTA}" stroke-width="0.8"/>"#,
        ancho - 24.0,
        alto - 24.0,
        ancho - 36.0,
        alto - 36.0
    );
    // La cabecera: el baile en versalitas y la figura grande.
    let _ = writeln!(
        s,
        r#"<text x="{MARGEN}" y="{}" font-size="15" letter-spacing="3" fill="{TINTA_TENUE}">{} <tspan fill="{}">&#9670;</tspan> FIGURA {} DE {n}</text>"#,
        MARGEN + 18.0,
        jefe.to_uppercase(),
        b.tinta,
        fase + 1
    );
    let _ = writeln!(
        s,
        r#"<text x="{MARGEN}" y="{}" font-size="40" font-style="italic" fill="{TINTA}">{}</text>"#,
        MARGEN + 64.0,
        figura.replace('&', "&amp;")
    );

    // La arena: la lamina oscura donde brillan las balas.
    let _ = writeln!(s, r#"<g transform="translate({ax} {ay})">"#);
    let _ = writeln!(
        s,
        r#"<rect width="{ARENA_W}" height="{ARENA_H}" rx="6" fill="{TARIMA}"/><rect width="{ARENA_W}" height="{ARENA_H}" rx="6" fill="url(#foco)"/>"#
    );
    let _ = writeln!(
        s,
        r#"<g clip-path="url(#arena)" fill="none" stroke-linecap="round" stroke-linejoin="round">"#
    );
    for (edad, porclase) in capas.iter().enumerate() {
        let _ = writeln!(s, r#"<g opacity="{}">"#, OPACIDAD[edad]);
        for &c in &ORDEN {
            if !porclase[c].is_empty() {
                let _ = writeln!(
                    s,
                    r#"<path stroke="{}" stroke-width="{}" marker-start="url(#p{c})" marker-mid="url(#p{c})" marker-end="url(#p{c})" d="{}"/>"#,
                    COLOR_BALA[c], GROSOR_TRAZA[c], porclase[c]
                );
            }
        }
        let _ = writeln!(s, "</g>");
    }
    // El camino del jefe, encima de las trazas: es el dibujo que hace en la
    // pista. Con un bajo claro, que la tinta del tango sobre la tarima no se ve.
    let _ = writeln!(
        s,
        r#"<path d="{camino}" stroke="{PAPEL}" stroke-width="7" stroke-opacity="0.16"/><path d="{camino}" stroke="{}" stroke-width="3" stroke-dasharray="7 6"/>"#,
        b.tinta
    );
    // La foto fija: cada bala con su contorno de tinta, como en el juego.
    for (edad, porclase) in foto.iter().enumerate() {
        let _ = writeln!(s, r#"<g opacity="{}">"#, OPACIDAD_FOTO[edad]);
        for &c in &ORDEN {
            if porclase[c].is_empty() {
                continue;
            }
            let radio = BULLET_KINDS[if c == 4 { 1 } else { c }].draw_radius;
            let grueso = if c == 3 { 3.0 } else { radio * 1.6 };
            let _ = writeln!(s, r#"<path id="f{edad}{c}" d="{}"/>"#, porclase[c]);
            let _ = writeln!(
                s,
                r##"<use xlink:href="#f{edad}{c}" stroke="{TINTA}" stroke-width="{}"/><use xlink:href="#f{edad}{c}" stroke="{}" stroke-width="{grueso}"/>"##,
                grueso + 2.5,
                COLOR_BALA[c]
            );
        }
        let _ = writeln!(s, "</g>");
    }
    // El jefe donde se ha quedado, con su aro de golpeo, y la jugadora.
    let p = w.boss.pos;
    let _ = writeln!(
        s,
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{TINTA}" stroke-width="3"/><circle cx="{}" cy="{}" r="{}" stroke="{}" stroke-width="1.5" stroke-dasharray="4 5"/>"#,
        r(p.x),
        r(p.y),
        w.boss.radius * 0.5,
        b.tinta,
        r(p.x),
        r(p.y),
        w.boss.radius,
        b.tinta
    );
    let _ = writeln!(
        s,
        r#"<path d="M{x} {}l9 9l-9 9l-9-9z" fill="{}" stroke="{TINTA}" stroke-width="2"/>"#,
        JUGADORA.y - 9.0,
        COLOR_BALA[0],
        x = JUGADORA.x
    );
    let _ = writeln!(s, "</g>");
    let _ = writeln!(
        s,
        r#"<rect width="{ARENA_W}" height="{ARENA_H}" rx="6" fill="none" stroke="{TINTA}" stroke-width="3"/>"#
    );
    let _ = writeln!(s, "</g>");

    // El pie: el verbo del baile y lo que dura la foto.
    let pie = ay + ARENA_H + 36.0;
    let _ = writeln!(
        s,
        r#"<rect x="{MARGEN}" y="{}" width="14" height="14" fill="{}" stroke="{TINTA}" stroke-width="1.5"/>"#,
        pie - 12.0,
        b.tinta
    );
    let _ = writeln!(
        s,
        r#"<text x="{}" y="{pie}" font-size="17" font-style="italic" fill="{TINTA}">{}</text>"#,
        MARGEN + 24.0,
        b.verbo
    );
    let _ = writeln!(
        s,
        r#"<text x="{}" y="{pie}" font-size="14" text-anchor="end" fill="{TINTA_TENUE}">{} s de exposicion</text>"#,
        ancho - MARGEN,
        format!("{:.1}", b.exponer as f32 * DT).replace('.', ",")
    );
    s.push_str("</svg>\n");
    s
}
