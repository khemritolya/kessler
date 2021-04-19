type Color = (u8, u8, u8, u8);

pub trait Viewable: std::fmt::Debug {
  fn dist2(&self, p: (f32, f32, f32), delta: f32) -> f32;
  fn color(&self) -> Color;
  fn clone(&self) -> Box<dyn Viewable>;
}

// x, y, z, rad
#[derive(Debug)]
pub struct Sphere(pub (f32, f32, f32), pub f32, pub Color);

impl Viewable for Sphere {
  fn dist2(&self, p: (f32, f32, f32), delta: f32) -> f32 {
    let dx = self.0 .0 - p.0;
    let dy = self.0 .1 - p.1 + delta / 2.;
    let dz = self.0 .2 - p.2;
    dx * dx + dy * dy + dz * dz - self.1 * self.1
  }

  fn color(&self) -> Color {
    self.2
  }

  fn clone(&self) -> Box<dyn Viewable> {
    Box::new(Sphere(self.0, self.1, self.2))
  }
}

#[derive(Clone, Debug)]
pub struct Ground(pub Color);

impl Viewable for Ground {
  fn dist2(&self, p: (f32, f32, f32), _delta: f32) -> f32 {
    let dz = p.1 + 25.;
    dz * dz
  }

  fn color(&self) -> Color {
    self.0
  }

  fn clone(&self) -> Box<dyn Viewable> {
    Box::new(Ground(self.0))
  }
}

fn ray_march(
  from: (f32, f32, f32),
  dir: (f32, f32, f32),
  scene: &[Box<dyn Viewable>],
  delta: f32,
) -> Option<Box<dyn Viewable>> {
  let mut x = from.0 + dir.0;
  let mut y = from.1 + dir.1;
  let mut z = from.2 + dir.2;

  let delta = f32::sin(delta);

  //println!("{}", delta);
  for _ in 0..100 {
    if x * x + y * y + z * z >= 5000. {
      return None;
    }

    let mut min_object = (f32::MAX, None);
    for o in scene {
      let dst2 = o.dist2((x, y, z), delta);
      if dst2 < min_object.0 {
        min_object = (dst2, Some(o));
      }
    }

    if min_object.0 < 0.5 {
      let object = (*min_object.1.unwrap()).clone();
      //println!("{:?}", object);
      return Some(object);
    }

    x += dir.0 * f32::sqrt(min_object.0);
    y += dir.1 * f32::sqrt(min_object.0);
    z += dir.2 * f32::sqrt(min_object.0);
  }

  return None;
}

pub fn get_color(
  x: f32,
  y: f32,
  width: f32,
  height: f32,
  scene: &[Box<dyn Viewable>],
  delta: f32,
) -> (u8, u8, u8, u8) {
  let min_dim = width.min(height);
  let x = (x - width / 2.) / (min_dim / 2.);
  let y = -(y - height / 2.) / (min_dim / 2.);
  let mag = f32::sqrt(x * x + y * y + 1.);
  match ray_march((0., 0., 0.), (x / mag, y / mag, 1. / mag), scene, delta) {
    None => (173, 216, 230, 255),
    Some(dr) => dr.color(),
  }
}
