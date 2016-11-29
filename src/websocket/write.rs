use tk_bufstream::Buf;


pub trait WriteExt {
    fn write_packet(&mut self, opcode: u8, data: &[u8]);
    fn write_close(&mut self, code: u16, reason: &str);
}

impl WriteExt for Buf {
    fn write_packet(&mut self, opcode: u8, data: &[u8]) {
        debug_assert!(opcode & 0xF0 == 0);
        let first_byte = opcode | 0x80;  // always fin
        match data.len() {
            len @ 0...125 => {
                self.extend(&[first_byte, len as u8]);
            }
            len @ 126...65535 => {
                self.extend(&[first_byte, 126,
                    (len >> 8) as u8, (len & 0xFF) as u8]);
            }
            len => {
                self.extend(&[first_byte, 127,
                    ((len >> 56) & 0xFF) as u8,
                    ((len >> 48) & 0xFF) as u8,
                    ((len >> 40) & 0xFF) as u8,
                    ((len >> 32) & 0xFF) as u8,
                    ((len >> 24) & 0xFF) as u8,
                    ((len >> 16) & 0xFF) as u8,
                    ((len >> 8) & 0xFF) as u8,
                    (len & 0xFF) as u8]);
            }
        }
        self.extend(data);
    }
    fn write_close(&mut self, code: u16, reason: &str) {
        let data = reason.as_bytes();
        assert!(data.len() <= 123);
        self.extend(&[0x88, (data.len() + 2) as u8,
                      (code >> 8) as u8, (code & 0xFF) as u8]);
        self.extend(data);
    }
}
