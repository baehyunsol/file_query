// has nothing to do with inode
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Uid(u128);

impl Uid {
    pub const BASE: Self = Uid(0);
}

impl Uid {
    pub fn normal_file() -> Self {
        Uid(rand::random::<u128>() & !(0xf << 124))
    }

    pub fn error() -> Self {
        Uid(rand::random::<u128>() & !(0xf << 124) | (0x1 << 124))
    }

    pub fn message_for_truncated_rows(n: usize) -> Self {
        Uid((0x2 << 124) | n as u128)
    }

    pub fn is_special(&self) -> bool {
        (self.0 >> 124) != 0
    }
}
