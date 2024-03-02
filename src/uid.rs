// has nothing to do with inode
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Uid(u128);

impl Uid {
    pub const BASE: Self = Uid(0);
    pub const ROOT: Self = Uid(1);
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

    pub fn message() -> Self {
        Uid(rand::random::<u128>() & !(0xf << 124) | (0x3 << 124))
    }

    pub fn is_special(&self) -> bool {
        (self.0 >> 124) != 0
    }

    pub fn debug_info(&self) -> String {
        if self.is_special() {
            if self.0 >> 124 == 0x1 {
                format!("Uid::error({})", self.0 & !(0xf << 124))
            }

            else if self.0 >> 124 == 0x2 {
                format!("Uid::truncated_rows({})", self.0 & !(0xf << 124))
            }

            else if self.0 >> 124 == 0x3 {
                format!("Uid::message({})", self.0 & !(0xf << 124))
            }

            else {
                unreachable!()
            }
        }

        else {
            format!("Uid::normal_file({})", self.0)
        }
    }
}
