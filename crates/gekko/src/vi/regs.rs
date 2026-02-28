use crate::mmio::{Mmio, traits::MmioRegister};
use chapa::BitEnum;

#[derive(Debug, BitEnum)]
pub enum VideoFormat {
    Ntsc = 0,
    Pal = 1,
    Mpal = 2,
    Debug = 3,
}

#[rustfmt::skip]
#[chapa::bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug)]
pub struct DisplayConfiguration {
    #[bits(0, alias = "enb")] pub enable: bool,
    #[bits(1, alias = "rst")] pub reset: bool,
    #[bits(2, alias = "nin")] pub interlace_selector: bool,
    #[bits(3, alias = "dlr")] pub display_mode_3d: bool,
    #[bits(4..=5, alias = "le0")] pub display_latch0: u8,
    #[bits(6..=7, alias = "le1")] pub display_latch1: u8,
    #[bits(8..=9, alias = "fmt")] pub video_format: VideoFormat,
}

#[rustfmt::skip]
impl MmioRegister for DisplayConfiguration {
    const ADDR: u32 = Mmio::virt_to_phys(0xCC002002);
    const SIZE: usize = 2;
    fn from_raw(raw: u32) -> Self { (raw as u16).into() }
    fn to_raw(self) -> u32 { self.raw() as u32 }
}

#[chapa::bitfield(u32, order = lsb0)]
#[derive(Copy, Clone, Debug)]
pub struct TopFieldBase {
    #[bits(9..=23, alias = "fbb")]
    pub xfb_addr: u32,
    
    #[bits(24..=27, alias = "xof")]
    pub horizontal_offset: u8,
    
    #[bits(28)]
    pub page_offset: bool,

    // TODO: 29-31	y	always zero (maybe some write only control register stuff?, setting bit 31 clears bits 31-28 (?))
}

#[chapa::bitfield(u32, order = lsb0)]
#[derive(Copy, Clone, Debug)]
pub struct BottomFieldBase {
    #[bits(9..=23, alias = "fbb")]
    pub xfb_addr: u32,

    #[bits(28)]
    pub page_offset: bool,

    // TODO:  	y	always zero (maybe some write-only control register stuff?)
}

#[rustfmt::skip]
impl MmioRegister for TopFieldBase {
    const ADDR: u32 = Mmio::virt_to_phys(0xCC00201c);
    const SIZE: usize = 4;
    fn from_raw(raw: u32) -> Self { raw.into() }
    fn to_raw(self) -> u32 { self.raw() }
}

#[rustfmt::skip]
impl MmioRegister for BottomFieldBase {
    const ADDR: u32 = Mmio::virt_to_phys(0xCC002024);
    const SIZE: usize = 4;
    fn from_raw(raw: u32) -> Self { raw.into() }
    fn to_raw(self) -> u32 { self.raw() }
}