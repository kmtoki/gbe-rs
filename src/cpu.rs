
use std::rc::RC;

use mbc::MBC;

#[drive(Debug)]
struct CPULog {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,

    zero: u8,
    negative: u8,
    half: u8,
    carry: u8,

    code: String,
    bank: u16,
    counter: u64,
}

#[drive(Display, Debug)]
struct OP = A | F | B | C | D | E | H | L | AF | BC | DE | HL | NZ | Z | NC | C | NONE

#[drive(Debug)]
pub struct CPU {
    mbc: Rc<MBC>,

    a: u8,
    f: u8, // always ignore lower 4bit 0bZNHC0000
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,

    ime: u8,
    cycle: u8,

    log: Vec<CPU>,
    log_limit: u64,
    is_log: bool,

    counter: u64,
}

impl CPU {
    fn new() -> Self {
        CPU {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0xfffe,
            pc: 0,

            ime: 0
            cycle: 0,

            log: vec![],
            log_limit: 0xffff,
            is_log: true,

            counter: 0,
        }
    }

    macro_rules! log {
        ($self:ident, $t:expr, $($arg:expr),*) => {
            if $self.is_log {
                let log = CPULog {
                    a: $self.a,
                    f: $self.f,
                    b: $self.b,
                    c: $self.c,
                    d: $self.d,
                    e: $self.e,
                    h: $self.h,
                    l: $self.l,
                    sp: $self.sp,
                    pc: $self.pc,

                    zero: $self.zero(),
                    negative: $self.negative(),
                    half: $self.half(),
                    carry: $self.carry(),

                    bank: $self.mbc.bank,
                    counter: $self.counter,

                    code: format!($t, $($arg,)*),
                }
                $self.log.push(log);
            }
        }
    }




    fn af(&self) -> u16 { (self.a as u16) << 8 | (self.f as u16) }
    fn bc(&self) -> u16 { (self.b as u16) << 8 | (self.c as u16) }
    fn de(&self) -> u16 { (self.d as u16) << 8 | (self.e as u16) }
    fn hl(&self) -> u16 { (self.h as u16) << 8 | (self.l as u16) }

