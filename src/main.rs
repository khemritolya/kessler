extern crate x11rb;

use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::*;
use x11rb::protocol::*;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;

use std::error::Error;
use std::time::*;

use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::instance::Instance;
use vulkano::instance::InstanceExtensions;
use vulkano::instance::PhysicalDevice;

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

fn repaint<C: Connection>(
    conn: &C,
    win: Window,
    gc: u32,
    screen: &Screen,
    image: &mut Image,
) -> Result<(), Box<dyn Error>> {
    let uwidth = screen.width_in_pixels as usize;
    let uheight = screen.height_in_pixels as usize;
    let fwidth = uwidth as f32;
    let fheight = uheight as f32;

    let data = image.data_mut();
    let delta = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % 10000000) as f32
        / 3000.;

    for i in 0..(uheight / 2) {
        for j in 0..(uwidth / 2) {
            // in rgba
            let color = (0, 255, 0, 255);
            //tracer::get_color(j as f32, i as f32, fwidth / 2., fheight / 2., scene, delta);
            data[8 * i * uwidth + 8 * j] = color.2; //b
            data[8 * i * uwidth + 8 * j + 1] = color.1; // g
            data[8 * i * uwidth + 8 * j + 2] = color.0; // r
            data[8 * i * uwidth + 8 * j + 3] = color.3; // a

            data[8 * i * uwidth + 8 * j + 4] = color.3; //b
            data[8 * i * uwidth + 8 * j + 5] = color.1; // g
            data[8 * i * uwidth + 8 * j + 6] = color.0; // r
            data[8 * i * uwidth + 8 * j + 7] = color.3; // a

            data[8 * i * uwidth + 4 * uwidth + 8 * j] = color.2; //b
            data[8 * i * uwidth + 4 * uwidth + 8 * j + 1] = color.1; // g
            data[8 * i * uwidth + 4 * uwidth + 8 * j + 2] = color.0; // r
            data[8 * i * uwidth + 4 * uwidth + 8 * j + 3] = color.3; // a

            data[8 * i * uwidth + 4 * uwidth + 8 * j + 4] = color.2; //b
            data[8 * i * uwidth + 4 * uwidth + 8 * j + 5] = color.1; // g
            data[8 * i * uwidth + 4 * uwidth + 8 * j + 6] = color.0; // r
            data[8 * i * uwidth + 4 * uwidth + 8 * j + 7] = color.3; // a
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

    let instance = Instance::new(None, &InstanceExtensions::none(), None)?;
    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .ok_or("No Physical Device")?;
    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics())
        .ok_or("No proper graphics family")?;

    let (device, mut queues) = {
        Device::new(
            physical,
            &Features::none(),
            &DeviceExtensions::none(),
            [(queue_family, 0.5)].iter().cloned(),
        )?
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
            repaint(&conn, win, gc, screen, &mut image)?;
            //std::thread::sleep(Duration::from_millis(100));
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
