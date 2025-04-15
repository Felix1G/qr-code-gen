struct GaloisField {
    exp: [u8; 512],
    log: [u8; 512]
}

impl GaloisField {
    fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 512];

        let mut x = 1u8;
        for i in 0..255 {
            exp[i] = x;
            log[x as usize] = i as u8;
            x = Self::mul_no_table(x, 2);
        }

        for i in 0..255 {
            exp[i + 255] = exp[i];
        }

        Self { exp, log }
    }

    fn mul_no_table(a: u8, b: u8) -> u8 {
        let mut a = a;
        let mut b = b;
        let mut res = 0;

        while b != 0 {
            if b & 1 != 0 {
                res ^= a;
            }

            let carry = a & 0x80;
            a <<= 1;

            if carry != 0 {
                a ^= 0x1D;
            }

            b >>= 1;
        }

        res
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

    for i in 0..a.len() {
        for j in 0..b.len() {
            let product = field.mul(a[i], b[j]);
            result[i + j] ^= product; //add in GF(256)
        }
    }

    result
}

fn generate_generator_poly(field: &GaloisField, err_len: usize) -> Vec<u8> {
    let mut gen = vec![1u8]; //starts with g(x) = 1

    for i in 0..err_len {
        let root = field.exp[i]; //a^i
        let next = vec![1u8, root]; //(x - a^i)

        //multiply current gen(x) by (x - α^i)
        gen = poly_mul(&gen, &next, field);
    }

    gen
}

pub struct ErrorCorrection {
    gf: GaloisField
}

impl ErrorCorrection {
    pub fn new() -> Self {
        Self {
            gf: GaloisField::new()
        }
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