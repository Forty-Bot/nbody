#![feature(associated_consts)]
#![allow(non_camel_case_types)]

use math::{vec2, Additive};
mod math;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io;
use std::ops::Deref;
use std::rc::Rc;

extern crate rayon;

use rayon::prelude::*;

extern crate sfml;

use sfml::system::{Clock, Time, Vector2f, Vector2i};
use sfml::window::{ContextSettings, Event, Key, style, VideoMode,};
use sfml::graphics::{Color, Drawable, Font, Image, RcSprite, RenderWindow, RenderTarget, Sprite, Text,
	Texture, TextureRef, Transformable, View};

#[derive(Clone, Copy, Debug)]
struct Object {
	s: vec2<f32>,
	v: vec2<f32>,
	m: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Deriv {
	ds: vec2<f32>,
	dv: vec2<f32>,
}

fn partial(o: &Object, d: &Deriv, dt: f32) -> Object {
	Object {
			m: o.m,
			s: o.s + d.ds * dt,
			v: o.v + d.dv * dt,
	}
}

const G: f32 = 6.67408e-11;
/* The gravitational acceleration that b exerts on a */
fn grav(a: &Object, b: &Object) -> vec2<f32> {
	let ba = b.s - a.s;
	let rsq = ba.normsq();
	let mag = G * b.m / rsq;
	ba * mag * (1.0 / rsq.sqrt())
}

fn diff(init: &[Object], t: f32, dt: f32, derivs: &[Deriv]) -> Vec<Deriv> {
	/* First calculate a new state based on the derivatives */
	let new = init.par_iter()
		.zip(derivs.par_iter())
		.map(|(o, d)| partial(o, d, dt))
		.collect::<Vec<Object>>();
	/* Now calculate the new acceleration */
	/* TODO: cache results */
	new.par_iter()
		.enumerate()
		.map(|(i, a)| -> vec2<f32> {
			new.par_iter()
				.take(i)
				.map(|b: &Object| grav(a, b))
				.reduce(|| vec2::ZERO, |a, v| a + v)
			+ new.par_iter()
				.skip(i + 1)
				.map(|b: &Object| grav(a, b))
				.reduce(|| vec2::ZERO, |a, v| a + v)
		})
	/* And zip it with the velocity for the new derivatives */
		.zip(new.par_iter())
		.map(|(a, o)| Deriv {
			ds: o.v,
			dv: a,
		})
		.collect()
}

fn weight(a: vec2<f32>, b: vec2<f32>, c: vec2<f32>, d: vec2<f32>) -> vec2<f32> {
	(a + (b + c)*2.0 + d) * (1.0/6.0)
}

fn integrate(state: &[Object], t: f32, dt: f32) -> Vec<Object> {
	let a = diff(state, t, 0.0, vec![Deriv::default(); state.len()].as_slice());
	let b = diff(state, t, 0.5 * dt, a.as_slice());
	let c = diff(state, t, 0.5 * dt, b.as_slice());
	let d = diff(state, t, dt, c.as_slice());

	a.par_iter().zip(b.par_iter().zip(c.par_iter().zip(d.par_iter())))
		.map(|(a, (b, (c, d)))| Deriv {
			ds: weight(a.ds, b.ds, c.ds, d.ds),
			dv: weight(a.dv, b.dv, c.dv, d.dv),
		})
		.zip(state.par_iter())
		.map(|(d, o)| partial(o, &d, dt))
		.collect()
}

fn preload_tex(cache: &mut HashMap<String, Rc<Texture>>, path: &str) {
	cache.entry(path.into()).or_insert({
		let img = Image::from_file(&path).expect(&format!("cannot load texture from {}", path));
		img.create_mask_from_color(&Color::black(), 0);
		Rc::new(Texture::from_image(&img).expect("could not convert image to texture"))
	});
}

fn main() {
	
	let mut window = RenderWindow::new(VideoMode::desktop_mode(), "nbody", style::DEFAULT,
		&ContextSettings::default());
	window.set_framerate_limit(60);

	let mut line = String::new();
	io::stdin().read_line(&mut line);
	line.pop();
	let num_objs: usize = line.trim().parse().expect(&format!("invalid number of objects: `{}'", line));
	line.clear();
	io::stdin().read_line(&mut line);
	line.trim();
	let r: f32 = line.trim().parse().expect(&format!("invalid universe size: {}", line));
	let mut view = View::new(Vector2f::new(0.0, 0.0), Vector2f::new(2.0 * r, 2.0 * r));
	window.set_view(&view);
	line.clear();

	let mut state = Vec::new();
	let mut tex_cache: RefCell<HashMap<String, _>> = RefCell::new(HashMap::new());
	let mut gfx = Vec::new();
	let def = window.default_view().size();
	let mut tmp = Vec::new();
	for i in 0..num_objs {
		line.clear();
		io::stdin().read_line(&mut line);
		let mut iter = line.trim().split_whitespace();
		state.push(Object {
			s: vec2 {
				x: {
					let tmp_w = iter.next();
					match tmp_w {
						Some(tmp) => tmp.parse().expect(tmp),
						None => continue
					}
				},
				y: { let tmp = iter.next().unwrap(); tmp.parse().expect(tmp) },
			},
			v: vec2 {
				x: { let tmp = iter.next().unwrap(); tmp.parse().expect(tmp) },
				y: { let tmp = iter.next().unwrap(); tmp.parse().expect(tmp) },
			},
			m: iter.next().unwrap().parse().unwrap(),
		});

		let path = format!("img/{}", iter.next().unwrap().parse::<String>().unwrap());

		preload_tex(&mut tex_cache.borrow_mut(), &path);
		tmp.push(path);
	}
		
	for path in tmp {
		let tex = tex_cache.borrow().get(&path).unwrap().clone();
		let sz = tex.size();
		let mut s = RcSprite::with_texture(tex);
		s.set_origin((sz.x as f32 / 2.0, sz.y as f32 / 2.0));
		s.scale((2.0 * r / def.x, 2.0 * r / def.y));
		gfx.push(s);
	}

	let hack = Font::from_file("/usr/share/fonts/TTF/Hack-Regular.ttf").expect("cannot load Hack font");
	let mut fps_counter = Text::default();
	fps_counter.set_font(&hack);
	fps_counter.set_position(window.map_pixel_to_coords_current_view(&Vector2i::new(0, 0)));
	fps_counter.scale((2.0 * r / def.x, 2.0 * r / def.y));

	let mut left = false;
	let mut right = false;
	let mut up = false;
	let mut down = false;
	
	let mut t = 0.0;
	let mut acc = 0.0;
	let mut mult = 1.0e6;
	let mut dt = 1.0 / 1024.0;
	let mut clk = Clock::start();

	loop {

		for evt in window.events() {
			match evt {
				Event::Closed => return,
				Event::KeyPressed {code, alt, ctrl, shift, system} => {
					println!("{:?} pressed", code);
					match code {
						Key::Comma => mult *= 0.5,
						Key::Period => mult *= 2.0,
						Key::LShift => view.zoom(0.5),
						Key::LControl => view.zoom(2.0),
						Key::W => up = true,
						Key::A => left = true,
						Key::S => down = true,
						Key::D => right = true,
						_ => {},
					}
				},
				Event::KeyReleased {code, alt, ctrl, shift, system} => {
					println!("{:?} released", code);
					match code {
						Key::W => up = false,
						Key::A => left = false,
						Key::S => right = false,
						Key::D => down = false,
						_ => {}
					}
				},
				_ => {},
			}
		}
		let size = view.size();
		if left { view.move_((size.x * -0.001, 0.0)) }
		if right { view.move_((size.x * 0.001, 0.0)) }
		if up { view.move_((0.0, size.y * -0.001)) }
		if down { view.move_((0.0, size.y * 0.001)) }
		window.set_view(&view);
		
		let frame_time = clk.restart().as_seconds();
		acc += frame_time;
		
		let mut i = 0;
		while acc >= dt && i < 5 {
			state = integrate(state.as_slice(), t, dt * mult);
			acc -= dt;
			t += dt * mult;
			i += 1;
		}
		
		window.clear(&Color::black());
		
		for (o, mut s) in state.iter().zip(gfx.iter_mut()) {
			s.set_position((o.s.x, o.s.y));
			let sprite: &Sprite = &*s;
			window.draw(sprite)
		}

		fps_counter.set_string(&format!("{:.0}\n{}", 1.0 / frame_time, mult));
		window.draw(&fps_counter);

		window.display();
	}
}
