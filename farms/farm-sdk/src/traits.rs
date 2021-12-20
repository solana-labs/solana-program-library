use crate::string::ArrayString64;

pub trait Named {
    fn name(&self) -> ArrayString64;
}

pub trait Versioned {
    fn version(&self) -> u16;
}
