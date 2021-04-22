extern crate rand;
extern crate x11rb;

use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::*;
use x11rb::protocol::*;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;

use rand::prelude::*;

use std::error::Error;
use std::time::*;

atom_manager! {
    pub Atoms: AtomsCookie {
        UTF8_STRING,
        _NET_WM_NAME,
        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_DESKTOP,
    }
}

fn create_window<C: Connection>(
    conn: &C,
    win: Window,
    screen: &Screen,
    atoms: &Atoms,
) -> Result<(), Box<dyn Error>> {
    let win_aux = CreateWindowAux::new()
        .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY | EventMask::NO_EVENT)
        .background_pixel(0x00000000);

    conn.create_window(
        COPY_DEPTH_FROM_PARENT,
        win,
        screen.root,
        0,
        0,
        screen.width_in_pixels,
        screen.height_in_pixels,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &win_aux,
    )?;

    conn.change_property8(
        PropMode::REPLACE,
        win,
        atoms._NET_WM_NAME,
        atoms.UTF8_STRING,
        "kessler".as_bytes(),
    )?;

    conn.change_property32(
        PropMode::REPLACE,
        win,
        atoms._NET_WM_WINDOW_TYPE,
        AtomEnum::ATOM,
        &[atoms._NET_WM_WINDOW_TYPE_DESKTOP],
    )?;

    conn.map_window(win)?;
    conn.flush()?;

    Ok(())
}

struct Flake {
    color: (u8, u8, u8, u8),
    beg: (f64, f64),
    mid: (f64, f64),
    end: (f64, f64),
    start: Instant,
}

struct Curve {
    mid: (f64, f64),
    end: (f64, f64),
}

struct SceneData {
    flakes: Vec<Flake>,
    curves: Vec<Box<dyn Fn(f64) -> Curve>>,
    root: (f64, f64),
}

