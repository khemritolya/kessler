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
    beg: (f32, f32),
    mid: (f32, f32),
    end: (f32, f32),
    start: Instant,
}

struct Curve {
    mid: (f32, f32),
    end: (f32, f32),
}

struct Root {
    pos: (f32, f32),
    color: (u8, u8, u8, u8),
}

struct SceneData {
    flakes: Vec<Flake>,
    curves: Vec<Box<dyn Fn(f32) -> Curve>>,
    roots: Vec<Root>,
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
    let fwidth = uwidth as f32;
    let fheight = uheight as f32;

    let data = image.data_mut();

    // I just want to zero all the bytes in the image.
    // And to do it fast, please.
    unsafe {
        let data_ptr = data.as_mut_ptr();
        std::ptr::write_bytes(data_ptr, 0, 4 * uwidth * uheight);
    }

    let lqx = fwidth / 4.;
    let lqy = fheight / 4.;

    let now = Instant::now();
    let gdt = now.duration_since(*start).as_secs_f32();
    for (i, root) in scene.roots.iter_mut().enumerate() {
        let i = i as f32;
        root.pos = (
            fwidth / 2.
                + lqx / 4.
                    * f32::cos(gdt / 15. + i * 2.718)
                    * (2. + f32::cos(2.718 * (gdt + i) / 15.)),
            fheight / 2.
                - lqy / 4.
                    * f32::sin(gdt / 15. + i * 2.718)
                    * (2. + f32::cos(2.718 * (gdt + i) / 15.)),
        );
    }

    let dt = Duration::from_secs(18);

    scene.flakes.retain(|f| now.duration_since(f.start) < dt);

    let initial_size = scene.flakes.len();

    let condition = |s| s < 120 + initial_size && s < 100000;

    while condition(scene.flakes.len()) {
        let root_idx = rand.gen_range(0..scene.roots.len());
        let root = &scene.roots[root_idx];

        let curve_idx = rand.gen_range(0..scene.curves.len());
        let curve = (&scene.curves[curve_idx])(gdt + 3.14 * root_idx as f32);

        let pos_0 = (
            root.pos.0 + rand.gen_range(-5.0..5.),
            root.pos.1 + rand.gen_range(-5.0..5.),
        );

        let pos_1 = (
            curve.mid.0 + rand.gen_range(-20.0..20.),
            curve.mid.1 + rand.gen_range(-20.0..20.),
        );
        let pos_2 = (
            curve.end.0 + rand.gen_range(-40.0..40.),
            curve.end.1 + rand.gen_range(-40.0..40.),
        );

        let color: (u8, u8, u8, u8) = rand.gen();
        let color = (
            color.0 / root.color.0,
            color.1 / root.color.1,
            color.2 / root.color.2,
            color.3 / root.color.3,
        );

        let flake = Flake {
            color: color,
            beg: pos_0,
            mid: pos_1,
            end: pos_2,
            start: now,
        };
        scene.flakes.push(flake);
    }

    for flake in scene.flakes.iter() {
        let dt = now.duration_since(flake.start).as_secs_f32() / 20.;

        // oh look, a B??zier curve
        let minus_dt = 1. - dt;
        let x = flake.mid.0
            + minus_dt * minus_dt * (flake.beg.0 - flake.mid.0)
            + dt * dt * (flake.end.0 - flake.mid.0);
        let y = flake.mid.1
            + minus_dt * minus_dt * (flake.beg.1 - flake.mid.1)
            + dt * dt * (flake.end.1 - flake.mid.1);

        let lx = if x - 1. < 0. { 0 } else { (x - 1.) as usize };
        let ox = if x + 1. > fwidth {
            uwidth
        } else {
            (x + 1.) as usize
        };

        let ly = if y - 1. < 0. { 0 } else { (y - 1.) as usize };
        let oy = if y + 1. > fheight {
            uheight
        } else {
            (y + 1.) as usize
        };

        let brightness = (minus_dt * 10.) as u8;

        for i in lx..ox {
            for j in ly..oy {
                data[4 * j * uwidth + 4 * i] =
                    data[4 * j * uwidth + 4 * i].saturating_add(flake.color.0 / 10 * brightness);
                data[4 * j * uwidth + 4 * i + 1] = data[4 * j * uwidth + 4 * i + 1]
                    .saturating_add(flake.color.1 / 10 * brightness);
                data[4 * j * uwidth + 4 * i + 2] = data[4 * j * uwidth + 4 * i + 2]
                    .saturating_add(flake.color.2 / 10 * brightness);
                data[4 * j * uwidth + 4 * i + 3] = data[4 * j * uwidth + 4 * i + 3]
                    .saturating_add(flake.color.3 / 10 * brightness);
            }
        }
    }

