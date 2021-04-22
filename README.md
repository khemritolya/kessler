<h1 align="center">Kessler</h1>

![Rust CI](https://github.com/khemritolya/kessler/workflows/Rust/badge.svg) ![License](https://img.shields.io/github/license/khemritolya/kessler) ![heehoo](https://img.shields.io/static/v1?label=Requires&message=X11&color=important)


Maybe you remember the old Apple screen saver. This is an attempt at recreating something like it, except as an animated desktop background for the X Window System. Made for personal use -- the color scheme ~~might be~~ is probably broken on your end.

Licensed under the [MIT License](./LICENSE).

<p align="center">

  ![](https://forthebadge.com/images/badges/made-with-rust.svg) ![](https://forthebadge.com/images/badges/contains-technical-debt.svg) ![](https://forthebadge.com/images/badges/designed-in-ms-paint.svg)

</p>

<h3 align="center">A Low Res Preview:</h3>

<p align="center">
  <img src="./.github/assets/preview.gif" alt="Preview" style="text-align: center" width="600px"/>
</p>

### Built using

- [x11rb](https://github.com/psychon/x11rb)
- [rand](https://github.com/rust-random/rand)

### Future work:

- Fix the color scheme badness going on right now.
- Installation? (i.e. optional start-on-login) 
- Configuration:
  - General config: FPS target, particile settings
  - Specify the number of roots/curves, their colors, etc.
  - Possible integration with `rhai` to allow user-defined curves?.
- Further optimizations:
  - More memset in `unsafe` code probably. It seems that writes to the image I'm using as a buffer are what is holding this back, according to [perf](https://perf.wiki.kernel.org/index.php/Main_Page).
  - Faster curves somehow?

### Configuration options:

```bash
TODO:

./kessler --fps --particle-growth-rate --max-particle-count --etc.
```

### How does it work?

While there seems to be a lot of highly non-linear bad-ness going on, it is actually rather simple. The program generates a bunch of [Bézier curves](https://en.wikipedia.org/wiki/B%C3%A9zier_curve) as a function of time, and then applies some noise. The fade-to-white effect simply comes from a saturating add to the color buffer instead of an overwrite.