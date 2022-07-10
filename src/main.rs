
//use std::rc::Rc;
//use std::cell::RefCell;
//
//macro_rules! log {
//    ($t:expr, $($arg:expr),*) => {
//        println!($t, $($arg,)*);
//    }
//}
//
//#[derive(Debug)]
//struct Mem {
//    ram: Vec<u8>,
//}
//
//#[derive(Debug)]
//struct CPU {
//    mem: Rc<RefCell<Mem>>
//}

//fn bitset(b: u8, i: u8, v: u8) -> u8 { (b & !(1 << i)) | (v << i) }

fn main() {
    //let b = 0b1111;
    //println!("{}", bitset(b, 3, 0));
    //println!("{}", bitset(b, 2, 0));
    //println!("{}", bitset(b, 1, 0));
    //println!("{}", bitset(b, 0, 0));
    //let mem = Rc::new(RefCell::new(Mem { ram: vec![1] }));
    //let cpu1 = CPU { mem: mem.clone() };
    //let cpu2 = CPU { mem: mem.clone() };
    //cpu1.mem.borrow_mut().ram.push(2);
    //cpu2.mem.borrow_mut().ram.push(3);
    //println!("{:?}",cpu1);
    //println!("{:?}",cpu2);
    //log!("log {} {:?}", 1, Mem{ram:vec![]});
}

//extern crate sdl2;
//extern crate rand;
//
//use sdl2::event::Event;
//use sdl2::keyboard::Keycode;
//use sdl2::pixels::Color;
//use sdl2::rect::{Point, Rect};
//use rand::random;
//use std::time::Duration;
//
//fn main() {
//    main_loop();
//}
//
//pub fn main_loop() -> Result<(), String> {
//    let sdl_context = sdl2::init()?;
//    let video_subsystem = sdl_context.video()?;
//
//    let (ry,rx) = (5,5);
//    let window = video_subsystem
//        .window("gbrs", 144*rx, 160*ry)
//        .position_centered()
//        .build()
//        .map_err(|e| e.to_string())?;
//
//    let (y,x) = window.size();
//    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
//
//    canvas.set_draw_color(Color::RGB(0, 0, 0));
//    canvas.clear();
//    canvas.present();
//
//    let mut event_pump = sdl_context.event_pump()?;
//
//    'running: loop {
//        for event in event_pump.poll_iter() {
//            match event {
//                Event::Quit { .. }
//                | Event::KeyDown {
//                    keycode: Some(Keycode::Escape),
//                    ..
//                } => break 'running,
//                _ => {}
//            }
//        }
//
//
//        for y in 0 .. y/ry {
//            for x in 0 .. x/rx {
//                canvas.set_draw_color(Color::RGB(random::<u8>(),random::<u8>(),random::<u8>()));
//                //canvas.draw_point(Point::new(y as i32,x as i32));
//                canvas.fill_rect(Rect::new((y * ry) as i32, (x * rx) as i32, ry, rx));
//            }
//        }
//
//        canvas.present();
//        ::std::thread::sleep(Duration::new(0, 1));
//    }
//
//    Ok(())
//}