    image.put(conn, win, gc, 0, 0)?;

    //conn.flush()?;

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

    let fwidth = width as f32;
    let fheight = height as f32;

    let curves = {
        let mut curves = Vec::new();

        // let top = Box::new(move |t| Curve {
        //     mid: (
        //         fwidth / 2. * f32::sin(t / 3.141) + fwidth / 2.,
        //         fheight / 4.,
        //     ),
        //     end: (fwidth / 2. * f32::sin(t / 8.423 + 3.) + fwidth / 2., 0.),
        // }) as Box<dyn Fn(f32) -> Curve>;
        // curves.push(top);

        // let bottom = Box::new(move |t| Curve {
        //     mid: (
        //         fwidth / 2. * f32::sin(t / 6.44) + fwidth / 2.,
        //         3. * fheight / 4.,
        //     ),
        //     end: (fwidth / 2. * f32::sin(t / 10.5 - 1.) + fwidth / 2., fheight),
        // }) as Box<dyn Fn(f32) -> Curve>;
        // curves.push(bottom);

        // let left = Box::new(move |t| Curve {
        //     mid: (fwidth / 4., fheight / 2. * f32::cos(t / 16.) + fheight / 2.),
        //     end: (0., fheight / 2. * f32::cos(t / 2.713 - 2.) + fheight / 2.),
        // }) as Box<dyn Fn(f32) -> Curve>;
        // curves.push(left);

        // let right = Box::new(move |t| Curve {
        //     mid: (
        //         3. * fwidth / 4.,
        //         fheight / 2. * f32::cos(t / 4.112 + 4.) + fheight / 2.,
        //     ),
        //     end: (fwidth, fheight / 2. * f32::cos(t / 13.) + fheight / 2.),
        // }) as Box<dyn Fn(f32) -> Curve>;
        // curves.push(right);

        let circ = Box::new(move |t| Curve {
            mid: (
                fwidth / 3. * f32::cos(t / -5.0_f32 + 1.445_f32) + fwidth / 2.,
                fheight / 3. * f32::sin(t / -4.0_f32 + 4.221_f32) + fheight / 2.,
            ),
            end: (
                2. * fwidth / 3. * f32::cos(t / 10.0_f32) + fwidth / 2.,
                2. * fheight / 3. * f32::sin(t / 10.0_f32) + fheight / 2.,
            ),
        }) as Box<dyn Fn(f32) -> Curve>;
        curves.push(circ);

        let circ = Box::new(move |t| Curve {
            mid: (
                fwidth / 3. * f32::cos(t / 5.0_f32 + 4.0_f32) + fwidth / 2.,
                fheight / 3. * f32::sin(t / 5.0_f32 + 3.0_f32) + fheight / 2.,
            ),
            end: (
                2. * fwidth / 3. * f32::cos(t / 10.0_f32) + fwidth / 2.,
                2. * fheight / 3. * f32::sin(t / 10.0_f32) + fheight / 2.,
            ),
        }) as Box<dyn Fn(f32) -> Curve>;
        curves.push(circ);

        let circ = Box::new(move |t| Curve {
            mid: (
                fwidth / 3. * f32::cos(t / -5.0_f32) + fwidth / 2.,
                fheight / 3. * f32::sin(t / -5.0_f32) + fheight / 2.,
            ),
            end: (
                2. * fwidth / 3. * f32::cos(t / -10.0_f32) + fwidth / 2.,
                2. * fheight / 3. * f32::sin(t / -10.0_f32) + fheight / 2.,
            ),
        }) as Box<dyn Fn(f32) -> Curve>;
        curves.push(circ);

        let circ = Box::new(move |t| Curve {
            mid: (
                fwidth / 4. * f32::cos(t / 5.0_f32 + 5.731_f32) + fwidth / 2.,
                fheight / 4. * f32::sin(t / 5.0_f32 + 3.0_f32) + fheight / 2.,
            ),
            end: (
                fwidth / 2. * f32::cos(t / -10.0_f32 + 2.131_f32) + fwidth / 2.,
                fheight / 2. * f32::sin(t / -10.0_f32) + fheight / 2.,
            ),
        }) as Box<dyn Fn(f32) -> Curve>;
        curves.push(circ);

        curves
    };

    let roots = vec![
        Root {
            pos: (0., 0.),
            color: (1, 2, 2, 1),
        },
        Root {
            pos: (0., 0.),
            color: (4, 2, 1, 1),
        },
    ];

    let mut scene = SceneData {
        flakes: Vec::new(),
        curves,
        roots,
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
                    conn.flush()?;
                }
                Event::ConfigureNotify(_) => {
                    conn.flush()?;
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

            std::thread::sleep(Duration::from_millis(33_u64.saturating_sub(delta)));
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
