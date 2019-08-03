use std::u32;

pub struct NetworkPktGenerator {
    cnt: u32,
}

impl NetworkPktGenerator {
    pub fn new() -> Self {
        Self { cnt: 0 }
    }

    pub fn wrap_in_pkt(&mut self, buf: &mut Vec<u8>) {
        self.cnt = self.cnt.overflowing_add(1).0;
        buf.splice(0..0, self.cnt.to_be_bytes().iter().cloned());
    }
}
