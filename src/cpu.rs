use crate::logger::Logger;
use crate::ppu::PPU;
use crate::ram::Reg;

use std::fmt;
use std::fmt::Write;

extern crate bit_field;
use bit_field::BitField;


#[derive(Debug, Clone, Default)]
pub struct CPULog {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,

    pub sp: u16,
    pub pc: u16,

    pub ime: bool,
    pub halting: bool,

    pub cycle: usize,
    pub sys_counter: usize,
    pub exe_counter: usize,

    pub reg_if: u8,
    pub reg_ie: u8,
    pub rom_bank: usize,
    pub ram_ex_bank: usize,

    pub codes: Vec<u8>,

    pub text: String,
}

impl CPULog {
    pub fn get_carry(&self) -> bool {
        self.f.get_bit(4)
    }

    pub fn get_half(&self) -> bool {
        self.f.get_bit(5)
    }

    pub fn get_negative(&self) -> bool {
        self.f.get_bit(6)
    }

    pub fn get_zero(&self) -> bool {
        self.f.get_bit(7)
    }
}

impl fmt::Display for CPULog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cs = String::new();
        write!(cs, "{:02x} {:02x} {:02x}", self.codes[0], self.codes[1], self.codes[2])?;
        write!(f, "--- {}\nBANK:{:x}\nPC:{:04x} [{}] SP:{:04x}\nF:{}{}{}{} A:{:02x} BC:{:04x} DE:{:04x} HL:{:04x} IF:{:04b} IE:{:04b} IME:{} HALT:{}\n> {}\n", 
            self.exe_counter, self.rom_bank, self.pc, cs, self.sp, 
            if self.get_zero() { "Z" } else { "0" },
            if self.get_negative() { "S" } else { "0" },
            if self.get_half() { "H" } else { "0" },
            if self.get_carry() { "C" } else { "0" },
            self.a, 
            u16::from_be_bytes([self.b, self.c]),
            u16::from_be_bytes([self.d, self.e]),
            u16::from_be_bytes([self.h, self.l]),
            self.reg_if,
            self.reg_ie,
            u8::from(self.ime),
            u8::from(self.halting),
            self.text,
        )
    }
}

#[derive(PartialEq)]
enum LogInfo {
    U8h(u8),
    U16h(u16),
    I8h(i8),
    None,
}

impl fmt::Display for LogInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogInfo::U8h(n) => write!(f, "{:02x}", n),
            LogInfo::U16h(n) => write!(f, "{:04x}", n),
            LogInfo::I8h(n) => write!(f, "{:+02}", n),
            LogInfo::None => Ok(()),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Clone, Copy)]
enum OP {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    A_,
    AF,
    BC,
    DE,
    HL,
    SP,
    N,
    NN,
    P_BC,
    P_DE,
    P_HL,
    P_NN,
    P_FF00_N,
    P_FF00_C,
    P_HL_INC,
    P_HL_DEC,
    Zero,
    Carry,
    NotZero,
    NotCarry,
    Always,
    None,
}

impl fmt::Display for OP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OP::A => "A",
            OP::B => "B",
            OP::C => "C",
            OP::D => "D",
            OP::E => "E",
            OP::H => "H",
            OP::L => "L",
            OP::N => "N",
            OP::A_ => "A_",
            OP::NN => "NN",
            OP::AF => "AF",
            OP::BC => "BC",
            OP::DE => "DE",
            OP::HL => "HL",
            OP::SP => "SP",
            OP::P_BC => "(BC)",
            OP::P_DE => "(DE)",
            OP::P_HL => "(HL)",
            OP::P_NN => "(NN)",
            OP::P_HL_INC => "(HL++)",
            OP::P_HL_DEC => "(HL--)",
            OP::P_FF00_N => "(FF00+N)",
            OP::P_FF00_C => "(FF00+C)",
            OP::Zero => "Z",
            OP::NotZero => "NZ",
            OP::Carry => "C",
            OP::NotCarry => "NC",
            OP::Always => "_",
            OP::None => "",
        };
        write!(f, "{}", s)
    }
}

trait AddCarryHalf<A=Self> where Self: Sized {
    fn add_carry_half(self, a: A) -> (Self, bool, bool);
}

impl AddCarryHalf<u8> for u8 {
    fn add_carry_half(self, a: u8) -> (u8, bool ,bool) {
        let (b, carry) = self.overflowing_add(a);
        (b, carry, (self ^ a ^ b) & 0x10 != 0)
    }
}

impl AddCarryHalf<u16> for u16 {
    fn add_carry_half(self, a: u16) -> (u16, bool ,bool) {
        let (b, carry) = self.overflowing_add(a);
        (b, carry, (self ^ a ^ b) & 0x1000 != 0)
    }
}

trait SubCarryHalf<A=Self> where Self: Sized {
    fn sub_carry_half(self, a: A) -> (Self, bool, bool);
}

impl SubCarryHalf<u8> for u8 {
    fn sub_carry_half(self, a: u8) -> (u8, bool ,bool) {
        let (b, carry) = self.overflowing_sub(a);
        (b, carry, (self ^ a ^ b) & 0x10 != 0)
    }
}

impl SubCarryHalf<u16> for u16 {
    fn sub_carry_half(self, a: u16) -> (u16, bool ,bool) {
        let (b, carry) = self.overflowing_sub(a);
        (b, carry, (self ^ a ^ b) & 0x1000 != 0)
    }
}

//trait AddSingedU8CarryHalf<A=Self> where Self: Sized {
//    fn add_signed_u8_carry_half(self, a: u8) -> (Self, bool ,bool);
//}
//
//impl AddSignedU8CarryHalf<u16> for u16 {
//    fn add_signed_u8_carry_half(self, a: u8) -> (u16, bool ,bool) {
//        let i = a as i8 as i32;
//        let u = i as u16;
//        let res = (self as i32).overflowing_add(i).0 as u16;
//        (res, (self ^ u ^ res) & 0x100 != 0, (self ^ u ^ res) & 0x10 != 0)
//    }
//}

pub fn add_signed_u8_carry_half(n: u16, a: u8) -> (u16, bool ,bool) {
    let i = a as i8 as i32;
    let u = i as u16;
    let res = ((n as i32) + i) as u16;
    (res, (n ^ u ^ res) & 0x100 != 0, (n ^ u ^ res) & 0x10 != 0)
}


pub struct CPU {
    pub ppu: PPU,
    pub cpu_logger: Logger<CPULog>,
    pub serial_logger: Logger<u8>,

    pub joypad_buffer: u8,

    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,

    pub sp: u16,
    pub pc: u16,

    pub halting: bool,
    pub ime: bool,

    pub cycle: usize,
    pub sys_counter: usize,
    pub exe_counter: usize,
}

impl CPU {
    pub fn new(ppu: PPU) -> Self {
        CPU {
            ppu: ppu,
            cpu_logger: Logger::new(0x1000),
            serial_logger: Logger::new(0x1000),

            joypad_buffer: 0b111111,

            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0xfffe,
            pc: 0x100,
            halting: false,
            ime: false,
            cycle: 0,
            sys_counter: 0,
            exe_counter: 0,
        }
    }

