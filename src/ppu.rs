//use crate::logger::Logger;
use crate::mbc::MBC;
use crate::ram::Reg;

extern crate bit_field;
use bit_field::BitField;

enum Mode {
    OAMScan,
    Drawing,
    HBlank,
    VBlank,
}

pub struct PPU {
    pub mbc: MBC,

    pub buffer: [[u8; 160]; 144],
    pub buffer_bg: [[u8; 256]; 256],
    pub buffer_win: [[u8; 256]; 256],
    pub buffer_obj: [[u8; 256]; 256],
    pub buffer_vram: [[u8; 256]; 256],

    pub lx: usize,
}
 
impl PPU {
    pub fn new(mbc: MBC) -> PPU {
        PPU {
            mbc: mbc,
            buffer: [[0; 160]; 144],
            buffer_bg: [[0; 256]; 256],
            buffer_win: [[0; 256]; 256],
            buffer_obj: [[0; 256]; 256],
            buffer_vram: [[0; 256]; 256],
            lx: 0,
        }
    }

    #[inline]
    fn read(&self, i: u16) -> u8 {
        self.mbc.read(i)
    }

    #[inline]
    fn read_reg(&self, i: Reg) -> u8 {
        self.mbc.read_reg(i)
    }

    #[inline]
    fn write_reg(&mut self, i: Reg, v: u8) {
        self.mbc.write_reg(i, v);
    }

    #[inline]
    fn modify_reg(&mut self, r: Reg, f: fn(u8) -> u8) {
        self.mbc.modify_reg(r, f);
    }

    #[inline]
    fn set_interrupt_stat(&mut self) {
        self.modify_reg(Reg::IF, |mut u| *u.set_bit(1, true));
    }

    #[inline]
    fn set_interrupt_vblank(&mut self) {
        self.modify_reg(Reg::IF, |mut u| *u.set_bit(0, true));
    }

    fn read_tile(&mut self, addr: u16) -> [[u8; 8]; 8] {
        let mut tile = [[0; 8]; 8];
        for y in 0..8 {
            let t1 = self.read(addr + (y as u16) * 2);
            let t2 = self.read(addr + (y as u16) * 2 + 1);
            for x in 0..8 {
                tile[y][7-x] = (t1 >> x & 1) | ((t2 >> x & 1) << 1);
            }
        }

        tile
    }

    fn adderssing_tile(&self, i: u8, is_obj: bool) -> u16 {
        let lcdc = self.read_reg(Reg::LCDC);
        let adderssing_mode = lcdc.get_bit(4);

        if is_obj || adderssing_mode {
            0x8000 + (i as u16) * 16
        } else {
            ((0x9000 as i32) + (i as i8 as i32 * 16)) as u16
        }
    }

    fn draw_background(&mut self) {
        let lcdc = self.read_reg(Reg::LCDC);
        let bg_addr = if lcdc.get_bit(3) { 0x9c00 } else { 0x9800 };
        let mut y = 0;
        let mut x = 0;
        for i in 0 .. 1024 {
            let ti = self.read(bg_addr + i);
            let addr = self.adderssing_tile(ti, false);
            let tile = self.read_tile(addr);
            for iy in 0..8 {
                for ix in 0..8 {
                    let color_id = tile[iy][ix];
                    let color = (self.read_reg(Reg::BGP) >> (color_id * 2)) & 0b11;
                    let yy = (y + iy) % 256;
                    let xx = (x + ix) % 256;
                    self.buffer_bg[yy][xx] = color;
                }
            }

            x += 8;
            if x >= 256 {
                x = 0;
                y += 8;
                if y >= 256 {
                    y = 0;
                }
            }
        }

        let mut scy = self.read_reg(Reg::SCY) as usize;
        for dy in 0..144 {
            let mut scx = self.read_reg(Reg::SCX) as usize;
            for dx in 0..160 {
                self.buffer[dy][dx] = self.buffer_bg[scy % 256][scx % 256];
                scx += 1;
            }
            scy += 1;
        }
    }

