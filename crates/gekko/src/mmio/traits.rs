pub trait MmioRegister: Sized {
    const ADDR: u32;
    const SIZE: usize;

    fn from_raw(raw: u32) -> Self;
    fn to_raw(self) -> u32;
}