    fn log(&mut self, instr: &str, op1: OP, op2: OP, info: LogInfo) {
        if self.cpu_logger.logging {
            let codes = (0..3).map(|n| self.read(self.pc - 1 + n)).collect();
            let c = CPULog {
                a: self.a,
                f: self.f,
                b: self.b,
                c: self.c,
                d: self.d,
                e: self.e,
                h: self.h,
                l: self.l,
                pc: self.pc - 1,
                sp: self.sp,
                halting: self.halting,
                ime: self.ime,
                cycle: self.cycle,
                sys_counter: self.sys_counter,
                exe_counter: self.exe_counter,
                reg_if: self.read_reg(Reg::IF),
                reg_ie: self.read_reg(Reg::IE),
                rom_bank: self.ppu.mbc.get_rom_bank(),
                ram_ex_bank: self.ppu.mbc.get_ram_ex_bank(),
                codes: codes,
                text: format!("{} {} {} {} {}", instr, op1, op2, if info == LogInfo::None { "" } else { "#" }, info),
            };
            self.cpu_logger.write(c);
        }
    }

    fn get_carry(&self) -> bool {
        self.f.get_bit(4)
    }

    fn get_half(&self) -> bool {
        self.f.get_bit(5)
    }

    fn get_negative(&self) -> bool {
        self.f.get_bit(6)
    }

    fn get_zero(&self) -> bool {
        self.f.get_bit(7)
    }

    fn set_carry(&mut self, b: bool) {
        self.f.set_bit(4, b);
    }

    fn set_half(&mut self, b: bool) {
        self.f.set_bit(5, b);
    }

    fn set_negative(&mut self, b: bool) {
        self.f.set_bit(6, b);
    }

    fn set_zero(&mut self, b: bool) {
        self.f.set_bit(7, b);
    }

