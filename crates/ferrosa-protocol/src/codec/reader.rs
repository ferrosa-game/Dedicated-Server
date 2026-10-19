use crate::codec::CodecError;

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
    bit_pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0, bit_pos: 0 }
    }

    #[inline]
    fn align(&mut self) {
        if self.bit_pos != 0 {
            self.pos += 1;
            self.bit_pos = 0;
        }
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], CodecError> {
        self.align();
        let end = self.pos.checked_add(n).ok_or(CodecError::EoF)?;

        if end > self.buf.len() {
            return Err(CodecError::EoF);
        }

        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub fn bits(&mut self, count: u32) -> Result<u32, CodecError> {
        if count == 0 {
            return Ok(0);
        }

        let abs = self.pos * 8 + self.bit_pos;

        if abs + count as usize > self.buf.len() * 8 {
            return Err(CodecError::EoF);
        }

        let mut read = 0u32;
        let mut remaining = count;

        let first = remaining.min(8 - self.bit_pos as u32);
        let mut data = (self.buf[self.pos] as u32 >> self.bit_pos) & ((1u32 << first) - 1);
        remaining -= first;
        read += first;
        self.bit_pos += first as usize;

        if self.bit_pos >= 8 {
            self.bit_pos = 0;
            self.pos += 1;
        }

        while remaining >= 8 {
            data |= (self.buf[self.pos] as u32) << read;
            remaining -= 8;
            read += 8;
            self.pos += 1;
        }

        if remaining > 0 {
            data |= ((self.buf[self.pos] as u32) & ((1u32 << remaining) - 1)) << read;
            self.bit_pos = remaining as usize;
        }

        Ok(data)
    }

    #[inline]
    fn take<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        Ok(self.bytes(N)?.try_into().unwrap())
    }

    pub fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take::<1>()?[0])
    }
    pub fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.take::<2>()?))
    }
    pub fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.take::<4>()?))
    }
    pub fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(self.take::<8>()?))
    }
    pub fn f32(&mut self) -> Result<f32, CodecError> {
        Ok(f32::from_le_bytes(self.take::<4>()?))
    }

    pub fn fixed_float(&mut self) -> Result<f32, CodecError> {
        let raw = self.bits(24)?;
        let sign = 1u32 << 23;
        let v = (((raw & (sign - 1)) as f32 as f64) / ((sign as f32 as f64) / std::f64::consts::TAU)) as f32;
        Ok(if raw & sign == sign { -v } else { v })
    }

    pub fn i32(&mut self) -> Result<i32, CodecError> {
        Ok(i32::from_le_bytes(self.take::<4>()?))
    }

    pub fn vec3(&mut self) -> Result<glam::Vec3, CodecError> {
        Ok(glam::Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }

    /// packet_read_bits_nullable: `count` bits, all of them set meaning -1.
    pub fn nullable(&mut self, count: u32) -> Result<i32, CodecError> {
        let v = self.bits(count)?;
        Ok(if v as u64 == (1u64 << count) - 1 { -1 } else { v as i32 })
    }

    /// read_packet_bits_nullable: `count` bits, the top one a sign extended over the rest.
    pub fn signed(&mut self, count: u32) -> Result<i32, CodecError> {
        let v = self.bits(count)?;
        Ok(if v & (1 << (count - 1)) != 0 { (v | u32::MAX.checked_shl(count).unwrap_or(0)) as i32 } else { v as i32 })
    }

    /// read_delta_from_packet: 0 then a 4-bit change from `previous`, 10 then a 9-bit change, 11 then `count` bits.
    pub fn delta(&mut self, previous: i32, count: u32) -> Result<i32, CodecError> {
        Ok(if self.bits(1)? == 0 {
            previous.wrapping_add(self.signed(4)?)
        } else if self.bits(1)? == 0 {
            previous.wrapping_add(self.signed(9)?)
        } else {
            self.bits(count)? as i32
        })
    }

    /// read_delta_from_packet_bits: (with `keep_flag`, a 0 first keeps `previous`) 0 then a change from `previous` in
    /// `small` bits, 10 one in `large` bits, 11 a signed value in `count` bits.
    pub fn delta_bits(&mut self, previous: i32, count: u32, small: u32, large: u32, keep_flag: bool) -> Result<i32, CodecError> {
        if keep_flag && self.bits(1)? == 0 {
            return Ok(previous);
        }

        Ok(if self.bits(1)? == 0 {
            previous.wrapping_add(self.signed(small)?)
        } else if self.bits(1)? == 0 {
            previous.wrapping_add(self.signed(large)?)
        } else {
            self.signed(count)?
        })
    }

    /// packet_read_string: an angle in `count` bits, the top one its sign and the rest a fraction of a turn.
    pub fn angle(&mut self, count: u32) -> Result<f32, CodecError> {
        let v = self.bits(count)?;
        let sign = 1u32 << count >> 1;
        let a = ((((sign - 1) & v) as f32) as f64 / ((sign as f32) as f64 / std::f64::consts::TAU)) as f32;
        Ok(if v & sign == sign { -a } else { a })
    }

    /// Bytes up to the first zero, as text.
    pub fn text(&mut self, len: usize) -> Result<Vec<u8>, CodecError> {
        let b = self.bytes(len)?;
        Ok(b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())].to_vec())
    }

    pub fn string(&mut self, len: usize) -> Result<String, CodecError> {
        Ok(String::from_utf8(self.bytes(len)?.into()).unwrap_or("".to_string()).split("\0").collect::<Vec<&str>>().first().unwrap().to_string())
    }
}
