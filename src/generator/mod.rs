mod data;
mod encoder;
mod bitstream;
use bitstream::BitStream;
use data::qr_version_query;
use encoder::{alphanum_value, is_kanji, AlphanumEncoder, BytesEncoder, Encoder, KanjiEncoder, NumeralEncoder};
use std::{fs::File, io::Read, process::exit};
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug)]
pub enum ErrorCorrection {
    Low,
    Medium,
    Quartile,
    High
}

#[derive(Debug)]
pub struct Flag {
    pub data: bool,
    pub bytes: bool,
    pub min_vers: u8,
    pub ecc: ErrorCorrection
}

impl Flag {
    pub fn new() -> Self {
        Self {
            data: false,
            bytes: false,
            min_vers: 1,
            ecc: ErrorCorrection::Quartile
        }
    }
}

#[derive(Debug)]
pub struct Generator {
	text: String,
    output: String,
    size: u32,
    flag: Flag
}

impl Generator {
	pub fn new(data: String, output: String, size: u32, flag: Flag) -> Self {
		let text =
            if flag.data {
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
                    },
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
            flag
		}
	}

    fn get_data_size(mode: u8, len: usize, str: &str) -> u16 {
        match mode {
            0 => (10 * (len / 3) + (if len % 3 == 1 { 4 } else if len % 3 == 2 { 7 } else { 0 })) as u16,
            1 => (11 * (len / 2) + (if len % 2 == 1 { 6 } else { 0 })) as u16,
            2 => (8 * str.len()) as u16,
            3 => (13 * len) as u16,
            4 => 0,
            _ => panic!("Something went wrong during qr code generation: weird mode {mode}")
        }
    }
  
    //     len + match mode {
    //         0 => if qr_version_query(&self.flag.ecc, len + 12) > 26 { 14 }
    //             else if qr_version_query(&self.flag.ecc, len + 10) > 9 { 12 }
    //             else { 10 },
    //         1 => if qr_version_query(&self.flag.ecc, len + 11) > 26 { 13 }
    //             else if qr_version_query(&self.flag.ecc, len + 9) > 9 { 11 }
    //             else { 9 },
    //         2 => if qr_version_query(&self.flag.ecc, len + 8) > 10 { 16 } else { 8 },
    //         3 => if qr_version_query(&self.flag.ecc, len + 10) > 26 { 12 }
    //             else if qr_version_query(&self.flag.ecc, len + 8) > 9 { 10 }
    //             else { 8 },
    //         _ => panic!("Something went wrong during qr code generation: weird mode {mode}")
    //     }
    // };

    fn get_version(&self) -> usize {
        if self.flag.bytes {
            let length = self.text.len();
            let v1 = qr_version_query(&self.flag.ecc, (length * 8 + 12) as u16); //test for v1-10 by 8 bit char count indicator
            let v2 = qr_version_query(&self.flag.ecc, (length * 8 + 20) as u16); //test for v11-40 by 16 bit char count indicator
            
            if v1 == 41 {
                println!("Error: data is too large to be converted into a QR code. Data length: {length}.");
                exit(0);
            }
            
            self.flag.min_vers.max(v1.min(v2) as u8) as usize
        } else {
            let mut iter = self.text.chars().rev().peekable();
            if let None = iter.peek() {
                return 0;
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
                            _ => false
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
                            let total_size = ((mode_indicator_size + cost) as usize) + dp[idx + size][next_mode];

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

            let min_cost = dp[data[0].0 as usize][data[0].1 as usize] + 4;
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

            println!("\n{min_cost}\n{encoding:?}");

            1
            // self.flag.min_vers.max(qr_version_query(&self.flag.ecc, size) as u8) as usize
        }
    }

    pub fn run(self) {
        let mut stream = BitStream::new();

        let version = self.get_version();
        
        let mut chars = self.text.chars();

        if self.flag.bytes {
            BytesEncoder::encode(&mut chars, self.text.len(), &mut stream, version);
            // println!("\nlength: {}; version: {version}", self.text.len());
            // stream.debug_print();
        } else {
            // KanjiEncoder::encode(&mut chars, self.text.chars().count(), &mut stream, version);
            println!("\nlength: {}; version: {version}", self.text.len());
            // stream.debug_print();
        }
    }
}