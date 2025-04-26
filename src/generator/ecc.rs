struct GaloisField {
    exp: [u8; 256],
    log: [u8; 256],
}

impl GaloisField {
    fn new() -> Self {
        let mut exp = [0u8; 256];
        let mut log = [0u8; 256];

        let mut x = 1usize;
        for e in 1..=255 {
            if x > 127 {
                x <<= 1;
                x ^= 285; // x^8 + x^4 + x^3 + x^2 + 1
            } else {
                x <<= 1;
            }
            
            exp[e % 255] = x as u8;
            log[x as usize] = (e % 255) as u8;
        }

        Self { exp, log }
    }

    fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let lgs = (self.log[a as usize] as usize + self.log[b as usize] as usize) % 255;
            self.exp[lgs]
        }
    }
}

fn poly_mul(a: &[u8], b: &[u8], field: &GaloisField) -> Vec<u8> {
    let mut result = vec![0u8; a.len() + b.len() - 1];

    for i in 0..result.len() {
        let mut coeff = 0;
        for aidx in 0..a.len() {
            let bidx = i as isize - aidx as isize;
            if bidx >= b.len() as isize || bidx < 0 {
                continue;
            }

            coeff ^= field.mul(a[aidx], b[bidx as usize]);
        }
        result[i] = coeff;
    }

    result
}

fn generate_generator_poly(field: &GaloisField, err_len: usize) -> Vec<u8> {
    let mut gen = vec![1u8]; //starts with g(x) = 1

    for i in 0..err_len {
        let root = field.exp[i]; //a^i
        let next = vec![1, root]; //(x - a^i)

        //multiply current gen(x) by (x - α^i)
        gen = poly_mul(&gen, &next, field);
    }

    gen
}

pub struct ErrorCorrection {
    gf: GaloisField,
}

impl ErrorCorrection {
    pub fn new() -> Self {
        let err = Self {
            gf: GaloisField::new(),
        };
        
        return err;
    }

    pub fn calculate(&self, bytes: &Vec<u8>, err_len: usize) -> Vec<u8> {
        let gen = generate_generator_poly(&self.gf, err_len);
        let mut data = bytes.clone();
        data.resize(bytes.len() + err_len, 0);

        for i in 0..bytes.len() {
            let coef = data[i];
            if coef != 0 {
                for j in 0..gen.len() {
                    data[i + j] ^= self.gf.mul(coef, gen[j]);
                }
            }
        }

        data[bytes.len()..].to_vec()
    }
}

pub fn version_information_process(version: u8) -> u32 {
    const GEN: u32 = 0x1F25;

    let mut num = (version as u32) << 12;

    for i in (12..=17).rev() {
        if (num >> i) & 1 == 1 {
            num ^= GEN << (i - 12);
        }
    }

    let bch = num & 0xFFF;
    return ((version as u32) << 12) | bch;
}
