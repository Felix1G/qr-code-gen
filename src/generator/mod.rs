mod bitstream;
mod data;
mod ecc;
mod encoder;
use bitstream::BitStream;
use data::{qr_version_query, BlockDivision};
use ecc::ErrorCorrection;
use encoder::{
    alphanum_value, is_kanji, AlphanumEncoder, BytesEncoder, Encoder, KanjiEncoder, NumeralEncoder,
};
use encoding_rs::WINDOWS_1252;
use std::{fs::File, io::Read, process::exit};

#[derive(Debug)]
pub enum ECCLevel {
    Low,
    Medium,
    Quartile,
    High,
}

#[derive(Debug)]
pub struct Flag {
    pub data: bool,
    pub bytes: bool,
    pub min_vers: u8,
    pub ecc: ECCLevel,
}

impl Flag {
    pub fn new() -> Self {
        Self {
            data: false,
            bytes: false,
            min_vers: 1,
            ecc: ECCLevel::Quartile,
        }
    }
}

#[derive(Debug)]
pub struct Generator {
    text: String,
    output: String,
    size: u32,
    flag: Flag,
}

impl Generator {
    pub fn new(data: String, output: String, size: u32, flag: Flag) -> Self {
        let text = if flag.data {
            match File::open(data) {
                Ok(mut file) => {
                    let mut buf = String::new();
                    match file.read_to_string(&mut buf) {
                        Ok(_) => buf,
                        Err(e) => {
                            println!("Error while reading file. {}", e);
                            exit(0);
                        }
                    }
                }
                Err(e) => {
                    println!("Error while reading file. {}", e);
                    exit(0);
                }
            }
        } else {
            data
        };

        Self {
            text,
            output,
            size,
            flag,
        }
    }

    fn get_data_size(mode: u8, len: usize, str: &str) -> u16 {
        match mode {
            0 => {
                (10 * (len / 3)
                    + (if len % 3 == 1 {
                        4
                    } else if len % 3 == 2 {
                        7
                    } else {
                        0
                    })) as u16
            }
            1 => (11 * (len / 2) + (if len % 2 == 1 { 6 } else { 0 })) as u16,
            2 => (8 * str.len()) as u16,
            3 => (13 * len) as u16,
            4 => 0,
            _ => panic!("Something went wrong during qr code generation: weird mode {mode}"),
        }
    }

    fn get_version(&self) -> (u8, Vec<(u16, u8)>, bool) {
        if self.flag.bytes {
            let length = self.text.len();
            let v1 = qr_version_query(&self.flag.ecc, length * 8 + 12); //test for v1-10 by 8 bit char count indicator
            let v2 = qr_version_query(&self.flag.ecc, length * 8 + 20); //test for v11-40 by 16 bit char count indicator

            (self.flag.min_vers.max(v1.min(v2)), Vec::new(), false)
        } else {
            let mut iter = self.text.chars().rev().peekable();
            if let None = iter.peek() {
                return (0, Vec::new(), false);
            }

            let len = iter.clone().count();
            let mut dp = vec![vec![u16::MAX as usize; 4]; len + 1];
            let mut next = vec![vec![None; 4]; len + 1];

            for mode in 0..=3 {
                dp[len][mode] = 0;
            }

            let mut buffer = String::new();

            for idx in (0..len).rev() {
                buffer.push(iter.next().unwrap());
                for mode in 0usize..=3 {
                    let mut max_size = 0;
                    let mut check_iter = buffer.chars().rev();
                    let mut str = String::new();
                    while let Some(ch) = check_iter.next() {
                        if match mode {
                            0 => ch.is_numeric(),
                            1 => alphanum_value(ch).is_some(),
                            2 => true,
                            3 => is_kanji(ch),
                            _ => false,
                        } {
                            str.push(ch);
                            max_size += 1;
                        } else {
                            break;
                        }
                    }

                    if str.is_empty() {
                        continue;
                    }

                    let mut str_iter = str.chars();
                    let mut str_div = String::new();

                    for size in 1..=max_size {
                        str_div.push(str_iter.next().unwrap());
                        let cost = Self::get_data_size(mode as u8, size, &str_div);
                        for next_mode in 0usize..=3 {
                            let mode_indicator_size = if mode == next_mode { 0 } else { 4 };
                            let total_size =
                                ((mode_indicator_size + cost) as usize) + dp[idx + size][next_mode];

                            if total_size < dp[idx][mode] {
                                dp[idx][mode] = total_size;
                                next[idx][mode] = Some((idx + size, next_mode));
                            }
                        }
                    }
                }
            }

            let mut data = Vec::<(u16, u8)>::new();
            let mut pos = 0;
            let mut mode = (0..=3).min_by_key(|&x| dp[0][x]).unwrap();

            while pos < len {
                data.push((pos as u16, mode as u8));
                if let Some((idx, next_mode)) = next[pos][mode] {
                    pos = idx;
                    mode = next_mode;
                } else {
                    break;
                }
            }

            let mut min_cost = dp[data[0].0 as usize][data[0].1 as usize] + 4;
            let mut encoding = Vec::<(u16, u8)>::new();
            let mut prev_mode = data[0].1;
            let mut prev_pos = data[0].0;
            for (pos, mode) in data {
                if mode == prev_mode {
                    continue;
                }

                encoding.push((pos - prev_pos, prev_mode));
                prev_mode = mode;
                prev_pos = pos;
            }
            encoding.push((len as u16 - prev_pos, prev_mode));

            let mut mode_count: Vec<usize> = vec![0, 0, 0, 0];
            let mut chars_using_byte = String::new();
            let mut text_iter = self.text.chars();
            for (len, mode) in &encoding {
                mode_count[*mode as usize] += 1;
                for _ in 0..*len {
                    if *mode == 2 {
                        chars_using_byte.push(text_iter.next().unwrap());
                    }
                }
            }
            let (_, _, add_eci_utf8) = WINDOWS_1252.encode(chars_using_byte.as_str());
            if add_eci_utf8 {
                min_cost += 12; //eci header
            }

            let v0 = qr_version_query(
                &self.flag.ecc,
                min_cost
                    + mode_count[0] * 14
                    + mode_count[1] * 13
                    + mode_count[2] * 16
                    + mode_count[3] * 12,
            );
            (
                self.flag.min_vers.max(if v0 <= 26 {
                    let v1 = qr_version_query(
                        &self.flag.ecc,
                        min_cost
                            + mode_count[0] * 12
                            + mode_count[1] * 11
                            + mode_count[2] * 16
                            + mode_count[3] * 10,
                    );
                    if v1 <= 9 {
                        qr_version_query(
                            &self.flag.ecc,
                            min_cost
                                + mode_count[0] * 10
                                + mode_count[1] * 9
                                + mode_count[2] * 8
                                + mode_count[3] * 8,
                        )
                    } else {
                        v1
                    }
                } else {
                    v0
                }) as u8,
                encoding,
                add_eci_utf8,
            )
        }
    }

