use encoding_rs::SHIFT_JIS;

use super::bitstream::BitStream;

//[mode indicator] [char count indicator] [encoding bytes] [preferred but optional: 4 times 0s terminator]
pub trait Encoder {
    //returns free bits in the current byte
    fn encode(text: &mut std::str::Chars, length: usize, bytes: &mut BitStream, version: u8);
}

pub struct NumeralEncoder;
pub struct AlphanumEncoder;
pub struct BytesEncoder;
pub struct KanjiEncoder;

impl Encoder for NumeralEncoder {
    fn encode(text: &mut std::str::Chars, length: usize, bytes: &mut BitStream, version: u8) {
        bytes.push_bits(0b0001, 4);

        if version <= 9 {
            bytes.push_bits_big(length, 10);
        } else if version <= 26 {
            bytes.push_bits_big(length, 12);
        } else {
            bytes.push_bits_big(length, 14);
        }

        let mut start_idx = 0;
        while start_idx < length {
            let s = start_idx;
            let e = length.min(start_idx + 3);
            let mut text_parse = String::new();
            for _ in s..e {
                text_parse.push(text.next().unwrap_or('\0'));
            }
            
            match text_parse.parse::<usize>() {
                Ok(x) => {
                    match e - s {
                        1 => bytes.push_bits_big(x, 4),
                        2 => bytes.push_bits_big(x, 7),
                        3 => bytes.push_bits_big(x, 10),
                        _ => {}
                    }
                }
                Err(e) => {
                    panic!("Error while parsing text numeral. {e}");
                }
            }

            start_idx += 3;
        }
    }
}

pub fn is_kanji(ch: char) -> bool {
    let mut buffer = [0; 4];
    let char_str = ch.encode_utf8(&mut buffer);

    // Encode the char as SHIFT_JIS.  If encoding succeeds without error, it's encodable.
    let (encoded, _, err) = SHIFT_JIS.encode(&char_str);

    !err && !encoded.is_empty()
}

pub const fn alphanum_value(c: char) -> Option<u8> {
    match c {
        '0' => Some(0),
        '1' => Some(1),
        '2' => Some(2),
        '3' => Some(3),
        '4' => Some(4),
        '5' => Some(5),
        '6' => Some(6),
        '7' => Some(7),
        '8' => Some(8),
        '9' => Some(9),
        'A' => Some(10),
        'B' => Some(11),
        'C' => Some(12),
        'D' => Some(13),
        'E' => Some(14),
        'F' => Some(15),
        'G' => Some(16),
        'H' => Some(17),
        'I' => Some(18),
        'J' => Some(19),
        'K' => Some(20),
        'L' => Some(21),
        'M' => Some(22),
        'N' => Some(23),
        'O' => Some(24),
        'P' => Some(25),
        'Q' => Some(26),
        'R' => Some(27),
        'S' => Some(28),
        'T' => Some(29),
        'U' => Some(30),
        'V' => Some(31),
        'W' => Some(32),
        'X' => Some(33),
        'Y' => Some(34),
        'Z' => Some(35),
        ' ' => Some(36),
        '$' => Some(37),
        '%' => Some(38),
        '*' => Some(39),
        '+' => Some(40),
        '-' => Some(41),
        '.' => Some(42),
        '/' => Some(43),
        ':' => Some(44),
        _ => None,
    }
}

impl Encoder for AlphanumEncoder {    
    fn encode(text: &mut std::str::Chars, length: usize, bytes: &mut BitStream, version: u8) {
        bytes.push_bits(0b0010, 4);

        if version <= 9 {
            bytes.push_bits_big(length, 9);
        } else if version <= 26 {
            bytes.push_bits_big(length, 11);
        } else {
            bytes.push_bits_big(length, 13);
        }

        let codes = length / 2;
        for _ in 0..codes {
            let c1 = alphanum_value(text.next().unwrap_or('\0') as char).unwrap_or(0) as usize;
            let c2 = alphanum_value(text.next().unwrap_or('\0') as char).unwrap_or(0) as usize;

            bytes.push_bits_big(c1 * 45 + c2, 11);
        }

        if length % 2 == 1 {
            bytes.push_bits_big(alphanum_value(text.next().unwrap_or('\0') as char).unwrap_or(0) as usize, 6);
        }
    }
}

impl Encoder for BytesEncoder {
    fn encode(text: &mut std::str::Chars, length: usize, bytes: &mut BitStream, version: u8) {
        bytes.push_bits(0b0100, 4);

        //TODO: divide byte into the eci headers

        let mut str = String::new();
        for _ in 0..length {
            str.push(text.next().unwrap());
        }
        let len = str.len();
        let mut str_iter = str.chars();
        
        if version <= 10 {
            bytes.push(len as u8);
        } else {
            bytes.push((len >> 8) as u8);
            bytes.push((len & 0xFF) as u8);
        }
        
        for _ in 0..length {
            let ch = str_iter.next().unwrap_or('\0');
            let mut text_bytes = [0u8; 4];
            ch.encode_utf8(&mut text_bytes);
            let val = ((text_bytes[3] as usize) << 24) |
                            ((text_bytes[2] as usize) << 16) |
                            ((text_bytes[1] as usize) << 8) |
                            text_bytes[0] as usize;
            bytes.push_bits_big(val, (ch.len_utf8() * 8) as u8);
        }
    }
}

impl Encoder for KanjiEncoder {
    fn encode(text: &mut std::str::Chars, length: usize, bytes: &mut BitStream, version: u8) {
        bytes.push_bits(0b1000, 4);
        
        if version <= 9 {
            bytes.push_bits_big(length, 8);
        } else if version <= 26 {
            bytes.push_bits_big(length, 10);
        } else {
            bytes.push_bits_big(length, 12);
        }

        let mut str = String::new();
        for _ in 0..length {
            str.push(text.next().unwrap());
        }
        let (encoded, _, error) = SHIFT_JIS.encode(&str);

        if error {
            panic!("Error: cannot encode kanji.");
        }

        let mut encoded_iter = encoded.iter();
        while let Some(byte_val) = encoded_iter.next() {
            let b1 = *byte_val;
            let b2 = *encoded_iter.next().unwrap();

            let mut val = ((b1 as usize) << 8) | b2 as usize;
            
            if val <= 0x9FFC {
                val -= 0x8140;
            } else if val >= 0xE040 {
                val -= 0xC140;
            }
            
            val = (val & 0xFF) + (val >> 8) * 0xC0;
            bytes.push_bits_big(val, 13);
        }
    }
}