use crc::crc32;
use failure::{Error, Fail};
use png::{Decoded, DecodingError, StreamingDecoder};
use std::fs;
use std::io::prelude::*;
use std::mem;
use std::path::Path;

#[derive(Debug, Fail)]
pub enum ReadFileError {
    #[fail(display = "failed to open file: {}", err)]
    ParseError { err: DecodingError },
    #[fail(display = "this file has no incorrect crc value")]
    CorrectCrc,
}

#[derive(Default)]
pub struct PngFile {
    raw_data: Vec<u8>,
    crc_data: CrcData,
    crc_val: u32,
    offset: usize,
}

impl PngFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut ret: PngFile = Default::default();
        ret.raw_data = fs::read(path)?;
        let (crc_val, crc_data) = CrcData::from_raw(&ret.raw_data)?;
        ret.crc_data = crc_data;
        ret.crc_val = crc_val;
        ret.offset = 12;
        Ok(ret)
    }

    pub fn try_fix(&mut self) -> Option<(u32, u32)> {
        let ret = self.crc_data.try_fix(self.crc_val);
        if ret.is_some() {
            let new_crc_data = self.crc_data.get_bytes();
            self.raw_data[self.offset..(self.offset + new_crc_data.len())]
                .clone_from_slice(&new_crc_data);
        }
        ret
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let mut file = fs::File::create(path)?;
        file.write_all(&self.raw_data)?;
        Ok(())
    }
}

/// 储存用来校验 crc 值的数据和 crc 值本身
#[derive(Debug, Default)]
struct CrcData {
    pub type_str: [u8; 4],
    pub width: u32,
    pub height: u32,
    pub bits: u8,
    pub color_type: u8,
    pub compr_method: u8,
    pub filter_method: u8,
    pub interlace_method: u8,
}

impl CrcData {
    pub fn from_raw(data: &[u8]) -> Result<(u32, Self), ReadFileError> {
        let mut crcdata: CrcData = Default::default();
        let mut decoder = StreamingDecoder::new();
        let mut idx = 0;

        for _ in 0..3 {
            let (len, decoded) = match decoder.update(&data[idx..]) {
                Ok(t) => t,
                Err(DecodingError::CrcMismatch { crc_val, .. }) => {
                    return Ok((crc_val, crcdata));
                }
                Err(e) => return Err(ReadFileError::ParseError { err: e }),
            };

            match decoded {
                Decoded::ChunkBegin(_length, type_str) => {
                    crcdata.type_str.clone_from_slice(&type_str);
                }
                Decoded::Header(width, height, bit_depth, color_type, interlaced) => {
                    crcdata.width = width;
                    crcdata.height = height;
                    crcdata.bits = bit_depth as u8;
                    crcdata.color_type = color_type as u8;
                    crcdata.interlace_method = interlaced as u8;
                }
                _ => (),
            }
            idx += len;
        }
        Err(ReadFileError::CorrectCrc)
    }

    /// 将 CrcData 转化为字节数组
    #[inline]
    pub fn get_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let bwidth: [u8; 4] = unsafe { mem::transmute(self.width) };
        let bheight: [u8; 4] = unsafe { mem::transmute(self.height) };

        bytes.extend(self.type_str.iter());
        bytes.extend(bwidth.iter().rev());
        bytes.extend(bheight.iter().rev());
        bytes.extend(
            [
                self.bits,
                self.color_type,
                self.compr_method,
                self.filter_method,
                self.interlace_method,
            ]
            .iter(),
        );
        bytes
    }

    /// 爆破 crc32 值
    pub fn try_fix(&mut self, crc_val: u32) -> Option<(u32, u32)> {
        let width = self.width;

        for i in 1..8192 {
            self.width = i;
            if crc_val == crc32::checksum_ieee(&self.get_bytes()) {
                return Some((self.width, self.height));
            }
        }
        self.width = width;
        for i in 1..8192 {
            self.height = i;
            if crc_val == crc32::checksum_ieee(&self.get_bytes()) {
                return Some((self.width, self.height));
            }
        }
        None
    }
}