    fn combine_data_err(data: Vec<(usize, Vec<u8>)>, err: Vec<Vec<u8>>) -> Vec<u8> {
        let mut res = Vec::new();

        let mut max_len = data.last().unwrap().1.len();
        let mut idx = 0;
        while idx < max_len {
            for (_, code) in &data {
                if idx < code.len() {
                    res.push(code[idx]);
                }
            }
            idx += 1;
        }

        max_len = err.last().unwrap().len();
        idx = 0;
        while idx < max_len {
            for code in &err {
                if idx < code.len() {
                    res.push(code[idx]);
                }
            }
            idx += 1;
        }

        res
    }

    pub fn run(self) {
        if self.text.chars().count() > 7100 {
            //should be 7089, but 7100 just for safety
            panic!("Error: number of characters cannot fit a QR code.")
        }

        let mut stream = BitStream::new();

        let (version, encoding, add_eci_utf8) = self.get_version();
        if version == 0 || version > 40 {
            panic!(
                "Error: {}",
                if version > 40 {
                    "number of characters cannot fit a QR code. Consider choosing a lower error correction level."
                } else {
                    "no characters found."
                }
            )
        }

        let mut chars = self.text.chars();

        if self.flag.bytes {
            BytesEncoder::encode(&mut chars, self.text.chars().count(), &mut stream, version);
        // println!("\nlength: {}; version: {version}", self.text.len());
        // stream.debug_print();
        } else {
            if add_eci_utf8 {
                stream.push_bits_big(0b011100011010usize, 12);
            }

            for (len, mode) in encoding {
                match mode {
                    0 => NumeralEncoder::encode(&mut chars, len as usize, &mut stream, version),
                    1 => AlphanumEncoder::encode(&mut chars, len as usize, &mut stream, version),
                    2 => BytesEncoder::encode(&mut chars, len as usize, &mut stream, version),
                    3 => KanjiEncoder::encode(&mut chars, len as usize, &mut stream, version),
                    _ => panic!(
                        "Error occurred during qr code generation parsing: weird mode {mode}"
                    ),
                }
            }

            // println!("\nversion: {version}");
            // stream.debug_print();
        }

        // let (data, size) = stream.consume();
        // for byte in data {
        //     print!("{byte:02X} ");
        // }
        // println!("\nlength: {size} = 8 x {} + {}; version: {version}", size / 8, size % 8);

        //obtain the data blocks
        let (mut blocks, mut blocks_num) = BlockDivision::new().consume(version, self.flag.ecc);
        blocks.reverse();
        blocks_num.reverse();
        let (data, _) = stream.consume();
        let data_size = data.len();
        let mut idx = 0;
        let mut data_codewords = Vec::new();
        while !blocks.is_empty() {
            let last_idx = blocks_num.last_mut().unwrap();
            *last_idx -= 1;

            if idx < data_size {
                let (total_len, data_len, _) = *blocks.last().unwrap();
                let mut data_vec = Vec::new();
                let old_idx = idx;
                while idx < data_size && idx - old_idx < data_len {
                    data_vec.push(data[idx]);
                    idx += 1;
                }

                while data_vec.len() + 2 < data_len {
                    data_vec.push(0xEC);
                    data_vec.push(0x11);
                }

                if data_vec.len() + 2 != data_len {
                    data_vec.push(0xEC);
                }

                data_codewords.push((total_len - data_len, data_vec));
            } else {
                let (total_len, data_len, _) = *blocks.last().unwrap();
                let mut data_vec = Vec::new();

                while data_vec.len() + 2 < data_len {
                    data_vec.push(0xEC);
                    data_vec.push(0x11);
                }

                if data_vec.len() + 2 != data_len {
                    data_vec.push(0xEC);
                }

                data_codewords.push((total_len - data_len, data_vec));
            }

            if *last_idx == 0 {
                blocks.pop();
                blocks_num.pop();
            }
        }

        //obtain the error correction blocks
        let err = ErrorCorrection::new();
        let mut error_codewords = Vec::new();
        for (err_len, code_vec) in &data_codewords {
            error_codewords.push(err.calculate(code_vec, *err_len, *err_len));
        }

        let qr_code_data = Self::combine_data_err(data_codewords, error_codewords);
    }
}