fn repaint<C: Connection>(
    conn: &C,
    win: Window,
    gc: u32,
    screen: &Screen,
    image: &mut Image,
    rand: &mut ThreadRng,
    start: &Instant,
    scene: &mut SceneData,
) -> Result<(), Box<dyn Error>> {
    let uwidth = screen.width_in_pixels as usize;
    let uheight = screen.height_in_pixels as usize;
    let fwidth = uwidth as f64;
    let fheight = uheight as f64;

    let data = image.data_mut();

    unsafe {
        let data_ptr = data.as_mut_ptr();
        std::ptr::write_bytes(data_ptr, 0, 4 * uwidth * uheight);
    }

    let lqx = fwidth / 4.;
    let lqy = fheight / 4.;

    let now = Instant::now();
    let gdt = now.duration_since(*start).as_secs_f64();
    scene.root = (
        fwidth / 2. + lqx / 2. * f64::cos(gdt / 5.),
        fheight / 2. - lqy / 2. * f64::sin(gdt / 5.),
    );

    let dt = Duration::from_secs(11);

    scene.flakes.retain(|f| now.duration_since(f.start) < dt);

    let initial_size = scene.flakes.len();

    let condition = |s| s < 50 || (s < 50 + initial_size && s < 200000);

    while condition(scene.flakes.len()) {
        let curve_idx = rand.gen_range(0..scene.curves.len());
        let curve = (&scene.curves[curve_idx])(gdt);

        let pos_0 = (
            scene.root.0 + rand.gen_range(-5.0..5.),
            scene.root.1 + rand.gen_range(-5.0..5.),
        );

        let pos_1 = (
            curve.mid.0 + rand.gen_range(-20.0..20.),
            curve.mid.1 + rand.gen_range(-20.0..20.),
        );
        let pos_2 = (
            curve.end.0 + rand.gen_range(-60.0..60.),
            curve.end.1 + rand.gen_range(-60.0..60.),
        );

        let flake = Flake {
            color: rand.gen(),
            beg: pos_0,
            mid: pos_1,
            end: pos_2,
            start: now,
        };
        scene.flakes.push(flake);
    }

    for flake in scene.flakes.iter() {
        let dt = now.duration_since(flake.start).as_secs_f64();

        // oh look, a Bézier curve
        let minus_dt = 1. - dt / 10.;
        let dt = dt / 10.;
        let x = flake.mid.0
            + minus_dt * minus_dt * (flake.beg.0 - flake.mid.0)
            + dt * dt * (flake.end.0 - flake.mid.0);
        let y = flake.mid.1
            + minus_dt * minus_dt * (flake.beg.1 - flake.mid.1)
            + dt * dt * (flake.end.1 - flake.mid.1);

        let lx = if x - 2. < 0. { 0 } else { (x - 2.) as usize };
        let ox = if x + 2. > fwidth {
            uwidth
        } else {
            (x + 2.) as usize
        };

        let ly = if y - 2. < 0. { 0 } else { (y - 2.) as usize };
        let oy = if y + 2. > fheight {
            uheight
        } else {
            (y + 2.) as usize
        };

        for i in lx..ox {
            for j in ly..oy {
                data[4 * j * uwidth + 4 * i] =
                    data[4 * j * uwidth + 4 * i].saturating_add(flake.color.0);
                data[4 * j * uwidth + 4 * i + 1] =
                    data[4 * j * uwidth + 4 * i + 1].saturating_add(flake.color.1 / 4);
                data[4 * j * uwidth + 4 * i + 2] =
                    data[4 * j * uwidth + 4 * i + 2].saturating_add(flake.color.2 / 2);
                data[4 * j * uwidth + 4 * i + 3] =
                    data[4 * j * uwidth + 4 * i + 3].saturating_add(flake.color.3);
            }
        }
    }

    image.put(conn, win, gc, 0, 0)?;

    conn.flush()?;

    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];

    let win = conn.generate_id()?;

    let atoms = Atoms::new(&conn)?.reply()?;

    create_window(&conn, win, &screen, &atoms)?;

    let width = screen.width_in_pixels;
    let height = screen.height_in_pixels;

    let gc = conn.generate_id()?;

    conn.create_gc(
        gc,
        screen.root,
        &CreateGCAux::default().graphics_exposures(0),
    )?;

    let mut image = Image::allocate_native(width, height, 24, &conn.setup())?;

    let fwidth = width as f64;
    let fheight = height as f64;

    let curves = {
        let mut curves = Vec::new();

        let top = Box::new(move |t| Curve {
            mid: (fwidth / 2. * f64::sin(t / 5.) + fwidth / 2., fheight / 4.),
            end: (fwidth / 2. * f64::sin(t / 10. + 3.) + fwidth / 2., 0.),
        }) as Box<dyn Fn(f64) -> Curve>;
        curves.push(top);

        let bottom = Box::new(move |t| Curve {
            mid: (
                fwidth / 2. * f64::sin(t / 6.) + fwidth / 2.,
                3. * fheight / 4.,
            ),
            end: (fwidth / 2. * f64::sin(t / 12. - 1.) + fwidth / 2., fheight),
        }) as Box<dyn Fn(f64) -> Curve>;
        curves.push(bottom);

        let left = Box::new(move |t| Curve {
            mid: (fwidth / 4., fheight / 2. * f64::cos(t / 16.) + fheight / 2.),
            end: (0., fheight / 2. * f64::cos(t / 8. - 2.) + fheight / 2.),
        }) as Box<dyn Fn(f64) -> Curve>;
        curves.push(left);

        let right = Box::new(move |t| Curve {
            mid: (
                3. * fwidth / 4.,
                fheight / 2. * f64::cos(t / 5. + 4.) + fheight / 2.,
            ),
            end: (fwidth, fheight / 2. * f64::cos(t / 10.) + fheight / 2.),
        }) as Box<dyn Fn(f64) -> Curve>;
        curves.push(right);

        let circ = Box::new(move |t| Curve {
            mid: (
                fwidth / 4. * f64::cos(t / 5. + 4.) + fwidth / 2.,
                fheight / 4. * f64::sin(t / 5. + 4.) + fheight / 2.,
            ),
            end: (
                fwidth / 2. * f64::cos(t / 10.) + fwidth / 2.,
                fheight / 2. * f64::sin(t / 10.) + fheight / 2.,
            ),
        }) as Box<dyn Fn(f64) -> Curve>;
        curves.push(circ);

        curves
    };

    let mut scene = SceneData {
        flakes: Vec::new(),
        curves,
        root: (width as f64 / 2., height as f64 / 2.),
    };

    let mut rng = thread_rng();

    let start = Instant::now();

    loop {
        if let Some(event) = conn.poll_for_event()? {
            println!("Event: {:?}", event);
            match event {
                Event::Expose(_) => {
                    repaint(
                        &conn, win, gc, screen, &mut image, &mut rng, &start, &mut scene,
                    )?;
                }
                Event::ConfigureNotify(_) => {
                    // TODO: close?
                }
                _ => (),
            }
        } else {
            let prev = Instant::now();
            repaint(
                &conn, win, gc, screen, &mut image, &mut rng, &start, &mut scene,
            )?;
            let after = Instant::now();
            let delta = after.duration_since(prev).subsec_millis() as u64;

            std::thread::sleep(Duration::from_millis(25_u64.saturating_sub(delta)));
            let fin = Instant::now();
            println!("delta {:?},\t work {:?}", fin - prev, after - prev);
        }
    }
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => println!("{:?}", e),
    }
}
