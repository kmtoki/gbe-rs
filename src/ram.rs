
#[derive(Debug)]
pub struct RAM {
    pub ram: Vec<u8>,
    pub ram_ex: Vec<u8>,
}

#[derive(Debug, Copy, Clone)]
pub enum Reg {
    JOYP = 0xff00,

    SB = 0xff01,
    SC = 0xff02,

    DIV = 0xff04,
    TIMA = 0xff05,
    TMA = 0xff06,
    TAC = 0xff07,

    NR10 = 0xff10,
    NR11 = 0xff11,
    NR12 = 0xff12,
    NR13 = 0xff13,
    NR14 = 0xff14,
    NR21 = 0xff16,
    NR22 = 0xff17,
    NR23 = 0xff18,
    NR24 = 0xff19,
    NR30 = 0xff1a,
    NR31 = 0xff1b,
    NR32 = 0xff1c,
    NR33 = 0xff1d,
    NR34 = 0xff1e,
    NR41 = 0xff20,
    NR42 = 0xff21,
    NR43 = 0xff22,
    NR44 = 0xff23,
    NR50 = 0xff24,
    NR51 = 0xff25,
    NR52 = 0xff26,
    WPR = 0xff30,

    LCDC = 0xff40,
    STAT = 0xff41,
    SCY = 0xff42,
    SCX = 0xff43,
    LY = 0xff44,
    LYC = 0xff45,
    WY = 0xff4a,
    WX = 0xff4b,
    BGP = 0xff47,
    OBP0 = 0xff48,
    OBP1 = 0xff49,
    BCPS = 0xff68,
    BCPD = 0xff69,
    OCPS = 0xff6a,
    DMA = 0xff46,
    VBK = 0xff4f,
    HDMA1 = 0xff51,
    HDMA2 = 0xff52,
    HDMA3 = 0xff53,
    HDMA4 = 0xff54,
    HDMA5 = 0xff55,

    IF = 0xff0f,
    IE = 0xffff,
}

impl RAM {
    pub fn new(ram_ex_size: usize) -> Self {
        RAM {
            ram: vec![0; 0x10000],
            ram_ex: vec![0; ram_ex_size],
        }
    }

    #[inline]
    pub fn read(&self, i: usize) -> u8 {
        self.ram[i]
    }

    #[inline]
    pub fn read_reg(&self, r: Reg) -> u8 {
        self.ram[r as usize]
    }

    #[inline]
    pub fn read_ex(&self, i: usize) -> u8 {
        self.ram_ex[i]
    }

    #[inline]
    pub fn write(&mut self, i: usize, v: u8) {
        self.ram[i] = v;
    }

    #[inline]
    pub fn write_reg(&mut self, r: Reg, v: u8) {
        self.ram[r as usize] = v;
    }

    #[inline]
    pub fn write_ex(&mut self, i: usize, v: u8) {
        self.ram_ex[i] = v;
    }

    #[inline]
    pub fn modify_reg(&mut self, r: Reg, f: fn(u8) -> u8) {
        let u = self.read_reg(r);
        self.write_reg(r, f(u));
    }

    #[inline]
    pub fn transfer_dma(&mut self, dma: usize) {
        for i in 0..0x100 {
            self.write(0xfe00 + i, self.read((dma << 8) + i));
        }
    }
}
