
use std::cell::RefCell;

pub trait MBC {
    fn new(rom: Vec<u8>) -> Self;
    fn read(&self, i: usize) -> u8;
    fn write(&mut self, i: usize, v: u8);
}

pub struct MBC1 {
    rom: Vec<u8>,
    ram: RefCell<Vec<u8>>,
    ramx: RefCell<Vec<u8>>,
    bank: usize,
    bank1: usize,
    bank2: usize,
    bank_mode: bool,
    ramx_enable: bool,

}

impl MBC for MBC1 {
    fn new(rom: Vec<u8>) -> Self {
        MBC1 {
            rom: rom,
            ram: RefCell::new(vec![0; 0xffff]),
            ramx: RefCell::new(vec![0; 131072]),//128 * 1024],
            bank: 0,
            bank1: 0,
            bank2: 0,
            ramx_enable: false,
            bank_mode: false,
        }
    }

    fn read(&self, i: usize) -> u8 {
        if i <= 0x3fff {
            //let j = self.bank2 & 0b11 == 0 ? i : self.bank2 << 19 | i;
            self.rom[i]
        }
        else if i >= 0x4000 && i <= 0x7fff {
            if self.bank_mode {
                self.rom[self.bank | (i - 0x4000)]
            }
            else {
                self.rom[i - 0x4000]
            }
        }
        else if i >= 0xa000 && i <= 0xbfff {
            if self.bank_mode {
                if self.ramx_enable {
                    self.ramx.borrow()[i - 0xa000]
                }
                else {
                    self.ram.borrow()[i]
                }
            }
            else {
                self.ram.borrow()[i]
            }
        }
        else {
            self.ram.borrow()[i]
        }
    }

    fn write(&mut self, i: usize, v: u8) {
        if i <= 0x1fff {
            //self.ram[i] = if v == 0xa { 0xa } else { 0 };
            self.ramx_enable = v == 0xa;
        }
        else if i >= 0x2000 && i <= 0x3fff {
            self.ram.borrow_mut()[i] = if v == 0 { 1 } else { v & 0b11111 };
            self.bank1 = self.ram.borrow()[i] as usize;
            self.bank = (self.bank2 << 19) | (self.bank1 << 14);
        }
        else if i >= 0x4000 && i <= 0x5fff {
            self.ram.borrow_mut()[i] = v & 0b11;
            self.bank2 = self.ram.borrow()[i] as usize;
            self.bank = (self.bank2 << 19) | (self.bank1 << 14);
        } 
        else if i >= 0xa000 && i <= 0xbfff {
            let enable = self.ram.borrow()[0] & 0b11;
            if enable == 0 {
                self.ram.borrow_mut()[i] = v;
            }
            else {
                self.ramx.borrow_mut()[i - 0xa000] = v;
            }
        }
        else if i >= 0x6000 && i <= 0x7fff {
            self.bank_mode = v == 1;
        }
        else if i >= 0xc000 && i <= 0xddff {
            self.ram.borrow_mut()[i] = v;
            self.ram.borrow_mut()[i + 0x2000] = v;
        }
        else {
            self.ram.borrow_mut()[i] = v;
        }
    }
}