    fn draw_window(&mut self) {
        let lcdc = self.read_reg(Reg::LCDC);
        let wy = self.read_reg(Reg::WY);
        let wx = self.read_reg(Reg::WX) - 6;

        let win_enable = lcdc.get_bit(5);
        if !win_enable { 
            return; 
        }

        let win_addr = if lcdc.get_bit(6) { 0x9c00 } else { 0x9800 };
        let mut y = wy as usize;
        let mut x = wx as usize;
        for i in 0 .. 1024 {
            let ti = self.read(win_addr + i);
            let addr = self.adderssing_tile(ti, false);
            let tile = self.read_tile(addr);
            for iy in 0..8 {
                for ix in 0..8 {
                    let color_id = tile[iy][ix];
                    let color = (self.read_reg(Reg::BGP) >> (color_id * 2)) & 0b11;
                    let yy = (y + iy) % 256;
                    let xx = (x + ix) % 256;
                    self.buffer_win[yy][xx] = color;
                }
            }
            x += 8;
            if x >= 256 {
                x = 0;
                y += 8;
                if y >= 256 {
                    y = 0;
                }
            }
        }

        for dy in 0..144 {
            for dx in 0..160 {
                let color = self.buffer_win[dy][dx];
                if wy <= (dy as u8) && wx <= (dx as u8) {
                    self.buffer[dy][dx] = color;
                }
            }
        }
    }

    fn draw_oam(&mut self) {
        let lcdc = self.read_reg(Reg::LCDC);

        let obj_enable = lcdc.get_bit(1);
        if !obj_enable {
            return;
        }

        let obj_size = 1 + lcdc.get_bit(2) as usize;
        let obj_len = 8 * obj_size as usize;

        //let mut oy = 0;
        //let mut ox = 0;
        for i in 0..40 {
            let o = 0xfe00 + i * 4;
            let y = self.read(o) as usize;
            let x = self.read(o + 1) as usize;
            let t = self.read(o + 2);
            let a = self.read(o + 3);

            let flip_y = a.get_bit(6);
            let flip_x = a.get_bit(5);
            let dmg_palette = self.read_reg(if a.get_bit(4) { Reg::OBP1 } else { Reg::OBP0 });
            //let cgb_palette_bank = a.get_bit(3);
            //let cgb_palette = a.get_bits(0..=2);

            let visible = y == 0 || y >= 160 || /*x <= 8 ||*/ x >= 168 || a.get_bit(7);

            for z in 0..obj_size {
                let zz = if flip_y && z == 0 { 1 } else if flip_y && z == 1 { 0 } else { z };
                let ti = self.adderssing_tile(t + (zz as u8), true);
                let tile = self.read_tile(ti);
                for yy in 0..8 {
                    for xx in 0..8 {
                        let iy = if flip_y { 7 - yy } else { yy };
                        let ix = if flip_x { 7 - xx } else { xx };
                        let color_id = tile[iy][ix];
                        let color = (dmg_palette >> (color_id * 2)) & 0b11;
                        //let ci = (color_id * 2) as usize;
                        //let color = dmg_palette.get_bits(ci..ci+1);
                        //self.buffer_obj[oy+yy+z*8][ox+xx] = color_id;
                        let yyy = y - obj_len + yy + z * 8;
                        let xxx = x - 8 + xx;
                        if yyy < 144 && xxx < 160 && !visible && color_id != 0 {
                            self.buffer[yyy][xxx] = color;
                        }
                    }
                }
            }
            //ox += 8;
            //if ox == 256 {
            //    ox = 0;
            //    oy += 8 * (1 + obj_size as usize);
            //    if oy == 256 {
            //        oy = 0;
            //    }
            //}
        }
    }

