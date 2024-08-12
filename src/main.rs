use gbe_rs::rom::read_rom;
use gbe_rs::mbc::select_mbc;
use gbe_rs::ppu::PPU;
use gbe_rs::cpu::CPU;

use minifb::{Key, Window, WindowOptions, Scale};

use std::env;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;
//const WIDTH: usize = 256;
//const HEIGHT: usize = 256;

fn display(mut cpu: CPU) {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut options = WindowOptions::default();
    //options.resize = true;
    options.scale = Scale::X4;
    let mut window = Window::new(
        "GBE.rs",
        WIDTH,
        HEIGHT,
        options,
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    //window.set_target_fps(60);

    let mut n = 0;
    'game: loop {
        let mut joypad: u8 = 0b11111111;
        for key in window.get_keys() {
            match key {
                Key::D => { joypad &= 0b11111110; },
                Key::A => { joypad &= 0b11111101; },
                Key::W => { joypad &= 0b11111011; },
                Key::S => { joypad &= 0b11110111; },
                Key::K => { joypad &= 0b11101111; },
                Key::J => { joypad &= 0b11011111; },
                Key::I => { joypad &= 0b10111111; },
                Key::U => { joypad &= 0b01111111; },
                Key::Escape => break 'game,
                Key::Space => cpu.halting = !cpu.halting,
                _ => {},
            }
        }

        cpu.joypad_buffer = joypad;
        cpu.step();
        n += 1;

        if n >= 70224 {
            n = 0;
            let mut i: usize = 0;
            for y in 0 .. HEIGHT {
                for x in 0 .. WIDTH {
                    buffer[i] = match cpu.ppu.buffer[y][x] {
                        3 => 0x44444444,
                        2 => 0x88888888,
                        1 => 0xaaaaaaaa,
                        0 => 0xeeeeeeee,
                        _ => 0,
                    };
                    i += 1;
                }
            }
            window
                .update_with_buffer(&buffer, WIDTH, HEIGHT)
                .unwrap();
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let rom = read_rom(args[1].clone()).unwrap();
    println!("{}", rom.title);

    let mut cpu = CPU::new(PPU::new(select_mbc(rom)));
    cpu.cpu_logger.logging = false;
    display(cpu);
    //loop {
    //    if cpu.exe_counter < 26000000 {
    //        cpu.step();
    //    } else {
    //        let ss = cpu.serial_logger.reads(108);
    //        println!("{}", String::from_utf8(ss.to_vec()).unwrap());
    //        break;
    //    }
    //}
}