    fn set_af(&mut self, af: u16) { self.a = (af >> 8 & 0xff) as u8); self.f = (af & 0xff) as u8; }
    fn set_bc(&mut self, bc: u16) { self.b = (bc >> 8 & 0xff) as u8); self.c = (bc & 0xff) as u8; }
    fn set_de(&mut self, de: u16) { self.d = (de >> 8 & 0xff) as u8); self.e = (de & 0xff) as u8; }
    fn set_hl(&mut self, hl: u16) { self.h = (hl >> 8 & 0xff) as u8); self.l = (hl & 0xff) as u8; }

    fn zero(&self) -> u8     { self.f >> 7 & 1 }
    fn negative(&self) -> u8 { self.f >> 6 & 1 }
    fn half(&self) -> u8     { self.f >> 5 & 1 }
    fn carry(&self) -> u8    { self.f >> 4 & 1 }

    fn bitset(b: u8, i: u8, v: u8) -> u8 { (b & !(1 << i)) | (v << i) }

    fn set_zero(&mut self, z: u8) -> u8     { self.f = bitset(self.f, 7, z) }
    fn set_negative(&mut self, n: u8) -> u8 { self.f = bitset(self.f, 6, n) }
    fn set_half(&mut self, h: u8) -> u8     { self.f = bitset(self.f, 5, h) }
    fn set_carry(&mut self, c: u8) -> u8    { self.f = bitset(self.f, 4, c) }

    fn _sub_u16(&mut self, u1: u16, u2: u16) -> u16 {
        let (_, half) = (u1 & 0x7ff).overflowing_sub(u2 & 0x7ff);
        let (u3, carry) = u1.overflowing_sub(u2);

        if half {
            self.set_half(1);
        }

        if carry {
            self.set_carry(1);
        }

        u3
    }


    fn execute_one(&mut self) {
        use OP::*;

        self.counter += 1;
        let instruction = self.mbc.read(self.pc);
        let op1 = self.mbc.read(self.pc + 1);
        let op2 = self.mbc.read(self.pc + 2);
        match instruction {
            0x00 -> self.nop(),
            0x3e -> self.ld_r_u8(A, op1),
            0x06 -> self.ld_r_u8(B, op1),
            0x0e -> self.ld_r_u8(C, op1),
            0x16 -> self.ld_r_u8(D, op1),
            0x1e -> self.ld_r_u8(E, op1),
            0x26 -> self.ld_r_u8(H, op1),
            0x2e -> self.ld_r_u8(L, op1),
            0x7f -> self.ld_r_r(A, self.a, A),
            0x78 -> self.ld_r_r(A, self.b, B),
            0x79 -> self.ld_r_r(A, self.c, C),
            0x7a -> self.ld_r_r(A, self.d, D),
            0x7b -> self.ld_r_r(A, self.e, E),
            0x7c -> self.ld_r_r(A, self.h, H),
            0x7d -> self.ld_r_r(A, self.l, L),
            0x7e -> self.ld_r_m(A, self.hl(), HL),
            0x47 -> self.ld_r_r(B, self.a, A),
            0x40 -> self.ld_r_r(B, self.b, B),
            0x41 -> self.ld_r_r(B, self.c, C),
            0x42 -> self.ld_r_r(B, self.d, D),
            0x43 -> self.ld_r_r(B, self.e, E),
            0x44 -> self.ld_r_r(B, self.h, H),
            0x45 -> self.ld_r_r(B, self.l, L),
            0x46 -> self.ld_r_m(B, self.hl(), HL),
            0x4f -> self.ld_r_r(C, self.a, A),
            0x48 -> self.ld_r_r(C, self.b, B),
            0x49 -> self.ld_r_r(C, self.c, C),
            0x4a -> self.ld_r_r(C, self.d, D),
            0x4b -> self.ld_r_r(C, self.e, E),
            0x4c -> self.ld_r_r(C, self.h, H),
            0x4d -> self.ld_r_r(C, self.l, L),
            0x4e -> self.ld_r_m(C, self.hl(), HL),
            0x57 -> self.ld_r_r(D, self.a, A),
            0x50 -> self.ld_r_r(D, self.b, B),
            0x51 -> self.ld_r_r(D, self.c, C),
            0x52 -> self.ld_r_r(D, self.d, D),
            0x53 -> self.ld_r_r(D, self.e, E),
            0x54 -> self.ld_r_r(D, self.h, H),
            0x55 -> self.ld_r_r(D, self.l, L),
            0x56 -> self.ld_r_m(D, self.hl(), HL),
            0x5f -> self.ld_r_r(E, self.a, A),
            0x58 -> self.ld_r_r(E, self.b, B),
            0x59 -> self.ld_r_r(E, self.c, C),
            0x5a -> self.ld_r_r(E, self.d, D),
            0x5b -> self.ld_r_r(E, self.e, E),
            0x5c -> self.ld_r_r(E, self.h, H),
            0x5d -> self.ld_r_r(E, self.l, L),
            0x5e -> self.ld_r_m(E, self.hl(), HL),
            0x67 -> self.ld_r_r(H, self.a, A),
            0x60 -> self.ld_r_r(H, self.b, B),
            0x61 -> self.ld_r_r(H, self.c, C),
            0x62 -> self.ld_r_r(H, self.d, D),
            0x63 -> self.ld_r_r(H, self.e, E),
            0x64 -> self.ld_r_r(H, self.h, H),
            0x65 -> self.ld_r_r(H, self.l, L),
            0x66 -> self.ld_r_m(H, self.hl(), HL),
            0x6f -> self.ld_r_r(L, self.a, A),
            0x68 -> self.ld_r_r(L, self.b, B),
            0x69 -> self.ld_r_r(L, self.c, C),
            0x6a -> self.ld_r_r(L, self.d, D),
            0x6b -> self.ld_r_r(L, self.e, E),
            0x6c -> self.ld_r_r(L, self.h, H),
            0x6d -> self.ld_r_r(L, self.l, L),
            0x6e -> self.ld_r_m(L, self.hl(), HL),
            0x70 -> self.ld_m_r(self.hl(), self.b, HL, B),
            0x71 -> self.ld_m_r(self.hl(), self.c, HL, C),
            0x72 -> self.ld_m_r(self.hl(), self.d, HL, D),
            0x73 -> self.ld_m_r(self.hl(), self.e, HL, E),
            0x74 -> self.ld_m_r(self.hl(), self.h, HL, H),
            0x75 -> self.ld_m_r(self.hl(), self.l, HL, L),
            0x36 -> self.ld_m_u8(self.hl(), op1, HL),
            0x0a -> self.ld_r_m(A, self.bc, BC),
            0x1a -> self.ld_r_m(A, self.de, DE),
            0xfa -> self.ld_r_m_i16(A, op16),
            0x02 -> self.ld_m_r(self.bc, self.a, BC, A),
            0x12 -> self.ld_m_r(self.de, self.a, DE, A),
            0x77 -> self.ld_m_r(self.hl(), self.a, HL, A),
            0xea -> self.ld_m_i16_r(op16, self.a, A),
            0xf2 -> self.ld_r_m(A, 0xFF00 + self.c, NONE),
            0xe2 -> self.ld_m_r(0xFF00 + self.c, self.a, A),
            0x3a -> self.ldd_a_m_hl(),
            0x32 -> self.ldd_m_hl_a(),

            0x2a -> self.ldi_a_m_hl(),
            0x22 -> self.ldi_m_hl_a(),

            0xe0 -> self.ldh_m_r(op1, self.a, A),
            0xf0 -> self.ldh_r_m(A, op1),

            0x01 -> self.ld_rr_i16(BC, op16),
            0x11 -> self.ld_rr_i16(DE, op16),
            0x21 -> self.ld_rr_i16(HL, op16),
            0x31 -> self.ld_rr_i16(SP, op16),

            0xf9 -> self.ld_sp_hl(),
            0xf8 -> self.ld_hl_sp_i8(op1),
            0x08 -> self.ld_m_i16_sp(op16),

            0xf5 -> self.push_rr(self.a, self.f, AF),
            0xc5 -> self.push_rr(self.b, self.c, BC),
            0xd5 -> self.push_rr(self.d, self.e, DE),
            0xe5 -> self.push_rr(self.h, self.l, HL),

            0xf1 -> self.pop_rr(AF),
            0xc1 -> self.pop_rr(BC),
            0xd1 -> self.pop_rr(DE),
            0xe1 -> self.pop_rr(HL),

            0x87 -> self.add_a_r(self.a, A),
            0x80 -> self.add_a_r(self.b, B),
            0x81 -> self.add_a_r(self.c, C),
            0x82 -> self.add_a_r(self.d, D),
            0x83 -> self.add_a_r(self.e, E),
            0x84 -> self.add_a_r(self.h, H),
            0x85 -> self.add_a_r(self.l, L),
            0x86 -> self.add_a_m_hl(),
            0xc6 -> self.add_a_u8(op1),

            0x8f -> self.adc_a_r(self.a, A),
            0x88 -> self.adc_a_r(self.b, B),
            0x89 -> self.adc_a_r(self.c, C),
            0x8a -> self.adc_a_r(self.d, D),
            0x8b -> self.adc_a_r(self.e, E),
            0x8c -> self.adc_a_r(self.h, H),
            0x8d -> self.adc_a_r(self.l, L),
            0x8e -> self.adc_a_m_hl(),
            0xce -> self.adc_a_u8(op1),

            0x97 -> self.sub_a_r(self.a, A),
            0x90 -> self.sub_a_r(self.b, B),
            0x91 -> self.sub_a_r(self.c, C),
            0x92 -> self.sub_a_r(self.d, D),
            0x93 -> self.sub_a_r(self.e, E),
            0x94 -> self.sub_a_r(self.h, H),
            0x95 -> self.sub_a_r(self.l, L),
            0x96 -> self.sub_a_m_hl(),
            0xd6 -> self.sub_a_u8(op1),

            0x9f -> self.sbc_a_r(self.a, A),
            0x98 -> self.sbc_a_r(self.b, B),
            0x99 -> self.sbc_a_r(self.c, C),
            0x9a -> self.sbc_a_r(self.d, D),
            0x9b -> self.sbc_a_r(self.e, E),
            0x9c -> self.sbc_a_r(self.h, H),
            0x9d -> self.sbc_a_r(self.l, L),
            0x9e -> self.sbc_a_m_hl(),
            0xde -> self.sbc_a_u8(op1),

            0xa7 -> self.and_a_r(self.a, A),
            0xa0 -> self.and_a_r(self.b, B),
            0xa1 -> self.and_a_r(self.c, C),
            0xa2 -> self.and_a_r(self.d, D),
            0xa3 -> self.and_a_r(self.e, E),
            0xa4 -> self.and_a_r(self.h, H),
            0xa5 -> self.and_a_r(self.l, L),
            0xa6 -> self.and_a_m_hl(),
            0xe6 -> self.and_a_u8(op1),

            0xb7 -> self.or_a_r(self.a, A),
            0xb0 -> self.or_a_r(self.b, B),
            0xb1 -> self.or_a_r(self.c, C),
            0xb2 -> self.or_a_r(self.d, D),
            0xb3 -> self.or_a_r(self.e, E),
            0xb4 -> self.or_a_r(self.h, H),
            0xb5 -> self.or_a_r(self.l, L),
            0xb6 -> self.or_a_m_hl(),
            0xf6 -> self.or_a_u8(op1),

            0xaf -> self.xor_a_r(self.a, A),
            0xa8 -> self.xor_a_r(self.b, B),
            0xa9 -> self.xor_a_r(self.c, C),
            0xaa -> self.xor_a_r(self.d, D),
            0xab -> self.xor_a_r(self.e, E),
            0xac -> self.xor_a_r(self.h, H),
            0xad -> self.xor_a_r(self.l, L),
            0xae -> self.xor_a_m_hl(),
            0xee -> self.xor_a_u8(op1),

            0xbf -> self.cp_a_r(self.a, A),
            0xb8 -> self.cp_a_r(self.b, B),
            0xb9 -> self.cp_a_r(self.c, C),
            0xba -> self.cp_a_r(self.d, D),
            0xbb -> self.cp_a_r(self.e, E),
            0xbc -> self.cp_a_r(self.h, H),
            0xbd -> self.cp_a_r(self.l, L),
            0xbe -> self.cp_a_m_hl(),
            0xfe -> self.cp_a_u8(op1),

            0x3c -> self.inc_r(A),
            0x04 -> self.inc_r(B),
            0x0c -> self.inc_r(C),
            0x14 -> self.inc_r(D),
            0x1c -> self.inc_r(E),
            0x24 -> self.inc_r(H),
            0x2c -> self.inc_r(L),
            0x34 -> self.inc_m_hl(),

            0x3d -> self.dec_r(A),
            0x05 -> self.dec_r(B),
            0x0d -> self.dec_r(C),
            0x15 -> self.dec_r(D),
            0x1d -> self.dec_r(E),
            0x25 -> self.dec_r(H),
            0x2d -> self.dec_r(L),
            0x35 -> self.dec_m_hl(),

            0x09 -> self.add_hl_rr(self.bc, BC),
            0x19 -> self.add_hl_rr(self.de, DE),
            0x29 -> self.add_hl_rr(self.hl(), HL),
            0x39 -> self.add_hl_rr(self.sp, SP),

            0xe8 -> self.add_sp_i8(op1),

            0x03 -> self.inc_rr(BC),
            0x13 -> self.inc_rr(DE),
            0x23 -> self.inc_rr(HL),
            0x33 -> self.inc_rr(SP),

            0x0b -> self.dec_rr(BC),
            0x1b -> self.dec_rr(DE),
            0x2b -> self.dec_rr(HL),
            0x3b -> self.dec_rr(SP),

            0x27 -> self.daa(),
            0x2f -> self.cpl(),
            0x3f -> self.ccf(),
            0x37 -> self.scf(),
            0x00 -> self.nop(),
            0x76 -> self.halt(),

            0xf3 -> self.di(),
            0xfb -> self.ei(),

            0x07 -> self.rlca(),
            0x17 -> self.rla(),
            0x0f -> self.rrca(),
            0x1f -> self.rra(),

            0xc3 -> self.jp_i16(NONE, op16),

            0xc2 -> self.jp_f_i16(NZ, op16),
            0xca -> self.jp_f_i16(Z, op16),
            0xd2 -> self.jp_f_i16(NC, op16),
            0xda -> self.jp_f_i16(C, op16),
            0xe9 -> self.jp_hl(),

            0x18 -> self.jr_i8(NONE, op1),
            0x20 -> self.jr_f_i8(NZ, op1),
            0x28 -> self.jr_f_i8(Z, op1),
            0x30 -> self.jr_f_i8(NC, op1),
            0x38 -> self.jr_f_i8(C, op1),

            0xcd -> self.call_i16(NONE, op16),
            0xc4 -> self.call_f_i16(NZ, op16),
            0xcc -> self.call_f_i16(Z, op16),
            0xd4 -> self.call_f_i16(NC, op16),
            0xdc -> self.call_f_i16(C, op16),

            0xc7 -> self.rst_n(0x00),
            0xcf -> self.rst_n(0x08),
            0xd7 -> self.rst_n(0x10),
            0xdf -> self.rst_n(0x18),
            0xe7 -> self.rst_n(0x20),
            0xef -> self.rst_n(0x28),
            0xf7 -> self.rst_n(0x30),
            0xff -> self.rst_n(0x38),

            0xc9 -> self.ret(_),
            0xc0 -> self.ret_f(NZ),
            0xc8 -> self.ret_f(Z),
            0xd0 -> self.ret_f(NC),
            0xd8 -> self.ret_f(C),

            0xd9 -> self.reti(),

            0x10 -> match op1 {
                0x00 -> self.stop(),
                _ -> panic!("0x10 ???"),
            }

            0xcb -> match op1 {
                0x37 -> self.swap_r(A),
                0x30 -> self.swap_r(B),
                0x31 -> self.swap_r(C),
                0x32 -> self.swap_r(D),
                0x33 -> self.swap_r(E),
                0x34 -> self.swap_r(H),
                0x35 -> self.swap_r(L),
                0x36 -> self.swap_m_hl(),

                0x07 -> self.rlc_r(A),
                0x00 -> self.rlc_r(B),
                0x01 -> self.rlc_r(C),
                0x02 -> self.rlc_r(D),
                0x03 -> self.rlc_r(E),
                0x04 -> self.rlc_r(H),
                0x05 -> self.rlc_r(L),
                0x06 -> self.rlc_m_hl(),

                0x17 -> self.rl_r(A),
                0x10 -> self.rl_r(B),
                0x11 -> self.rl_r(C),
                0x12 -> self.rl_r(D),
                0x13 -> self.rl_r(E),
                0x14 -> self.rl_r(H),
                0x15 -> self.rl_r(L),
                0x16 -> self.rl_m_hl(),

                0x0f -> self.rrc_r(A),
                0x08 -> self.rrc_r(B),
                0x09 -> self.rrc_r(C),
                0x0a -> self.rrc_r(D),
                0x0b -> self.rrc_r(E),
                0x0c -> self.rrc_r(H),
                0x0d -> self.rrc_r(L),
                0x0e -> self.rrc_m_hl(),

                0x1f -> self.rr_r(A),
                0x18 -> self.rr_r(B),
                0x19 -> self.rr_r(C),
                0x1a -> self.rr_r(D),
                0x1b -> self.rr_r(E),
                0x1c -> self.rr_r(H),
                0x1d -> self.rr_r(L),
                0x1e -> self.rr_m_hl(),

                0x27 -> self.sla_r(A),
                0x20 -> self.sla_r(B),
                0x21 -> self.sla_r(C),
                0x22 -> self.sla_r(D),
                0x23 -> self.sla_r(E),
                0x24 -> self.sla_r(H),
                0x25 -> self.sla_r(L),
                0x26 -> self.sla_m_hl(),

                0x2f -> self.sra_r(A),
                0x28 -> self.sra_r(B),
                0x29 -> self.sra_r(C),
                0x2a -> self.sra_r(D),
                0x2b -> self.sra_r(E),
                0x2c -> self.sra_r(H),
                0x2d -> self.sra_r(L),
                0x2e -> self.sra_m_hl(),

                0x3f -> self.srl_r(A),
                0x38 -> self.srl_r(B),
                0x39 -> self.srl_r(C),
                0x3a -> self.srl_r(D),
                0x3b -> self.srl_r(E),
                0x3c -> self.srl_r(H),
                0x3d -> self.srl_r(L),
                0x3e -> self.srl_m_hl(),

                0x47 -> self.bit_b_r(0, A),
                0x40 -> self.bit_b_r(0, B),
                0x41 -> self.bit_b_r(0, C),
                0x42 -> self.bit_b_r(0, D),
                0x43 -> self.bit_b_r(0, E),
                0x44 -> self.bit_b_r(0, H),
                0x45 -> self.bit_b_r(0, L),
                0x46 -> self.bit_b_m_hl(0),
                0x4f -> self.bit_b_r(1, A),
                0x48 -> self.bit_b_r(1, B),
                0x49 -> self.bit_b_r(1, C),
                0x4a -> self.bit_b_r(1, D),
                0x4b -> self.bit_b_r(1, E),
                0x4c -> self.bit_b_r(1, H),
                0x4d -> self.bit_b_r(1, L),
                0x4e -> self.bit_b_m_hl(1),
                0x57 -> self.bit_b_r(2, A),
                0x50 -> self.bit_b_r(2, B),
                0x51 -> self.bit_b_r(2, C),
                0x52 -> self.bit_b_r(2, D),
                0x53 -> self.bit_b_r(2, E),
                0x54 -> self.bit_b_r(2, H),
                0x55 -> self.bit_b_r(2, L),
                0x56 -> self.bit_b_m_hl(2),
                0x5f -> self.bit_b_r(3, A),
                0x58 -> self.bit_b_r(3, B),
                0x59 -> self.bit_b_r(3, C),
                0x5a -> self.bit_b_r(3, D),
                0x5b -> self.bit_b_r(3, E),
                0x5c -> self.bit_b_r(3, H),
                0x5d -> self.bit_b_r(3, L),
                0x5e -> self.bit_b_m_hl(3),
                0x67 -> self.bit_b_r(4, A),
                0x60 -> self.bit_b_r(4, B),
                0x61 -> self.bit_b_r(4, C),
                0x62 -> self.bit_b_r(4, D),
                0x63 -> self.bit_b_r(4, E),
                0x64 -> self.bit_b_r(4, H),
                0x65 -> self.bit_b_r(4, L),
                0x66 -> self.bit_b_m_hl(4),
                0x6f -> self.bit_b_r(5, A),
                0x68 -> self.bit_b_r(5, B),
                0x69 -> self.bit_b_r(5, C),
                0x6a -> self.bit_b_r(5, D),
                0x6b -> self.bit_b_r(5, E),
                0x6c -> self.bit_b_r(5, H),
                0x6d -> self.bit_b_r(5, L),
                0x6e -> self.bit_b_m_hl(5),
                0x77 -> self.bit_b_r(6, A),
                0x70 -> self.bit_b_r(6, B),
                0x71 -> self.bit_b_r(6, C),
                0x72 -> self.bit_b_r(6, D),
                0x73 -> self.bit_b_r(6, E),
                0x74 -> self.bit_b_r(6, H),
                0x75 -> self.bit_b_r(6, L),
                0x76 -> self.bit_b_m_hl(6),
                0x7f -> self.bit_b_r(7, A),
                0x78 -> self.bit_b_r(7, B),
                0x79 -> self.bit_b_r(7, C),
                0x7a -> self.bit_b_r(7, D),
                0x7b -> self.bit_b_r(7, E),
                0x7c -> self.bit_b_r(7, H),
                0x7d -> self.bit_b_r(7, L),
                0x7e -> self.bit_b_m_hl(7),

                0xc7 -> self.set_b_r(0, A),
                0xc0 -> self.set_b_r(0, B),
                0xc1 -> self.set_b_r(0, C),
                0xc2 -> self.set_b_r(0, D),
                0xc3 -> self.set_b_r(0, E),
                0xc4 -> self.set_b_r(0, H),
                0xc5 -> self.set_b_r(0, L),
                0xc6 -> self.set_b_m_hl(0),
                0xcf -> self.set_b_r(1, A),
                0xc8 -> self.set_b_r(1, B),
                0xc9 -> self.set_b_r(1, C),
                0xca -> self.set_b_r(1, D),
                0xcb -> self.set_b_r(1, E),
                0xcc -> self.set_b_r(1, H),
                0xcd -> self.set_b_r(1, L),
                0xce -> self.set_b_m_hl(1),
                0xd7 -> self.set_b_r(2, A),
                0xd0 -> self.set_b_r(2, B),
                0xd1 -> self.set_b_r(2, C),
                0xd2 -> self.set_b_r(2, D),
                0xd3 -> self.set_b_r(2, E),
                0xd4 -> self.set_b_r(2, H),
                0xd5 -> self.set_b_r(2, L),
                0xd6 -> self.set_b_m_hl(2),
                0xdf -> self.set_b_r(3, A),
                0xd8 -> self.set_b_r(3, B),
                0xd9 -> self.set_b_r(3, C),
                0xda -> self.set_b_r(3, D),
                0xdb -> self.set_b_r(3, E),
                0xdc -> self.set_b_r(3, H),
                0xdd -> self.set_b_r(3, L),
                0xde -> self.set_b_m_hl(3),
                0xe7 -> self.set_b_r(4, A),
                0xe0 -> self.set_b_r(4, B),
                0xe1 -> self.set_b_r(4, C),
                0xe2 -> self.set_b_r(4, D),
                0xe3 -> self.set_b_r(4, E),
                0xe4 -> self.set_b_r(4, H),
                0xe5 -> self.set_b_r(4, L),
                0xe6 -> self.set_b_m_hl(4),
                0xef -> self.set_b_r(5, A),
                0xe8 -> self.set_b_r(5, B),
                0xe9 -> self.set_b_r(5, C),
                0xea -> self.set_b_r(5, D),
                0xeb -> self.set_b_r(5, E),
                0xec -> self.set_b_r(5, H),
                0xed -> self.set_b_r(5, L),
                0xee -> self.set_b_m_hl(5),
                0xf7 -> self.set_b_r(6, A),
                0xf0 -> self.set_b_r(6, B),
                0xf1 -> self.set_b_r(6, C),
                0xf2 -> self.set_b_r(6, D),
                0xf3 -> self.set_b_r(6, E),
                0xf4 -> self.set_b_r(6, H),
                0xf5 -> self.set_b_r(6, L),
                0xf6 -> self.set_b_m_hl(6),
                0xff -> self.set_b_r(7, A),
                0xf8 -> self.set_b_r(7, B),
                0xf9 -> self.set_b_r(7, C),
                0xfa -> self.set_b_r(7, D),
                0xfb -> self.set_b_r(7, E),
                0xfc -> self.set_b_r(7, H),
                0xfd -> self.set_b_r(7, L),
                0xfe -> self.set_b_m_hl(7),

                0x87 -> self.res_b_r(0, A),
                0x80 -> self.res_b_r(0, B),
                0x81 -> self.res_b_r(0, C),
                0x82 -> self.res_b_r(0, D),
                0x83 -> self.res_b_r(0, E),
                0x84 -> self.res_b_r(0, H),
                0x85 -> self.res_b_r(0, L),
                0x86 -> self.res_b_m_hl(0),
                0x8f -> self.res_b_r(1, A),
                0x88 -> self.res_b_r(1, B),
                0x89 -> self.res_b_r(1, C),
                0x8a -> self.res_b_r(1, D),
                0x8b -> self.res_b_r(1, E),
                0x8c -> self.res_b_r(1, H),
                0x8d -> self.res_b_r(1, L),
                0x8e -> self.res_b_m_hl(1),
                0x97 -> self.res_b_r(2, A),
                0x90 -> self.res_b_r(2, B),
                0x91 -> self.res_b_r(2, C),
                0x92 -> self.res_b_r(2, D),
                0x93 -> self.res_b_r(2, E),
                0x94 -> self.res_b_r(2, H),
                0x95 -> self.res_b_r(2, L),
                0x96 -> self.res_b_m_hl(2),
                0x9f -> self.res_b_r(3, A),
                0x98 -> self.res_b_r(3, B),
                0x99 -> self.res_b_r(3, C),
                0x9a -> self.res_b_r(3, D),
                0x9b -> self.res_b_r(3, E),
                0x9c -> self.res_b_r(3, H),
                0x9d -> self.res_b_r(3, L),
                0x9e -> self.res_b_m_hl(3),
                0xa7 -> self.res_b_r(4, A),
                0xa0 -> self.res_b_r(4, B),
                0xa1 -> self.res_b_r(4, C),
                0xa2 -> self.res_b_r(4, D),
                0xa3 -> self.res_b_r(4, E),
                0xa4 -> self.res_b_r(4, H),
                0xa5 -> self.res_b_r(4, L),
                0xa6 -> self.res_b_m_hl(4),
                0xaf -> self.res_b_r(5, A),
                0xa8 -> self.res_b_r(5, B),
                0xa9 -> self.res_b_r(5, C),
                0xaa -> self.res_b_r(5, D),
                0xab -> self.res_b_r(5, E),
                0xac -> self.res_b_r(5, H),
                0xad -> self.res_b_r(5, L),
                0xae -> self.res_b_m_hl(5),
                0xb7 -> self.res_b_r(6, A),
                0xb0 -> self.res_b_r(6, B),
                0xb1 -> self.res_b_r(6, C),
                0xb2 -> self.res_b_r(6, D),
                0xb3 -> self.res_b_r(6, E),
                0xb4 -> self.res_b_r(6, H),
                0xb5 -> self.res_b_r(6, L),
                0xb6 -> self.res_b_m_hl(6),
                0xbf -> self.res_b_r(7, A),
                0xb8 -> self.res_b_r(7, B),
                0xb9 -> self.res_b_r(7, C),
                0xba -> self.res_b_r(7, D),
                0xbb -> self.res_b_r(7, E),
                0xbc -> self.res_b_r(7, H),
                0xbd -> self.res_b_r(7, L),
                0xbe -> self.res_b_m_hl(7),

                _ -> panic!("cpu instruction 0xcb undefined"),
            },

            _ -> panic!("cpu instruction undefined"),
        }
    }

    fn.nop() {
        log!("nop");
        self.pc += 1;
        //self.cycle = 0;
    }

    fn ld_r_u8(&mut self, op: OP, u: u8)  {
        log!("LD {} {}", op, u);
        match op {
            A -> self.a = u,
            B -> self.b = u,
            C -> self.c = u,
            D -> self.d = u,
            E -> self.e = u,
            H -> self.h = u,
            L -> self.l = u,
            _ -> panic!("ld_r_u8 {} {}", op, u),
        }
        self.pc += 2;
        self.cycle = 1;
    }

    fn ld_r_r(&mut self, op: OP, r: u8, op1, OP) {
        log!("LD {} {}:{}", op, op1, r);
        match op {
            A -> self.a = r,
            B -> self.b = r,
            C -> self.c = r,
            D -> self.d = r,
            E -> self.e = r,
            H -> self.h = r,
            L -> self.l = r,
            _ ->  panic!("ld_r_r {}", r),
        }
        self.pc += 1;
        self.cycle = 1;
    }

    fn ld_r_m(&mut self, op: OP, m: u16, op1: OP) {
        log!("LD {} ({}:{:x})", op, op1, m);
        match op {
            A -> self.a = self.mbc.read(m),
            B -> self.b = self.mbc.read(m),
            C -> self.c = self.mbc.read(m),
            D -> self.d = self.mbc.read(m),
            E -> self.e = self.mbc.read(m),
            H -> self.h = self.mbc.read(m),
            L -> self.l = self.mbc.read(m),
            _ -> panic!("ld_r_m {}", op),
        }
        self.pc += 1;
        self.cycle = 2;
    }

    fn ld_m_r(&mut self, m: u16, r: u8, op: OP, op1: OP) {
        log!("LD ({}:{:x}) {}:{:x}", op, m, op1, r);
        self.mbc.write(m, r);
        self.pc += 1;
        self.cycle = 2;
    }

    fn ld_m_i16_r(m: u16, r: u8, op: OP) {
        log!("LD ({:x}) {}:{:x}", m, op, r);
        self.mbc.write(m, r);
        self.pc += 3;
        self.cycle = 2;
    }

    fn ld_m_u8(m: u16, u: u8, op: OP) {
        log!("LD ({}:{:x}) {:x}", op, m, u);
        self.m.write(m, u);
        self.pc += 2;
        self.cycle = 3;
    }

    fn ld_r_m_i16(op: OP, m: u16) {
        log!("LD {} ({:x})", op, m);
        match op {
            A -> self.a = self.mbc.read(m),
            _ -> panic!("ld_r_m_i16"),
        }
        self.pc += 3;
        self.cycle = 4;
    }

    fn ldd_a_m_hl() {
        log!("LDD A (HL:{})", self.hl());
        self.a = self.mbc.read(self.hl());
        self.set_hl(self._sub_u16(self.hl(), 1));
        self.pc += 1;
        self.cycle = 2;
    }

    fn ldd_m_hl_a() {
        log!("LDD (HL:{:x}) A:${:x}", self.hl(), self.a);
        self.mbc.write(self.hl(), self.a);
        self.set_hl(self.hl() - 1);
        self.pc += 1;
        self.cycle = 2;
    }

    fn ldi_a_m_hl() {
        log!("LDI A (HL:{})", self.hl());
        self.a = self.mbc.read(self.hl);
        self.set_hl(self._add_u16(self.hl, 1));
        self.pc += 1;
        self.cycle = 2;
    }

  fn ldi_m_hl_a() {
    log!("LDI (HL:{:x}) A:${:x}", self.hl(), self.a);
    self.mbc.write(self.hl(), self.a);
    self.set_hl(self._add_u16(self.hl(), 1));
    self.pc += 1;
    self.clock = 2;
  }

  fn ldh_m_r(m: u16, r: u8, op: OP) {
    log!("LDH ($FF00+${:x}) {}:{:x}", self.m, op, r);
    self.mbc.write(0xFF00 + m, r);
    self.pc += 2;
    self.clock = 3;
  }

  fn ldh_r_m(op: OP, m: U8) {
    log!("LDH {} ($FF00+{:x})", op, m);
    match op {
       A -> self.a = self.mbc.read(0xFF00 + m),
        _ -> panic!("ldh_r_m");
    }
    self.pc += 2;
    self.cycle = 3;
  }

  ld_rr_i16(r: Operand, u: U16) {
    this.log(`LD ${r} ${toHex(u)}`);
    switch (r) {
      case "BC":
        this.bc = u;
        break;
      case "DE":
        this.de = u;
        break;
      case "HL":
        this.hl = u;
        break;
      case "SP":
        this.sp = u;
        break;
      default:
        throw "ld_rr_i16";
    }
    this.pc += 3;
    this.clock = 12;
  }

  ld_sp_hl() {
    this.log(`LD SP HL:${toHex(this.hl)}`);
    this.sp = this.hl;
    this.pc += 1;
    this.clock = 8;
  }

  ld_hl_sp_i8(u: U8) {
    let i = toI8(u);
    this.log(`LD HL SP${showI8(i)}`);
    const offset = u << 24 >> 24;
    const tmp = this.sp + offset;
    this.carry = (this.sp & 0xff) + (offset & 0xff) > 0xff ? 1 : 0;
    this.half = (this.sp & 0xf) + (offset & 0xf) > 0xf ? 1 : 0;
    if (tmp > 0xffff) {
      this.hl = tmp - 0x10000;
    } else if (tmp < 0) {
      this.hl = tmp + 0x10000;
    } else {
      this.hl = tmp;
    }
    this.zero = 0;
    this.negative = 0;
    this.pc += 2;
    this.clock = 12;
  }

  ld_m_i16_sp(u: U16) {
    this.log(`LD (${toHex(u)}) SP:${toHex(this.sp)}`);
    this.m.write(u, this.sp & 0xff);
    this.m.write(u + 1, this.sp >> 8 & 0xff);
    this.pc += 3;
    this.clock = 20;
  }


  }
}