    #[allow(dead_code)]
    fn clear_buffer(&mut self) {
        for y in 0 .. 144 {
            for x in 0 .. 160 {
                self.buffer[y][x] = 0;
            }
        }
        for y in 0 .. 256 {
            for x in 0 .. 256 {
                self.buffer_obj[y][x] = 0;
            }
        }
        for y in 0 .. 256 {
            for x in 0 .. 256 {
                self.buffer_win[y][x] = 0;
            }
        }
        for y in 0 .. 256 {
            for x in 0 .. 256 {
                self.buffer_bg[y][x] = 0;
            }
        }
        for y in 0 .. 256 {
            for x in 0 .. 256 {
                self.buffer_vram[y][x] = 0;
            }
        }
    }

    #[allow(dead_code)]
    fn dump_vram(&mut self) {
        let mut y: usize = 0;
        let mut x: usize = 0;
        let mut z: usize = 0;
        for i in 0..512 { // VRAM Address: 0x8000 .. 0x9fff
            let addr = 0x8000 + i * 16;
            for j in 0 .. 8 {
                let t1 = self.read(addr + (j as u16) * 2);
                let t2 = self.read(addr + (j as u16) * 2 + 1);
                for k in 0 .. 8 {
                    let color_id = (t2 >> (7 - k) & 1) << 1 | (t1 >> (7 - k) & 1);
                    self.buffer_vram[y+j][x+k] = color_id;
                }
            }

            z += 1;

            if z == 1 {
                y += 8;
            }

            if z == 2 {
                z = 0;
                x += 8;
                y -= 8;
                if x >= self.buffer_vram[y].len() {
                    x = 0;
                    y += 16;
                    if y >= self.buffer_vram.len() {
                        y = 0;
                    }
                }
            }
        }
    }

    fn draw(&mut self) {
        //self.clear_buffer();
        self.draw_background();
        self.draw_window();
        self.draw_oam();
        //self.dump_vram();
    }

    fn compare_lyc(&mut self) {
        let mut stat = self.read_reg(Reg::STAT);
        let lyc = self.read_reg(Reg::LYC);
        let ly = self.read_reg(Reg::LY);

        if lyc == ly {
            self.write_reg(Reg::STAT, *stat.set_bit(2, true));
            if stat.get_bit(6) {
                self.set_interrupt_stat();
            }
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        let mut stat = self.read_reg(Reg::STAT);
        match mode {
            Mode::HBlank => {
                self.write_reg(Reg::STAT, *stat.set_bits(0..=1, 0));
                self.mbc.set_vram_blocking(false);
                self.mbc.set_oam_blocking(false);
                if stat.get_bit(3) {
                    self.set_interrupt_stat();
                }
            },
            Mode::VBlank => {
                self.write_reg(Reg::STAT, *stat.set_bits(0..=1, 1));
                self.mbc.set_vram_blocking(false);
                self.mbc.set_oam_blocking(false);
                if stat.get_bit(4) {
                    self.set_interrupt_stat();
                }
                self.set_interrupt_vblank();
            },
            Mode::OAMScan => {
                self.write_reg(Reg::STAT, *stat.set_bits(0..=1, 2));
                self.mbc.set_vram_blocking(false);
                self.mbc.set_oam_blocking(true);
                if stat.get_bit(5) {
                    self.set_interrupt_stat();
                }
            },
            Mode::Drawing => {
                self.write_reg(Reg::STAT, *stat.set_bits(0..=1, 3));
                //self.mbc.set_vram_blocking(true);
                self.mbc.set_oam_blocking(true);
            },
       }
    }

    pub fn step(&mut self) {
        let mut ly = self.read_reg(Reg::LY);

        if self.lx == 457 {
            self.lx = 0;
            ly += 1;

            if ly == 154 {
                ly = 0;
            }

            self.write_reg(Reg::LY, ly);
            self.compare_lyc();
        }

        if ly <= 143 {
            if self.lx == 0 {
                self.set_mode(Mode::OAMScan);
            } else if self.lx == 80 {
                self.set_mode(Mode::Drawing);
            } else if self.lx == 252 {
                self.set_mode(Mode::HBlank);
            }
        } else if ly == 144 {
            if self.lx == 0 {
                self.draw();
            }
            self.set_mode(Mode::VBlank);
        }

        self.lx += 1;
    }
}
