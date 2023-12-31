use std::io;
use std::fs;

#[derive(Debug)]
pub struct ROMType { 
    pub mbc_type: MBCType, 
    pub ram_ex: bool, 
    pub battery: bool, 
    pub timer: bool,
}

#[derive(Debug)]
pub enum MBCType { 
    None,
    MBC1,
}

#[derive(Debug)]
pub struct ROM {
    pub title: String,
    pub manufacturer_code: Vec<u8>,
    pub cgb_flag: u8,
    pub new_licensee_code: Vec<u8>,
    pub sgb_flag: u8,
    pub rom_type: ROMType,
    pub rom_size: usize,
    pub ram_ex_size: usize,
    pub destination_code: u8,
    pub old_licensee_code: u8,
    pub mask_rom_version_number: u8,
    pub header_checksum: u8,
    pub global_checksum: u16,

    pub raw: Vec<u8>,
}

impl ROM {
    pub fn new(raw: Vec<u8>) -> ROM {
        ROM {
            //title: raw[0x134..=0x143].to_vec(),
            title: raw[0x134..=0x143].iter().map(|&b| b as char).collect(), 
            manufacturer_code: raw[0x13f..=0x142].to_vec(),
            cgb_flag: raw[0x143],
            new_licensee_code: raw[0x144..=0x145].to_vec(),
            sgb_flag: raw[0x146],
            rom_type: match raw[0x147] {
                0x00 => ROMType { mbc_type: MBCType::None, ram_ex: false, battery: false, timer: false },
                0x01 => ROMType { mbc_type: MBCType::MBC1, ram_ex: false, battery: false, timer: false },
                0x02 => ROMType { mbc_type: MBCType::MBC1, ram_ex: true, battery: false, timer: false },
                0x03 => ROMType { mbc_type: MBCType::MBC1, ram_ex: true, battery: true, timer: false },
                0x08 => ROMType { mbc_type: MBCType::None, ram_ex: true, battery: false, timer: false },
                0x09 => ROMType { mbc_type: MBCType::None, ram_ex: true, battery: true, timer: false },
                _    => ROMType { mbc_type: MBCType::None, ram_ex: false, battery: false, timer: false }
            },
            rom_size: 0x8000 * (1 << (raw[0x148] as usize)),
            ram_ex_size: match raw[0x149] {
                0x0 => 0,
                0x1 => 0,
                0x2 => 0x2000,
                0x3 => 0x8000,
                0x4 => 0x20000,
                0x5 => 0x10000,
                _ => 0
            },
            destination_code: raw[0x14a],
            old_licensee_code: raw[0x14b],
            mask_rom_version_number: raw[0x14c],
            header_checksum: raw[0x14d],
            global_checksum: (raw[0x14e] as u16) << 8 | (raw[0x14e] as u16),

            raw: raw
        }
    }

    #[inline]
    pub fn read(&self, i: usize) -> u8 {
        self.raw[i]
    }
}

pub fn read_rom(path: String) -> Result<ROM, io::Error> {
    let raw = fs::read(path)?;
    Ok(ROM::new(Vec::from(raw)))
}