    fn get_af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.f])
    }

    fn get_bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    fn get_de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    fn get_hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    fn set_af(&mut self, v: u16) {
        let bs = v.to_be_bytes();
        self.a = bs[0];
        self.f = bs[1] & 0b11110000;
    }

    fn set_bc(&mut self, v: u16) {
        let bs = v.to_be_bytes();
        self.b = bs[0];
        self.c = bs[1];
    }

    fn set_de(&mut self, v: u16) {
        let bs = v.to_be_bytes();
        self.d = bs[0];
        self.e = bs[1];
    }

    fn set_hl(&mut self, v: u16) {
        let bs = v.to_be_bytes();
        self.h = bs[0];
        self.l = bs[1];
    }

    #[inline]
    fn read(&mut self, i: u16) -> u8 {
        self.ppu.mbc.read(i)
    }

    #[inline]
    fn write(&mut self, i: u16, v: u8) {
        self.ppu.mbc.write(i, v);
    }

    #[inline]
    fn read_reg(&self, i: Reg) -> u8 {
        self.ppu.mbc.read_reg(i)
    }

    #[inline]
    fn write_reg(&mut self, i: Reg, v: u8) {
        self.ppu.mbc.write_reg(i, v);
    }

    #[inline]
    fn modify_reg(&mut self, r: Reg, f: fn(u8) -> u8) {
        self.ppu.mbc.modify_reg(r, f);
    }

    #[inline]
    fn tick(&mut self) {
        self.cycle += 1;
    }

    fn fetch8(&mut self) -> u8 {
        let v = self.read(self.pc);
        self.pc += 1;
        self.tick();
        v
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch8();
        let hi = self.fetch8();
        u16::from_be_bytes([hi, lo])
    }

    fn load8(&mut self, op: OP) -> u8 {
        match op {
            OP::A => self.a,
            OP::A_ => self.a,
            OP::B => self.b,
            OP::C => self.c,
            OP::D => self.d,
            OP::E => self.e,
            OP::H => self.h,
            OP::L => self.l,
            OP::N => self.fetch8(),
            OP::P_BC => self.read(self.get_bc()),
            OP::P_DE => self.read(self.get_de()),
            OP::P_HL => self.read(self.get_hl()),
            OP::P_NN => {
                let i = self.fetch16();
                self.read(i)
            }
            OP::P_HL_INC => {
                let hl = self.get_hl();
                self.set_hl(hl + 1);
                self.read(hl)
            }
            OP::P_HL_DEC => {
                let hl = self.get_hl();
                self.set_hl(hl - 1);
                self.read(hl)
            }
            OP::P_FF00_C => self.read(0xff00 + (self.c as u16)),
            OP::P_FF00_N => {
                let i = self.fetch8();
                self.read(0xff00 + (i as u16))
            }
            _ => panic!("CPU::load8 unexpected {:?}", op),
        }
    }

    fn load16(&mut self, op: OP) -> u16 {
        match op {
            OP::AF => self.get_af(),
            OP::BC => self.get_bc(),
            OP::DE => self.get_de(),
            OP::HL => self.get_hl(),
            OP::SP => self.sp,
            OP::NN => self.fetch16(),
            _ => panic!("CPU::load16 unexpected {:?}", op),
        }
    }

    fn store8(&mut self, op: OP, v: u8) {
        match op {
            OP::A => self.a = v,
            OP::A_ => self.a = v,
            OP::B => self.b = v,
            OP::C => self.c = v,
            OP::D => self.d = v,
            OP::E => self.e = v,
            OP::H => self.h = v,
            OP::L => self.l = v,
            OP::P_BC => self.write(self.get_bc(), v),
            OP::P_DE => self.write(self.get_de(), v),
            OP::P_HL => self.write(self.get_hl(), v),
            OP::P_NN => {
                let i = self.fetch16();
                self.write(i, v);
            },
            OP::P_HL_INC => {
                let hl = self.get_hl();
                self.set_hl(hl + 1);
                self.write(hl, v);
            },
            OP::P_HL_DEC => {
                let hl = self.get_hl();
                self.set_hl(hl - 1);
                self.write(hl, v);
            },
            OP::P_FF00_C => self.write(0xff00 + (self.c as u16), v),
            OP::P_FF00_N => {
                let i = self.fetch8();
                self.write(0xff00 + (i as u16), v);
            },
            _ => panic!("CPU::store8 unexpected {}", op),
        }
    }

    fn store16(&mut self, op: OP, v: u16) {
        match op {
            OP::AF => self.set_af(v),
            OP::BC => self.set_bc(v),
            OP::DE => self.set_de(v),
            OP::HL => self.set_hl(v),
            OP::P_NN => {
                let bs = v.to_be_bytes();
                let i = self.fetch16();
                self.write(i, bs[1]);
                self.write(i + 1, bs[0]);
                self.cycle -= 1;
            }
            OP::SP => self.sp = v,
            _ => panic!("CPU::store16 unexpected {}", op),
        }
    }

    fn push8(&mut self, v: u8) {
        self.sp -= 1;
        self.write(self.sp, v);
        self.tick();
    }

    fn pop8(&mut self) -> u8 {
        let v = self.read(self.sp);
        self.sp += 1;
        self.tick();
        v
    }

    fn push16(&mut self, v: u16) {
        let bs = v.to_be_bytes();
        self.push8(bs[0]);
        self.push8(bs[1]);
    }

    fn pop16(&mut self) -> u16 {
        let l = self.pop8();
        let h = self.pop8();
        u16::from_be_bytes([h, l])
    }

    fn cond_flag(&mut self, op: OP) -> bool {
        match op {
            OP::Zero => self.get_zero(),
            OP::NotZero => !self.get_zero(),
            OP::Carry => self.get_carry(),
            OP::NotCarry => !self.get_carry(),
            OP::Always => true,
            _ => panic!("CPU::cond_flag unexpected {}", op),
        }
    }

    fn ld8(&mut self, op1: OP, op2: OP)  {
        self.log("LD", op1, op2, LogInfo::None);
        let n = self.load8(op2);
        self.store8(op1, n);
    }

    fn ld16(&mut self, op1: OP, op2: OP)  {
        self.log("LD", op1, op2, LogInfo::None);
        let n = self.load16(op2);
        self.store16(op1, n);
    }

    fn ld16_hl_sp_n(&mut self)  {
        let n = self.fetch8();

        self.pc -=1;
        self.log("LD", OP::HL, OP::SP, LogInfo::I8h(n as i8));
        self.pc +=1;
        
        let (a, carry, half) = add_signed_u8_carry_half(self.sp, n);
        self.set_hl(a);
        self.set_carry(carry);
        self.set_half(half);
        self.set_negative(false);
        self.set_zero(false);
        self.tick();
    }

    fn push(&mut self, op: OP)  {
        self.log("PUSH", op, OP::None, LogInfo::None);
        let v = self.load16(op);
        self.tick();
        self.push16(v);
    }

    fn pop(&mut self, op: OP)  {
        self.log("POP", op, OP::None, LogInfo::None);
        let n = self.pop16();
        self.store16(op, n);
    }

    fn add(&mut self, op: OP)  {
        self.log("ADD", op, OP::None, LogInfo::None);
        let (a, carry, half) = self.a.add_carry_half(self.load8(op));
        self.a = a;
        self.set_carry(carry);
        self.set_half(half);
        self.set_negative(false);
        self.set_zero(self.a == 0);
    }

    fn adc(&mut self, op: OP)  {
        self.log("ADC", op, OP::None, LogInfo::None);
        let (a, a_carry, a_half) = self.a.add_carry_half(self.load8(op));
        let (b, b_carry, b_half) = a.add_carry_half(self.get_carry() as u8);
        self.a = b;
        self.set_carry(a_carry || b_carry);
        self.set_half(a_half || b_half);
        self.set_negative(false);
        self.set_zero(self.a == 0);
    }

    fn sub(&mut self, op: OP)  {
        self.log("SUB", op, OP::None, LogInfo::None);
        let (a, carry, half) = self.a.sub_carry_half(self.load8(op));
        self.a = a;
        self.set_carry(carry);
        self.set_half(half);
        self.set_negative(true);
        self.set_zero(self.a == 0);
    }

    fn sbc(&mut self, op: OP)  {
        self.log("SBC", op, OP::None, LogInfo::None);
        let (a, a_carry, a_half) = self.a.sub_carry_half(self.load8(op));
        let (b, b_carry, b_half) = a.sub_carry_half(self.get_carry().into());
        self.a = b;
        self.set_carry(a_carry || b_carry);
        self.set_half(a_half || b_half);
        self.set_negative(true);
        self.set_zero(self.a == 0);
    }

    fn and_(&mut self, op: OP)  {
        self.log("AND", op, OP::None, LogInfo::None);
        self.a = self.a & self.load8(op);
        self.set_carry(false);
        self.set_half(true);
        self.set_negative(false);
        self.set_zero(self.a == 0);
    }

    fn or_(&mut self, op: OP)  {
        self.log("OR", op, OP::None, LogInfo::None);
        self.a = self.a | self.load8(op);
        self.set_carry(false);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(self.a == 0);
    }

    fn xor(&mut self, op: OP)  {
        self.log("XOR", op, OP::None, LogInfo::None);
        self.a = self.a ^ self.load8(op);
        self.set_carry(false);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(self.a == 0);
    }

    fn cp(&mut self, op: OP)  {
        self.log("CP", op, OP::None, LogInfo::None);
        let (a, carry, half) = self.a.sub_carry_half(self.load8(op));
        self.set_carry(carry);
        self.set_half(half);
        self.set_negative(true);
        self.set_zero(a == 0);
    }

    fn inc8(&mut self, op: OP)  {
        self.log("INC", op, OP::None, LogInfo::None);
        let (a, _, half) = self.load8(op).add_carry_half(1);
        self.store8(op, a);
        self.set_half(half);
        self.set_negative(false);
        self.set_zero(a == 0);
    }

    fn dec8(&mut self, op: OP)  {
        self.log("DEC", op, OP::None, LogInfo::None);
        let (a, _, half) = self.load8(op).sub_carry_half(1);
        self.store8(op, a);
        self.set_half(half);
        self.set_negative(true);
        self.set_zero(a == 0);
    }

    fn add_hl(&mut self, op: OP)  {
        self.log("ADD", OP::HL, op, LogInfo::None);
        let (a, carry, half) = self.get_hl().add_carry_half(self.load16(op));
        self.set_hl(a);
        self.set_carry(carry);
        self.set_half(half);
        self.set_negative(false);
    }

    fn add_sp_n(&mut self)  {
        let n = self.fetch8();

        self.pc -= 1;
        self.log("ADD", OP::SP, OP::N, LogInfo::I8h(n as i8));
        self.pc += 1;

        let (a, carry, half) = add_signed_u8_carry_half(self.sp, n);

        self.sp = a;
        self.set_carry(carry);
        self.set_half(half);
        self.set_negative(false);
        self.set_zero(false);
        self.tick();
        self.tick();
    }

    fn inc16(&mut self, op: OP)  {
        self.log("INC", op, OP::None, LogInfo::None);
        let a = self.load16(op).add_carry_half(1).0;
        self.store16(op, a);
    }

    fn dec16(&mut self, op: OP)  {
        self.log("DEC", op, OP::None, LogInfo::None);
        let a = self.load16(op).sub_carry_half(1).0;
        self.store16(op, a);
    }

    fn daa(&mut self)  {
        self.log("DAA", OP::None, OP::None, LogInfo::None);
        let mut adjust: u8 = 0;
        adjust |= if self.get_carry() { 0x60 } else { 0 };
        adjust |= if self.get_half()  { 0x06 } else { 0 };
        if !self.get_negative() {
            adjust |= if self.a & 0x0f > 0x09 { 0x06 } else { 0 };
            adjust |= if self.a > 0x99 { 0x60 } else { 0 };
            self.a += adjust;
        } else {
            self.a -= adjust;
        }
        self.set_carry(adjust >= 0x60);
        self.set_half(false);
        self.set_zero(self.a == 0);
    }
    

    fn cpl(&mut self)  {
        self.log("CPL", OP::None, OP::None, LogInfo::None);
        self.a ^= 0xff;
        self.set_half(true);
        self.set_negative(true);
    }

    fn ccf(&mut self)  {
        self.log("CCF", OP::None, OP::None, LogInfo::None);
        self.set_carry(!self.get_carry());
        self.set_half(false);
        self.set_negative(false);
    }

    fn scf(&mut self)  {
        self.log("SCF", OP::None, OP::None, LogInfo::None);
        self.set_carry(true);
        self.set_half(false);
        self.set_negative(false);
    }

    fn di(&mut self)  {
        self.log("DI", OP::None, OP::None, LogInfo::None);
        self.ime = false;
    }

    fn ei(&mut self)  {
        self.log("EI", OP::None, OP::None, LogInfo::None);
        self.ime = true;
    }

    fn halt(&mut self)  {
        self.log("HALT", OP::None, OP::None, LogInfo::None);
        self.halting = true;
    }

    fn stop(&mut self)  {
        self.log("STOP", OP::None, OP::None, LogInfo::None);
        //self.halting = true;
    }

    fn nop(&mut self)  {
        self.log("NOP", OP::None, OP::None, LogInfo::None);
        //self.tick();
    }

    fn jp(&mut self, op: OP)  {
        let nn = self.fetch16();

        self.pc -= 2;
        self.log("JP", op, OP::None, LogInfo::U16h(nn));
        self.pc += 2;

        if self.cond_flag(op) {
            self.pc = nn;
            self.tick();
        }
    }

    fn jp_p_hl(&mut self)  {
        let hl = self.get_hl();
        self.log("JP", OP::HL, OP::None, LogInfo::U16h(hl));
        self.pc = hl;
        self.tick();
    }

    fn jr(&mut self, op: OP)  {
        let n = self.fetch8();

        self.pc -= 1;
        self.log("JR", op, OP::None, LogInfo::I8h(n as i8));
        self.pc += 1;

        if self.cond_flag(op) {
            self.pc = add_signed_u8_carry_half(self.pc, n).0;
            self.tick();
        }
    }

    fn call(&mut self, op: OP)  {
        let nn = self.fetch16();

        self.pc -= 2;
        self.log("CALL", op, OP::None, LogInfo::U16h(nn));
        self.pc += 2;

        if self.cond_flag(op) {
            self.tick();
            self.push16(self.pc);
            self.pc = nn;
        }
    }

    fn ret(&mut self, op: OP)  {
        self.log("RET", op, OP::None, LogInfo::None);
        if self.cond_flag(op) {
            self.pc = self.pop16();
            self.tick();
        }
    }

    fn reti(&mut self) {
        let pc = self.pop16();

        self.sp -= 2;
        self.log("RETI", OP::None, OP::None, LogInfo::U16h(self.pc));
        self.sp += 2;

        self.pc = pc;
        self.tick();
        self.ime = true;
    }

    fn rst(&mut self, addr: u16)  {
        self.log("RST", OP::None, OP::None, LogInfo::U16h(addr));
        self.tick();
        self.push16(self.pc);
        self.pc = addr;
    }


    fn swap(&mut self, op: OP)  {
        self.log("SWAP", op, OP::None, LogInfo::None);
        let r = self.load8(op); 
        let a = (r << 4) | (r >> 4);
        self.store8(op, a);
        self.set_carry(false);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(a == 0);
    }

    fn rlc(&mut self, op: OP)  {
        self.log("RLC", op, OP::None, LogInfo::None);
        let r = self.load8(op);
        let c = r >> 7;
        let a = (r << 1) | c;
        self.store8(op, a);
        self.set_carry(c == 1);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(if op == OP::A_ { false } else { a == 0 });
    }
 
    fn rl(&mut self, op: OP)  {
        self.log("RL", op, OP::None, LogInfo::None);
        let r = self.load8(op);
        let a = (r << 1) | (self.get_carry() as u8);
        self.store8(op, a);
        self.set_carry(r >> 7 == 1);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(if op == OP::A_ { false } else { a == 0});
    }
 
    fn rrc(&mut self, op: OP)  {
        self.log("RRC", op, OP::None, LogInfo::None);
        let r = self.load8(op);
        let c = r & 1;
        let a = (c << 7) | (r >> 1);
        self.store8(op, a);
        self.set_carry(c == 1);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(if op == OP::A_ { false } else { a == 0 });
    }
 
    fn rr(&mut self, op: OP)  {
        self.log("RR", op, OP::None, LogInfo::None);
        let r = self.load8(op);
        let a = ((self.get_carry() as u8) << 7) | (r >> 1);
        self.store8(op, a);
        self.set_carry(r & 1 == 1);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(if op == OP::A_ { false } else { a == 0 });
    }
 
    fn sla(&mut self, op: OP)  {
        self.log("SLA", op, OP::None, LogInfo::None);
        let r = self.load8(op);
        let a = r << 1;
        self.store8(op, a);
        self.set_carry(r >> 7 == 1);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(a == 0);
    }

    fn sra(&mut self, op: OP)  {
        self.log("SRA", op, OP::None, LogInfo::None);
        let r = self.load8(op);
        let a = (r & 0b10000000) | (r >> 1);
        self.store8(op, a);
        self.set_carry(r & 1 == 1);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(a == 0);
    }

    fn srl(&mut self, op: OP)  {
        self.log("SRL", op, OP::None, LogInfo::None);
        let r = self.load8(op);
        let a = r >> 1;
        self.store8(op, a);
        self.set_carry(r & 1 == 1);
        self.set_half(false);
        self.set_negative(false);
        self.set_zero(a == 0);
    }

    fn bit(&mut self, n: u8, op: OP)  {
        self.log("BIT", op, OP::None, LogInfo::U8h(n));
        let a = self.load8(op).get_bit(n as usize);
        self.set_half(true);
        self.set_negative(false);
        self.set_zero(a == false);
    }

    fn set(&mut self, n: u8, op: OP)  {
        self.log("SET", op, OP::None, LogInfo::U8h(n));
        let n = *self.load8(op).set_bit(n as usize, true);
        self.store8(op, n);
    }

    fn res(&mut self, n: u8, op: OP)  {
        self.log("RES", op, OP::None, LogInfo::U8h(n));
        let n = *self.load8(op).set_bit(n as usize, false);
        self.store8(op, n);
    }


    fn execute(&mut self)  {
        let code = self.fetch8();
        match code {
            0x3e => self.ld8(OP::A, OP::N),
            0x06 => self.ld8(OP::B, OP::N),
            0x0e => self.ld8(OP::C, OP::N),
            0x16 => self.ld8(OP::D, OP::N),
            0x1e => self.ld8(OP::E, OP::N),
            0x26 => self.ld8(OP::H, OP::N),
            0x2e => self.ld8(OP::L, OP::N),
            0x7f => self.ld8(OP::A, OP::A),
            0x78 => self.ld8(OP::A, OP::B),
            0x79 => self.ld8(OP::A, OP::C),
            0x7a => self.ld8(OP::A, OP::D),
            0x7b => self.ld8(OP::A, OP::E),
            0x7c => self.ld8(OP::A, OP::H),
            0x7d => self.ld8(OP::A, OP::L),
            0x7e => self.ld8(OP::A, OP::P_HL),
            0x0a => self.ld8(OP::A, OP::P_BC),
            0x1a => self.ld8(OP::A, OP::P_DE),
            0x47 => self.ld8(OP::B, OP::A),
            0x40 => self.ld8(OP::B, OP::B),
            0x41 => self.ld8(OP::B, OP::C),
            0x42 => self.ld8(OP::B, OP::D),
            0x43 => self.ld8(OP::B, OP::E),
            0x44 => self.ld8(OP::B, OP::H),
            0x45 => self.ld8(OP::B, OP::L),
            0x46 => self.ld8(OP::B, OP::P_HL),
            0x4f => self.ld8(OP::C, OP::A),
            0x48 => self.ld8(OP::C, OP::B),
            0x49 => self.ld8(OP::C, OP::C),
            0x4a => self.ld8(OP::C, OP::D),
            0x4b => self.ld8(OP::C, OP::E),
            0x4c => self.ld8(OP::C, OP::H),
            0x4d => self.ld8(OP::C, OP::L),
            0x4e => self.ld8(OP::C, OP::P_HL),
            0x57 => self.ld8(OP::D, OP::A),
            0x50 => self.ld8(OP::D, OP::B),
            0x51 => self.ld8(OP::D, OP::C),
            0x52 => self.ld8(OP::D, OP::D),
            0x53 => self.ld8(OP::D, OP::E),
            0x54 => self.ld8(OP::D, OP::H),
            0x55 => self.ld8(OP::D, OP::L),
            0x56 => self.ld8(OP::D, OP::P_HL),
            0x5f => self.ld8(OP::E, OP::A),
            0x58 => self.ld8(OP::E, OP::B),
            0x59 => self.ld8(OP::E, OP::C),
            0x5a => self.ld8(OP::E, OP::D),
            0x5b => self.ld8(OP::E, OP::E),
            0x5c => self.ld8(OP::E, OP::H),
            0x5d => self.ld8(OP::E, OP::L),
            0x5e => self.ld8(OP::E, OP::P_HL),
            0x67 => self.ld8(OP::H, OP::A),
            0x60 => self.ld8(OP::H, OP::B),
            0x61 => self.ld8(OP::H, OP::C),
            0x62 => self.ld8(OP::H, OP::D),
            0x63 => self.ld8(OP::H, OP::E),
            0x64 => self.ld8(OP::H, OP::H),
            0x65 => self.ld8(OP::H, OP::L),
            0x66 => self.ld8(OP::H, OP::P_HL),
            0x6f => self.ld8(OP::L, OP::A),
            0x68 => self.ld8(OP::L, OP::B),
            0x69 => self.ld8(OP::L, OP::C),
            0x6a => self.ld8(OP::L, OP::D),
            0x6b => self.ld8(OP::L, OP::E),
            0x6c => self.ld8(OP::L, OP::H),
            0x6d => self.ld8(OP::L, OP::L),
            0x6e => self.ld8(OP::L, OP::P_HL),

            0x70 => self.ld8(OP::P_HL, OP::B),
            0x71 => self.ld8(OP::P_HL, OP::C),
            0x72 => self.ld8(OP::P_HL, OP::D),
            0x73 => self.ld8(OP::P_HL, OP::E),
            0x74 => self.ld8(OP::P_HL, OP::H),
            0x75 => self.ld8(OP::P_HL, OP::L),
            0x36 => self.ld8(OP::P_HL, OP::N),
            0x02 => self.ld8(OP::P_BC, OP::A),
            0x12 => self.ld8(OP::P_DE, OP::A),
            0x77 => self.ld8(OP::P_HL, OP::A),
            0xea => self.ld8(OP::P_NN, OP::A),

            0xf0 => self.ld8(OP::A, OP::P_FF00_N),
            0xf2 => self.ld8(OP::A, OP::P_FF00_C),
            0xfa => self.ld8(OP::A, OP::P_NN),
            0xe0 => self.ld8(OP::P_FF00_N, OP::A),
            0xe2 => self.ld8(OP::P_FF00_C, OP::A),

            0x22 => self.ld8(OP::P_HL_INC, OP::A),
            0x2a => self.ld8(OP::A, OP::P_HL_INC),
            0x32 => self.ld8(OP::P_HL_DEC, OP::A),
            0x3a => self.ld8(OP::A, OP::P_HL_DEC),

            0x01 => self.ld16(OP::BC, OP::NN),
            0x11 => self.ld16(OP::DE, OP::NN),
            0x21 => self.ld16(OP::HL, OP::NN),
            0x31 => self.ld16(OP::SP, OP::NN),
            0xf9 => self.ld16(OP::SP, OP::HL),
            0x08 => self.ld16(OP::P_NN, OP::SP),
            0xf8 => self.ld16_hl_sp_n(),

            0xf5 => self.push(OP::AF),
            0xc5 => self.push(OP::BC),
            0xd5 => self.push(OP::DE),
            0xe5 => self.push(OP::HL),
            0xf1 => self.pop(OP::AF),
            0xc1 => self.pop(OP::BC),
            0xd1 => self.pop(OP::DE),
            0xe1 => self.pop(OP::HL),

            0x87 => self.add(OP::A),
            0x80 => self.add(OP::B),
            0x81 => self.add(OP::C),
            0x82 => self.add(OP::D),
            0x83 => self.add(OP::E),
            0x84 => self.add(OP::H),
            0x85 => self.add(OP::L),
            0x86 => self.add(OP::P_HL),
            0xc6 => self.add(OP::N),

            0x8f => self.adc(OP::A),
            0x88 => self.adc(OP::B),
            0x89 => self.adc(OP::C),
            0x8a => self.adc(OP::D),
            0x8b => self.adc(OP::E),
            0x8c => self.adc(OP::H),
            0x8d => self.adc(OP::L),
            0x8e => self.adc(OP::P_HL),
            0xce => self.adc(OP::N),

            0x97 => self.sub(OP::A),
            0x90 => self.sub(OP::B),
            0x91 => self.sub(OP::C),
            0x92 => self.sub(OP::D),
            0x93 => self.sub(OP::E),
            0x94 => self.sub(OP::H),
            0x95 => self.sub(OP::L),
            0x96 => self.sub(OP::P_HL),
            0xd6 => self.sub(OP::N),

            0x9f => self.sbc(OP::A),
            0x98 => self.sbc(OP::B),
            0x99 => self.sbc(OP::C),
            0x9a => self.sbc(OP::D),
            0x9b => self.sbc(OP::E),
            0x9c => self.sbc(OP::H),
            0x9d => self.sbc(OP::L),
            0x9e => self.sbc(OP::P_HL),
            0xde => self.sbc(OP::N),

            0xa7 => self.and_(OP::A),
            0xa0 => self.and_(OP::B),
            0xa1 => self.and_(OP::C),
            0xa2 => self.and_(OP::D),
            0xa3 => self.and_(OP::E),
            0xa4 => self.and_(OP::H),
            0xa5 => self.and_(OP::L),
            0xa6 => self.and_(OP::P_HL),
            0xe6 => self.and_(OP::N),

            0xb7 => self.or_(OP::A),
            0xb0 => self.or_(OP::B),
            0xb1 => self.or_(OP::C),
            0xb2 => self.or_(OP::D),
            0xb3 => self.or_(OP::E),
            0xb4 => self.or_(OP::H),
            0xb5 => self.or_(OP::L),
            0xb6 => self.or_(OP::P_HL),
            0xf6 => self.or_(OP::N),

            0xaf => self.xor(OP::A),
            0xa8 => self.xor(OP::B),
            0xa9 => self.xor(OP::C),
            0xaa => self.xor(OP::D),
            0xab => self.xor(OP::E),
            0xac => self.xor(OP::H),
            0xad => self.xor(OP::L),
            0xae => self.xor(OP::P_HL),
            0xee => self.xor(OP::N),

            0xbf => self.cp(OP::A),
            0xb8 => self.cp(OP::B),
            0xb9 => self.cp(OP::C),
            0xba => self.cp(OP::D),
            0xbb => self.cp(OP::E),
            0xbc => self.cp(OP::H),
            0xbd => self.cp(OP::L),
            0xbe => self.cp(OP::P_HL),
            0xfe => self.cp(OP::N),

            0x3c => self.inc8(OP::A),
            0x04 => self.inc8(OP::B),
            0x0c => self.inc8(OP::C),
            0x14 => self.inc8(OP::D),
            0x1c => self.inc8(OP::E),
            0x24 => self.inc8(OP::H),
            0x2c => self.inc8(OP::L),
            0x34 => self.inc8(OP::P_HL),

            0x3d => self.dec8(OP::A),
            0x05 => self.dec8(OP::B),
            0x0d => self.dec8(OP::C),
            0x15 => self.dec8(OP::D),
            0x1d => self.dec8(OP::E),
            0x25 => self.dec8(OP::H),
            0x2d => self.dec8(OP::L),
            0x35 => self.dec8(OP::P_HL),

            0x09 => self.add_hl(OP::BC),
            0x19 => self.add_hl(OP::DE),
            0x29 => self.add_hl(OP::HL),
            0x39 => self.add_hl(OP::SP),
            0xe8 => self.add_sp_n(),

            0x03 => self.inc16(OP::BC),
            0x13 => self.inc16(OP::DE),
            0x23 => self.inc16(OP::HL),
            0x33 => self.inc16(OP::SP),

            0x0b => self.dec16(OP::BC),
            0x1b => self.dec16(OP::DE),
            0x2b => self.dec16(OP::HL),
            0x3b => self.dec16(OP::SP),

            0x07 => self.rlc(OP::A_),
            0x17 => self.rl(OP::A_),
            0x0f => self.rrc(OP::A_),
            0x1f => self.rr(OP::A_),

            0x27 => self.daa(),
            0x2f => self.cpl(),
            0x3f => self.ccf(),
            0x37 => self.scf(),
            0xf3 => self.di(),
            0xfb => self.ei(),
            0x76 => self.halt(),
            0x00 => self.nop(),

            0xc3 => self.jp(OP::Always),
            0xc2 => self.jp(OP::NotZero),
            0xca => self.jp(OP::Zero),
            0xd2 => self.jp(OP::NotCarry),
            0xda => self.jp(OP::Carry),
            0xe9 => self.jp_p_hl(),
            0x18 => self.jr(OP::Always),
            0x20 => self.jr(OP::NotZero),
            0x28 => self.jr(OP::Zero),
            0x30 => self.jr(OP::NotCarry),
            0x38 => self.jr(OP::Carry),
            0xcd => self.call(OP::Always),
            0xc4 => self.call(OP::NotZero),
            0xcc => self.call(OP::Zero),
            0xd4 => self.call(OP::NotCarry),
            0xdc => self.call(OP::Carry),
            0xc7 => self.rst(0x00),
            0xcf => self.rst(0x08),
            0xd7 => self.rst(0x10),
            0xdf => self.rst(0x18),
            0xe7 => self.rst(0x20),
            0xef => self.rst(0x28),
            0xf7 => self.rst(0x30),
            0xff => self.rst(0x38),
            0xc9 => self.ret(OP::Always),
            0xc0 => self.ret(OP::NotZero),
            0xc8 => self.ret(OP::Zero),
            0xd0 => self.ret(OP::NotCarry),
            0xd8 => self.ret(OP::Carry),
            0xd9 => self.reti(),

            0x10 => {
                let code10 = self.fetch8();
                match code10 {
                    0x00 => self.stop(),
                    _ => panic!("CPU.execute: undefined instruction 0x10 0x{:x}", code10),
                }
            },

            0xcb => {
                let code_cb = self.fetch8();
                match code_cb {
                    0x37 => self.swap(OP::A),
                    0x30 => self.swap(OP::B),
                    0x31 => self.swap(OP::C),
                    0x32 => self.swap(OP::D),
                    0x33 => self.swap(OP::E),
                    0x34 => self.swap(OP::H),
                    0x35 => self.swap(OP::L),
                    0x36 => self.swap(OP::P_HL),

                    0x07 => self.rlc(OP::A),
                    0x00 => self.rlc(OP::B),
                    0x01 => self.rlc(OP::C),
                    0x02 => self.rlc(OP::D),
                    0x03 => self.rlc(OP::E),
                    0x04 => self.rlc(OP::H),
                    0x05 => self.rlc(OP::L),
                    0x06 => self.rlc(OP::P_HL),

                    0x17 => self.rl(OP::A),
                    0x10 => self.rl(OP::B),
                    0x11 => self.rl(OP::C),
                    0x12 => self.rl(OP::D),
                    0x13 => self.rl(OP::E),
                    0x14 => self.rl(OP::H),
                    0x15 => self.rl(OP::L),
                    0x16 => self.rl(OP::P_HL),

                    0x0f => self.rrc(OP::A),
                    0x08 => self.rrc(OP::B),
                    0x09 => self.rrc(OP::C),
                    0x0a => self.rrc(OP::D),
                    0x0b => self.rrc(OP::E),
                    0x0c => self.rrc(OP::H),
                    0x0d => self.rrc(OP::L),
                    0x0e => self.rrc(OP::P_HL),

                    0x1f => self.rr(OP::A),
                    0x18 => self.rr(OP::B),
                    0x19 => self.rr(OP::C),
                    0x1a => self.rr(OP::D),
                    0x1b => self.rr(OP::E),
                    0x1c => self.rr(OP::H),
                    0x1d => self.rr(OP::L),
                    0x1e => self.rr(OP::P_HL),

                    0x27 => self.sla(OP::A),
                    0x20 => self.sla(OP::B),
                    0x21 => self.sla(OP::C),
                    0x22 => self.sla(OP::D),
                    0x23 => self.sla(OP::E),
                    0x24 => self.sla(OP::H),
                    0x25 => self.sla(OP::L),
                    0x26 => self.sla(OP::P_HL),

                    0x2f => self.sra(OP::A),
                    0x28 => self.sra(OP::B),
                    0x29 => self.sra(OP::C),
                    0x2a => self.sra(OP::D),
                    0x2b => self.sra(OP::E),
                    0x2c => self.sra(OP::H),
                    0x2d => self.sra(OP::L),
                    0x2e => self.sra(OP::P_HL),

                    0x3f => self.srl(OP::A),
                    0x38 => self.srl(OP::B),
                    0x39 => self.srl(OP::C),
                    0x3a => self.srl(OP::D),
                    0x3b => self.srl(OP::E),
                    0x3c => self.srl(OP::H),
                    0x3d => self.srl(OP::L),
                    0x3e => self.srl(OP::P_HL),

                    0x47 => self.bit(0, OP::A),
                    0x40 => self.bit(0, OP::B),
                    0x41 => self.bit(0, OP::C),
                    0x42 => self.bit(0, OP::D),
                    0x43 => self.bit(0, OP::E),
                    0x44 => self.bit(0, OP::H),
                    0x45 => self.bit(0, OP::L),
                    0x46 => self.bit(0, OP::P_HL),
                    0x4f => self.bit(1, OP::A),
                    0x48 => self.bit(1, OP::B),
                    0x49 => self.bit(1, OP::C),
                    0x4a => self.bit(1, OP::D),
                    0x4b => self.bit(1, OP::E),
                    0x4c => self.bit(1, OP::H),
                    0x4d => self.bit(1, OP::L),
                    0x4e => self.bit(1, OP::P_HL),
                    0x57 => self.bit(2, OP::A),
                    0x50 => self.bit(2, OP::B),
                    0x51 => self.bit(2, OP::C),
                    0x52 => self.bit(2, OP::D),
                    0x53 => self.bit(2, OP::E),
                    0x54 => self.bit(2, OP::H),
                    0x55 => self.bit(2, OP::L),
                    0x56 => self.bit(2, OP::P_HL),
                    0x5f => self.bit(3, OP::A),
                    0x58 => self.bit(3, OP::B),
                    0x59 => self.bit(3, OP::C),
                    0x5a => self.bit(3, OP::D),
                    0x5b => self.bit(3, OP::E),
                    0x5c => self.bit(3, OP::H),
                    0x5d => self.bit(3, OP::L),
                    0x5e => self.bit(3, OP::P_HL),
                    0x67 => self.bit(4, OP::A),
                    0x60 => self.bit(4, OP::B),
                    0x61 => self.bit(4, OP::C),
                    0x62 => self.bit(4, OP::D),
                    0x63 => self.bit(4, OP::E),
                    0x64 => self.bit(4, OP::H),
                    0x65 => self.bit(4, OP::L),
                    0x66 => self.bit(4, OP::P_HL),
                    0x6f => self.bit(5, OP::A),
                    0x68 => self.bit(5, OP::B),
                    0x69 => self.bit(5, OP::C),
                    0x6a => self.bit(5, OP::D),
                    0x6b => self.bit(5, OP::E),
                    0x6c => self.bit(5, OP::H),
                    0x6d => self.bit(5, OP::L),
                    0x6e => self.bit(5, OP::P_HL),
                    0x77 => self.bit(6, OP::A),
                    0x70 => self.bit(6, OP::B),
                    0x71 => self.bit(6, OP::C),
                    0x72 => self.bit(6, OP::D),
                    0x73 => self.bit(6, OP::E),
                    0x74 => self.bit(6, OP::H),
                    0x75 => self.bit(6, OP::L),
                    0x76 => self.bit(6, OP::P_HL),
                    0x7f => self.bit(7, OP::A),
                    0x78 => self.bit(7, OP::B),
                    0x79 => self.bit(7, OP::C),
                    0x7a => self.bit(7, OP::D),
                    0x7b => self.bit(7, OP::E),
                    0x7c => self.bit(7, OP::H),
                    0x7d => self.bit(7, OP::L),
                    0x7e => self.bit(7, OP::P_HL),

                    0xc7 => self.set(0, OP::A),
                    0xc0 => self.set(0, OP::B),
                    0xc1 => self.set(0, OP::C),
                    0xc2 => self.set(0, OP::D),
                    0xc3 => self.set(0, OP::E),
                    0xc4 => self.set(0, OP::H),
                    0xc5 => self.set(0, OP::L),
                    0xc6 => self.set(0, OP::P_HL),
                    0xcf => self.set(1, OP::A),
                    0xc8 => self.set(1, OP::B),
                    0xc9 => self.set(1, OP::C),
                    0xca => self.set(1, OP::D),
                    0xcb => self.set(1, OP::E),
                    0xcc => self.set(1, OP::H),
                    0xcd => self.set(1, OP::L),
                    0xce => self.set(1, OP::P_HL),
                    0xd7 => self.set(2, OP::A),
                    0xd0 => self.set(2, OP::B),
                    0xd1 => self.set(2, OP::C),
                    0xd2 => self.set(2, OP::D),
                    0xd3 => self.set(2, OP::E),
                    0xd4 => self.set(2, OP::H),
                    0xd5 => self.set(2, OP::L),
                    0xd6 => self.set(2, OP::P_HL),
                    0xdf => self.set(3, OP::A),
                    0xd8 => self.set(3, OP::B),
                    0xd9 => self.set(3, OP::C),
                    0xda => self.set(3, OP::D),
                    0xdb => self.set(3, OP::E),
                    0xdc => self.set(3, OP::H),
                    0xdd => self.set(3, OP::L),
                    0xde => self.set(3, OP::P_HL),
                    0xe7 => self.set(4, OP::A),
                    0xe0 => self.set(4, OP::B),
                    0xe1 => self.set(4, OP::C),
                    0xe2 => self.set(4, OP::D),
                    0xe3 => self.set(4, OP::E),
                    0xe4 => self.set(4, OP::H),
                    0xe5 => self.set(4, OP::L),
                    0xe6 => self.set(4, OP::P_HL),
                    0xef => self.set(5, OP::A),
                    0xe8 => self.set(5, OP::B),
                    0xe9 => self.set(5, OP::C),
                    0xea => self.set(5, OP::D),
                    0xeb => self.set(5, OP::E),
                    0xec => self.set(5, OP::H),
                    0xed => self.set(5, OP::L),
                    0xee => self.set(5, OP::P_HL),
                    0xf7 => self.set(6, OP::A),
                    0xf0 => self.set(6, OP::B),
                    0xf1 => self.set(6, OP::C),
                    0xf2 => self.set(6, OP::D),
                    0xf3 => self.set(6, OP::E),
                    0xf4 => self.set(6, OP::H),
                    0xf5 => self.set(6, OP::L),
                    0xf6 => self.set(6, OP::P_HL),
                    0xff => self.set(7, OP::A),
                    0xf8 => self.set(7, OP::B),
                    0xf9 => self.set(7, OP::C),
                    0xfa => self.set(7, OP::D),
                    0xfb => self.set(7, OP::E),
                    0xfc => self.set(7, OP::H),
                    0xfd => self.set(7, OP::L),
                    0xfe => self.set(7, OP::P_HL),

                    0x87 => self.res(0, OP::A),
                    0x80 => self.res(0, OP::B),
                    0x81 => self.res(0, OP::C),
                    0x82 => self.res(0, OP::D),
                    0x83 => self.res(0, OP::E),
                    0x84 => self.res(0, OP::H),
                    0x85 => self.res(0, OP::L),
                    0x86 => self.res(0, OP::P_HL),
                    0x8f => self.res(1, OP::A),
                    0x88 => self.res(1, OP::B),
                    0x89 => self.res(1, OP::C),
                    0x8a => self.res(1, OP::D),
                    0x8b => self.res(1, OP::E),
                    0x8c => self.res(1, OP::H),
                    0x8d => self.res(1, OP::L),
                    0x8e => self.res(1, OP::P_HL),
                    0x97 => self.res(2, OP::A),
                    0x90 => self.res(2, OP::B),
                    0x91 => self.res(2, OP::C),
                    0x92 => self.res(2, OP::D),
                    0x93 => self.res(2, OP::E),
                    0x94 => self.res(2, OP::H),
                    0x95 => self.res(2, OP::L),
                    0x96 => self.res(2, OP::P_HL),
                    0x9f => self.res(3, OP::A),
                    0x98 => self.res(3, OP::B),
                    0x99 => self.res(3, OP::C),
                    0x9a => self.res(3, OP::D),
                    0x9b => self.res(3, OP::E),
                    0x9c => self.res(3, OP::H),
                    0x9d => self.res(3, OP::L),
                    0x9e => self.res(3, OP::P_HL),
                    0xa7 => self.res(4, OP::A),
                    0xa0 => self.res(4, OP::B),
                    0xa1 => self.res(4, OP::C),
                    0xa2 => self.res(4, OP::D),
                    0xa3 => self.res(4, OP::E),
                    0xa4 => self.res(4, OP::H),
                    0xa5 => self.res(4, OP::L),
                    0xa6 => self.res(4, OP::P_HL),
                    0xaf => self.res(5, OP::A),
                    0xa8 => self.res(5, OP::B),
                    0xa9 => self.res(5, OP::C),
                    0xaa => self.res(5, OP::D),
                    0xab => self.res(5, OP::E),
                    0xac => self.res(5, OP::H),
                    0xad => self.res(5, OP::L),
                    0xae => self.res(5, OP::P_HL),
                    0xb7 => self.res(6, OP::A),
                    0xb0 => self.res(6, OP::B),
                    0xb1 => self.res(6, OP::C),
                    0xb2 => self.res(6, OP::D),
                    0xb3 => self.res(6, OP::E),
                    0xb4 => self.res(6, OP::H),
                    0xb5 => self.res(6, OP::L),
                    0xb6 => self.res(6, OP::P_HL),
                    0xbf => self.res(7, OP::A),
                    0xb8 => self.res(7, OP::B),
                    0xb9 => self.res(7, OP::C),
                    0xba => self.res(7, OP::D),
                    0xbb => self.res(7, OP::E),
                    0xbc => self.res(7, OP::H),
                    0xbd => self.res(7, OP::L),
                    0xbe => self.res(7, OP::P_HL),
                    //_ => panic!("CPU.execute: undefined instruction 0xcb {:#x}", code_cb),
                }
            },

            _ => panic!("CPU.execute: undefined instruction {:#x}", code),
        }
    }


    fn serial(&mut self) {
        let mut sc = self.read_reg(Reg::SC);
        if sc.get_bit(7) {
            let clock_list: [usize; 4] = [512, 256, 16, 8];
            let clock = clock_list[(sc & 0b11) as usize];
            if self.sys_counter % clock == 0 {
                let sb = self.read_reg(Reg::SB);

                self.serial_logger.write(sb);

                self.write_reg(Reg::SC, *sc.set_bit(7, false));
                self.modify_reg(Reg::IF, |mut u| *u.set_bit(3, true));
            }
        }
    }

    fn timer(&mut self) {
        if self.sys_counter % 256 == 0 {
            self.modify_reg(Reg::DIV, |u| u + 1);
        }

        let tac = self.read_reg(Reg::TAC);
        if tac.get_bit(2) {
            let clock_list: [usize; 4] = [1024, 16, 64, 256];
            let clock = clock_list[(tac & 0b11) as usize];
            if self.sys_counter % clock == 0 {
                let (tima, carry) = self.read_reg(Reg::TIMA).overflowing_add(1);
                if carry {
                    self.modify_reg(Reg::IF, |mut u| *u.set_bit(2, true));
                    self.write_reg(Reg::TIMA, self.read_reg(Reg::TMA));
                } else {
                    self.write_reg(Reg::TIMA, tima);
                }
            }
        }
    }

    fn joypad(&mut self) {
        let jb = self.joypad_buffer;
        let jp = self.read_reg(Reg::JOYP);
        if !jp.get_bit(4) {
            self.write_reg(Reg::JOYP, 0b100000 | jb & 0b1111);
            self.modify_reg(Reg::IF, |mut u| *u.set_bit(4, true));
        }
        if !jp.get_bit(5) {
            self.write_reg(Reg::JOYP, 0b010000 | jb >> 4);
            self.modify_reg(Reg::IF, |mut u| *u.set_bit(4, true));
        }
    }

    fn interrupt(&mut self) {
        if self.read_reg(Reg::IE) & self.read_reg(Reg::IF) != 0 {
            self.halting = false;
        }

        if self.ime {
            //self.halting = false;

            let enable = self.read_reg(Reg::IE);
            let request = self.read_reg(Reg::IF);
            let (addr, n, _name) = if enable.get_bit(0) && request.get_bit(0) {
                (0x40, 0, "VBlack")
            } else if enable.get_bit(1) && request.get_bit(1) {
                (0x48, 1, "LSTAT")
            } else if enable.get_bit(2) && request.get_bit(2) {
                (0x50, 2, "Timer")
            } else if enable.get_bit(3) && request.get_bit(3) {
                (0x58, 3, "Serial")
            } else if enable.get_bit(4) && request.get_bit(4) {
                (0x60, 4, "Joypad")
            } else {
                (0, 0, "")
            };

            if addr != 0 {
                self.push16(self.pc);
                self.pc = addr;
                self.ime = false;
                self.halting = false;

                self.write_reg(Reg::IF, *self.read_reg(Reg::IF).set_bit(n, false));

                self.tick();
                self.tick();
                self.tick();
            }
        }
    }

    pub fn step(&mut self) {
        self.cycle = 0;

        if self.halting {
            self.tick();
        } else {
            self.execute();
            self.exe_counter += 1;
        }


        while self.cycle > 0 {
            self.cycle -= 1;
            for _ in 0..4 {
                self.ppu.step();
                self.serial();
                self.timer();
                self.joypad();
                self.interrupt();
                self.sys_counter += 1;
            }
        }
    }
}
