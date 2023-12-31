use crate::ram::{Reg, RAM};
use crate::rom::MBCType;
use crate::rom::ROM;

pub type MBC = Box<dyn MBCTrait>;

pub trait MBCTrait {
    fn read(&self, i: u16) -> u8;
    fn write(&mut self, i: u16, v: u8);
    fn read_reg(&self, r: Reg) -> u8;
    fn write_reg(&mut self, r: Reg, v: u8);
    fn modify_reg(&mut self, r: Reg, f: fn(u8) -> u8);
    fn get_rom(&self) -> &ROM;
    fn get_ram(&self) -> &RAM;
    fn get_rom_bank(&self) -> usize;
    fn get_ram_ex_bank(&self) -> usize;
    fn set_vram_blocking(&mut self, b: bool);
    fn set_oam_blocking(&mut self, b: bool);
}

pub fn select_mbc(rom: ROM) -> MBC {
    match rom.rom_type.mbc_type {
        MBCType::MBC1 => Box::new(MBC1::new(rom)),
        _ => unimplemented!(),
    }
}

#[derive(Debug)]
pub struct MBC1 {
    pub rom: ROM,
    pub ram: RAM,

    pub rom_bank: usize,
    pub rom_bank1: usize,
    pub rom_bank2: usize,

    pub ram_ex_bank: usize,
    pub ram_ex_enable: bool,

    pub banking_mode: bool,
    pub vram_blocking: bool,
    pub oam_blocking: bool,
}

impl MBC1 {
    pub fn new(rom: ROM) -> MBC1 {
        let ram = RAM::new(rom.ram_ex_size);
        MBC1 {
            rom: rom,
            ram: ram,
            rom_bank: 0,
            rom_bank1: 0,
            rom_bank2: 0,
            ram_ex_bank: 0,
            ram_ex_enable: false,
            banking_mode: false,
            vram_blocking: false,
            oam_blocking: false,
        }
    }
}

impl MBCTrait for MBC1 {
    #[inline]
    fn read_reg(&self, r: Reg) -> u8 {
        self.ram.read_reg(r)
    }

    #[inline]
    fn write_reg(&mut self, r: Reg, v: u8) {
        self.ram.write_reg(r, v)
    }

    #[inline]
    fn modify_reg(&mut self, r: Reg, f: fn(u8) -> u8) {
        self.ram.modify_reg(r, f)
    }

    #[inline]
    fn get_rom(&self) -> &ROM {
        &self.rom
    }

    #[inline]
    fn get_ram(&self) -> &RAM {
        &self.ram
    }

    #[inline]
    fn get_rom_bank(&self) -> usize {
        self.rom_bank
    }

    #[inline]
    fn get_ram_ex_bank(&self) -> usize {
        self.ram_ex_bank
    }

    #[inline]
    fn set_oam_blocking(&mut self, b: bool) {
        self.oam_blocking = b;
    }

    #[inline]
    fn set_vram_blocking(&mut self, b: bool) {
        self.vram_blocking = b;
    }

    fn read(&self, i: u16) -> u8 {
        let i = i as usize;
        match i {
            0..=0x3fff => self.rom.read(i),
            0x4000..=0x7fff => { self.rom.read(self.rom_bank | (i - 0x4000)) },
            0x8000..=0x9fff => self.ram.read(i),
            0xa000..=0xbfff => {
                if self.ram_ex_enable {
                    self.ram.read_ex(self.ram_ex_bank | (i - 0xa000))
                } else {
                    0
                }
            }
            _ => self.ram.read(i),
        }
    }

    fn write(&mut self, i: u16, v: u8) {
        let i = i as usize;
        match i {
            0x0000..=0x1fff => {
                self.ram_ex_enable = v & 0xf == 0xa;
            }
            0x2000..=0x3fff => {
                self.rom_bank1 = if v == 0 { 1 } else { (v as usize) & 0x1f };
                self.rom_bank = self.rom_bank2 << 19 | self.rom_bank1 << 14;
            }
            0x4000..=0x5fff => {
                self.rom_bank2 = (v as usize) & 0x3;
                self.rom_bank = self.rom_bank2 << 19 | self.rom_bank1 << 14;
            }
            0x6000..=0x7fff => {
                if v != 0 {
                    self.ram_ex_enable = true;
                    self.ram_ex_bank = self.rom_bank2 << 13;
                } else {
                    self.ram_ex_enable = false;
                    self.ram_ex_bank = 0;
                }
            }
            0x8000..=0x9fff => {
                if !self.vram_blocking {
                    self.ram.write(i, v);
                }
            }
            0xa000..=0xbfff => {
                if self.ram_ex_enable {
                    self.ram.write_ex(self.ram_ex_bank | ((i as usize) - 0xa000), v);
                }
            }
            0xff46 => {
                if !self.oam_blocking {
                    self.ram.transfer_dma(v as usize);
                }
                self.ram.write(i, v);
            }
            _ => self.ram.write(i, v),
        }
    }
}
