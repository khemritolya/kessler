extern crate noise;
extern crate x11rb;

use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::*;
use x11rb::protocol::*;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;

use noise::*;

use rand::prelude::*;

use std::cmp::*;
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

#[inline(always)]
fn set_if(data: &mut [u8], idx: usize, v: u8) {
    if data[idx] < v {
        data[idx] = v
    }
}

#[inline(always)]
fn set(data: &mut [u8], idx: usize, v: u8) {
    data[idx] = v
}

#[inline(always)]
fn put_if(data: &mut [u8], i: usize, j: usize, width: usize, rgba: (u8, u8, u8, u8)) {
    set_if(data, 8 * i * width + 8 * j, rgba.2); //b
    set_if(data, 8 * i * width + 8 * j + 1, rgba.1); // g
    set_if(data, 8 * i * width + 8 * j + 2, rgba.0); // r
    set_if(data, 8 * i * width + 8 * j + 3, rgba.3); // a

    set_if(data, 8 * i * width + 8 * j + 4, rgba.2); //b
    set_if(data, 8 * i * width + 8 * j + 5, rgba.1); // g
    set_if(data, 8 * i * width + 8 * j + 6, rgba.0); // r
    set_if(data, 8 * i * width + 8 * j + 7, rgba.3); // a

    set_if(data, 8 * i * width + 4 * width + 8 * j, rgba.2); //b
    set_if(data, 8 * i * width + 4 * width + 8 * j + 1, rgba.1); // g
    set_if(data, 8 * i * width + 4 * width + 8 * j + 2, rgba.0); // r
    set_if(data, 8 * i * width + 4 * width + 8 * j + 3, rgba.3); // a

    set_if(data, 8 * i * width + 4 * width + 8 * j + 4, rgba.2); //b
    set_if(data, 8 * i * width + 4 * width + 8 * j + 5, rgba.1); // g
    set_if(data, 8 * i * width + 4 * width + 8 * j + 6, rgba.0); // r
    set_if(data, 8 * i * width + 4 * width + 8 * j + 7, rgba.3); // a
}

#[inline(always)]
fn put(data: &mut [u8], i: usize, j: usize, width: usize, rgba: (u8, u8, u8, u8)) {
    set(data, 8 * i * width + 8 * j, rgba.2); //b
    set(data, 8 * i * width + 8 * j + 1, rgba.1); // g
    set(data, 8 * i * width + 8 * j + 2, rgba.0); // r
    set(data, 8 * i * width + 8 * j + 3, rgba.3); // a

    set(data, 8 * i * width + 8 * j + 4, rgba.2); //b
    set(data, 8 * i * width + 8 * j + 5, rgba.1); // g
    set(data, 8 * i * width + 8 * j + 6, rgba.0); // r
    set(data, 8 * i * width + 8 * j + 7, rgba.3); // a

    set(data, 8 * i * width + 4 * width + 8 * j, rgba.2); //b
    set(data, 8 * i * width + 4 * width + 8 * j + 1, rgba.1); // g
    set(data, 8 * i * width + 4 * width + 8 * j + 2, rgba.0); // r
    set(data, 8 * i * width + 4 * width + 8 * j + 3, rgba.3); // a

    set(data, 8 * i * width + 4 * width + 8 * j + 4, rgba.2); //b
    set(data, 8 * i * width + 4 * width + 8 * j + 5, rgba.1); // g
    set(data, 8 * i * width + 4 * width + 8 * j + 6, rgba.0); // r
    set(data, 8 * i * width + 4 * width + 8 * j + 7, rgba.3); // a
}

struct SceneData {
    stars: Vec<(i16, i16, i16)>,
}

fn repaint<C: Connection>(
    conn: &C,
    win: Window,
    gc: u32,
    screen: &Screen,
    image: &mut Image,
    noise: &dyn NoiseFn<[f64; 3]>,
    scene: &SceneData,
) -> Result<(), Box<dyn Error>> {
    let uwidth = screen.width_in_pixels as usize;
    let uheight = screen.height_in_pixels as usize;

    let data = image.data_mut();
    let delta = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % 10000000) as f64
        / 3000.;

    for j in 0..uheight {
        for i in 0..uwidth {
            data[4 * j * uwidth + 4 * i] = 0; //b
            data[4 * j * uwidth + 4 * i + 1] = 20; // g
            data[4 * j * uwidth + 4 * i + 2] = 40; // r
            data[4 * j * uwidth + 4 * i + 3] = 255; // a
        }
    }

    for s in scene.stars.iter() {
        for i in max(0, s.0 - 4 * s.2)..min(uwidth as i16 / 2, s.0 + 4 * s.2) {
            for j in max(0, s.1 - 4 * s.2)..min(uheight as i16 / 2, s.1 + 4 * s.2) {
                let d2 = (i - s.0) * (i - s.0) + (j - s.1) * (j - s.1) + 1;
                let off = f64::sin(delta + s.0 as f64 + s.1 as f64) * 128.;
                let b = ((256 * s.2 + off as i16) / d2).clamp(0, 255) as u8;

                put_if(data, j as usize, i as usize, uwidth, (b, b, b, 255))
            }
        }
    }

    let min_dim = uwidth.min(uheight) / 2;
    let center = min_dim as i32 / 2;

    for i in min_dim / 4..3 * min_dim / 4 {
        for j in min_dim / 4..3 * min_dim / 4 {
            let fv = noise.get([
                i as f64 / 20. + delta,
                j as f64 / 20. + f64::sin(i as f64 / min_dim as f64),
                delta,
            ]) * 128.
                + 128.;
            let fv = (fv * fv / 8.) as i32;
            let d2 = (i as i32 - center) * (i as i32 - center)
                + (j as i32 - center) * (j as i32 - center);
            let da = d2.saturating_sub(fv).max(1);
            let r = (2680000 / da).clamp(0, 255);

            if r == 255 {
                let fv = noise.get([
                    i as f64 / 20. - delta,
                    j as f64 / 20. - f64::sin(i as f64 / min_dim as f64),
                    delta - 100.,
                ]) * 128.
                    + 128.;
                let fv = (fv * fv / 10.) as i32;
                let da = d2.saturating_sub(fv).max(1);

                let g = 255 - (16800 / da).clamp(0, 255) as u8;
                put(data, j, i, uwidth, (255, g, 0, 255));
            } else {
                put_if(data, j, i, uwidth, (r as u8, (2 * r / 4) as u8, 0, 255));
            }
        }
    }

    image.put(conn, win, gc, 0, 0)?;

    conn.flush()?;

    //println!("repainted! {:?}", Instant::now());
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

    let noise = OpenSimplex::new();

    let scene = {
        let mut rng = rand::thread_rng();

        let mut stars = Vec::new();
        for _ in 0..(width as usize * height as usize / 2048) {
            let x = rng.gen_range(0..width / 2) as i16;
            let y = rng.gen_range(0..height / 2) as i16;
            let r = rng.gen_range(0..2) as i16;
            stars.push((x, y, r))
        }

        SceneData { stars }
    };

    loop {
        if let Some(event) = conn.poll_for_event()? {
            println!("Event: {:?}", event);
            match event {
                Event::Expose(_) => {
                    //repaint(&conn, win, pixmap, gc, screen, &mut color_map)?;
                }
                _ => (),
            }
        } else {
            let prev = Instant::now();
            repaint(&conn, win, gc, screen, &mut image, &noise, &scene)?;
            std::thread::sleep(Duration::from_millis(20));
            let after = Instant::now();
            println!("delta {:?}", after - prev);
        }
    }
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => println!("{:?}", e),
    }
}
