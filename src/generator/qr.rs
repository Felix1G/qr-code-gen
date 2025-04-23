use image::{ImageBuffer, Rgba};

use crate::generator::ecc::version_information_process;

use super::{
    data::{get_format_ecc, obtain_qr_alignment},
    ECCLevel,
};

fn mask0(i: usize, j: usize) -> bool {
    (i + j) % 2 == 0
}

fn mask1(i: usize, _j: usize) -> bool {
    i % 2 == 0
}

fn mask2(_i: usize, j: usize) -> bool {
    j % 3 == 0
}

fn mask3(i: usize, j: usize) -> bool {
    (i + j) % 3 == 0
}

fn mask4(i: usize, j: usize) -> bool {
    ((i / 2) + (j / 3)) % 2 == 0
}

fn mask5(i: usize, j: usize) -> bool {
    ((i * j) % 2) + ((i * j) % 3) == 0
}

fn mask6(i: usize, j: usize) -> bool {
    (((i * j) % 2) + ((i * j) % 3)) % 2 == 0
}

fn mask7(i: usize, j: usize) -> bool {
    (((i + j) % 2) + ((i * j) % 3)) % 2 == 0
}

pub struct QRCode {
    data: Vec<u8>,
    mat: Vec<Vec<u8>>,
    align_pat: Vec<(u8, u8)>,
    size: usize,
    version: u8,
    ecc: u8,
}

impl QRCode {
    pub fn new(data: Vec<u8>, version: u8, ecc: &ECCLevel) -> Self {
        let size = 21 + ((version as usize) - 1) * 4;
        let bits = (size >> 3) + ((size & 0b111) > 0) as usize;
        let mut mat = Vec::<Vec<u8>>::new();
        mat.resize_with(size, || {
            let mut v = Vec::<u8>::new();
            v.resize(bits, 0);
            return v;
        });

        let align_pattern = &obtain_qr_alignment()[(version - 1) as usize];
        let mut align_pat = Vec::new();
        for x_idx in 0..align_pattern.len() {
            for y_idx in 0..align_pattern.len() {
                let px = align_pattern[x_idx] as u8;
                let py = align_pattern[y_idx] as u8;
                if (px - 2 < 7 && py - 2 < 7)
                    || (px + 2 >= size as u8 - 7 && py - 2 < 7)
                    || (px - 2 < 7 && py + 2 >= size as u8 - 7)
                {
                    continue;
                }

                align_pat.push((px, py));
            }
        }

        let mut qr_code = Self {
            data,
            mat,
            align_pat,
            version,
            size,
            ecc: match ecc {
                &ECCLevel::Low => 0,
                &ECCLevel::Medium => 1,
                &ECCLevel::Quartile => 2,
                &ECCLevel::High => 3,
            },
        };

        qr_code.generate_matrix();
        let mask = qr_code.mask_matrix();
        qr_code.add_format_symbols(mask);

        return qr_code;
    }

    fn set_bit_mat(mat: &mut Vec<Vec<u8>>, x: usize, y: usize, flag: bool) {
        let val = mat[y][x / 8];
        mat[y][x / 8] = val ^ (val & (1 << (7 - x % 8))) | ((flag as u8) << (7 - x % 8));
    }

    fn set_bit(&mut self, x: usize, y: usize, flag: bool) {
        Self::set_bit_mat(&mut self.mat, x, y, flag);
    }

    fn add_find_pattern(&mut self) {
        for x in 0..7 {
            for y in 0..7 {
                if ((y == 1 || y == 5) && x >= 1 && x <= 5)
                    || ((x == 1 || x == 5) && y >= 1 && y <= 5)
                {
                    continue;
                }

                self.set_bit(x, y, true);
                self.set_bit(x, y + self.size - 7, true);
                self.set_bit(x + self.size - 7, y, true);
            }
        }
    }

    fn add_timing_pattern(&mut self) {
        for x in 8..(self.size - 8) {
            self.set_bit(x, 6, x % 2 == 0);
        }

        for y in 8..(self.size - 8) {
            self.set_bit(6, y, y % 2 == 0);
        }
    }

    fn add_alignment_pattern(&mut self) {
        for idx in 0..self.align_pat.len() {
            let x = self.align_pat[idx].0 as isize;
            let y = self.align_pat[idx].1 as isize;

            for i in -2 as isize..=2 {
                self.set_bit((x + i) as usize, (y + 2) as usize, true);
                self.set_bit((x + i) as usize, (y - 2) as usize, true);
            }

            for i in -1 as isize..=1 {
                self.set_bit((x - 2) as usize, (y + i) as usize, true);
                self.set_bit((x + 2) as usize, (y + i) as usize, true);
            }

            self.set_bit(x as usize, y as usize, true);
        }
    }

    fn add_format_symbols(&mut self, mask: u8) {
        let mut fmt = 0u16;

        fmt |= (match self.ecc {
            0 => 0b01,
            1 => 0b00,
            2 => 0b11,
            3 => 0b10,
            _ => 0b00,
        }) << 3;

        fmt |= mask as u16;

        let err = get_format_ecc(fmt as u8);
        fmt = (fmt << 10) | err;

        fmt = fmt ^ 0b101010000010010;

        //top left
        for y in 0..=5 {
            self.set_bit(8, y, (fmt >> y) & 1 == 1);
        }

        self.set_bit(8, 7, (fmt >> 6) & 1 == 1);
        self.set_bit(8, 8, (fmt >> 7) & 1 == 1);
        self.set_bit(7, 8, (fmt >> 8) & 1 == 1);

        for x in (0..=5).rev() {
            self.set_bit(5 - x, 8, (fmt >> (9 + x)) & 1 == 1);
        }

        //top right
        for x in 0..=7 {
            self.set_bit(self.size - x - 1, 8, (fmt >> x) & 1 == 1);
        }

        //bottom left
        for y in (0..=6).rev() {
            self.set_bit(8, self.size - y - 1, (fmt >> (14 - y)) & 1 == 1);
        }

        //the single compulsory black module
        self.set_bit(8, self.size - 8, true);

        //version information
        if self.version >= 7 {
            let vers_info = version_information_process(self.version);
            
            let mut idx = 0;
            let mut px = self.size - 11;
            let mut py = 0;
            while idx < 18 {
                self.set_bit(px, py, (vers_info >> idx) & 1 == 1);
                self.set_bit(py, px, (vers_info >> idx) & 1 == 1);
                px += 1;
                if px == self.size - 8 {
                    px = self.size - 11;
                    py += 1;
                }
                idx += 1;
            }
        }
    }

    fn is_occupied(&self, x: usize, y: usize) -> bool {
        if x >= self.size || y >= self.size {
            return true; //out of bounds
        }

        if y == 6 || x == 6 {
            return true; //timing
        }

        //finder & format
        if x < 9 && y < 9 {
            return true;
        }

        if x >= self.size - 8 && y < 9 {
            return true;
        }

        if x < 9 && y >= self.size - 8 {
            return true;
        }

        if self.version >= 7 {
            if y < 6 && x >= self.size - 11 {
                return true;
            }

            if x < 6 && y >= self.size - 11 {
                return true;
            }
        }

        for (px, py) in &self.align_pat {
            let px = *px as usize;
            let py = *py as usize;
            if (x >= px - 2 && x <= px + 2) && (y >= py - 2 && y <= py + 2) {
                return true;
            }
        }

        false
    }

    fn generate_matrix(&mut self) {
        self.add_find_pattern();
        self.add_timing_pattern();
        self.add_alignment_pattern();

        let mut px = self.size - 1;
        let mut py = self.size - 1;
        let mut move_up = true;
        let mut bit = 0;
        let mut pos = 0;
        let length = self.data.len();

        while pos < length {
            if !self.is_occupied(px, py) {
                self.set_bit(px, py, (self.data[pos] >> (7 - bit)) & 1 == 1);
                bit += 1;
                if bit == 8 {
                    bit = 0;
                    pos += 1;
                }
            }

            if !self.is_occupied(px - 1, py) && pos < length {
                self.set_bit(px - 1, py, (self.data[pos] >> (7 - bit)) & 1 == 1);
                bit += 1;
                if bit == 8 {
                    bit = 0;
                    pos += 1;
                }
            }

            if move_up {
                if py == 0 {
                    if px <= 2 {
                        break;
                    }
                    move_up = false;
                    px -= 2;
                } else {
                    py -= 1;
                }
            } else {
                if py == self.size - 1 {
                    if px <= 2 {
                        break;
                    }
                    move_up = true;
                    px -= 2;
                } else {
                    py += 1;
                }
            }

            if px == 6 {
                px = 5;
            }
        }

        /*for vec in &self.mat {
            let mut s = 0;
            for bit in vec {
                for i in (0..8).rev() {
                    print!("{}", if ((bit & (1 << i)) >> i) == 1 { "◼️ " } else { "◻️ " });
                    s += 1;
                    if s >= self.size {
                        break;
                    }
                }
                if s >= self.size {
                    break;
                }
            }
            println!();
        }*/
    }

    fn get_val_mat(mat: &Vec<Vec<u8>>, x: usize, y: usize) -> bool {
        (mat[y][x / 8] & (1 << (7 - x % 8))) >> (7 - x % 8) == 1
    }

    fn get_range_x_mat(mat: &Vec<Vec<u8>>, x1: usize, x2: usize, y: usize) -> Vec<bool> {
        let mut vec = Vec::new();
        for x in x1..x2 {
            vec.push(Self::get_val_mat(mat, x, y));
        }
        vec
    }

    fn get_range_y_mat(mat: &Vec<Vec<u8>>, x: usize, y1: usize, y2: usize) -> Vec<bool> {
        let mut vec = Vec::new();
        for y in y1..y2 {
            vec.push(Self::get_val_mat(mat, x, y));
        }
        vec
    }

    fn perform_mask(&self, mat: &mut Vec<Vec<u8>>, mask: fn(usize, usize) -> bool) -> usize {
        let mut penalty = 0;

        let mut blacks = 0;
        let mut whites = 0;

        let mut rule1_white = false;
        let mut rule1_count = 0isize;

        let mut y = 0;
        for yidx in 0..mat.len() {
            let mut x = 0;
            for xidx in 0..mat[yidx].len() {
                for i in (0..8).rev() {
                    if self.is_occupied(x, y) {
                        x += 1;
                        penalty += (rule1_count - 5).max(0) as usize;
                        rule1_count = 0;
                        continue;
                    }

                    if mask(y, x) {
                        let byte = mat[yidx][xidx];
                        mat[yidx][xidx] = byte ^ (1 << i);
                    }

                    //rule 1 horizontal and part of rule 4
                    if mat[yidx][xidx] & (1 << i) == 0 {
                        //white
                        if !rule1_white {
                            penalty += (rule1_count - 5).max(0) as usize;
                            rule1_white = true;
                            rule1_count = 0;
                        }
                        rule1_count += 1;

                        whites += 1;
                    } else {
                        //black
                        if rule1_white {
                            penalty += (rule1_count - 5).max(0) as usize;
                            rule1_white = false;
                            rule1_count = 0;
                        }
                        rule1_count += 1;

                        blacks += 1;
                    }

                    x += 1;
                    if x >= self.size {
                        break;
                    }
                }

                if x >= self.size {
                    break;
                }
            }
            y += 1;

            penalty += (rule1_count - 5).max(0) as usize;
            rule1_count = 0;
        }

        let mut rule2_count = 0;

        //rule 1 vertical and rule 2
        rule1_white = false;
        rule1_count = 0;
        for x in 0..self.size {
            for y in 0..self.size {
                if self.is_occupied(x, y) {
                    penalty += (rule1_count - 5).max(0) as usize;
                    rule1_count = 0;
                    continue;
                }

                let cell = Self::get_val_mat(mat, x, y);
                if cell {
                    //black
                    if rule1_white {
                        penalty += (rule1_count - 5).max(0) as usize;
                        rule1_white = false;
                        rule1_count = 0;
                    }
                    rule1_count += 1;
                } else {
                    //white
                    if !rule1_white {
                        penalty += (rule1_count - 5).max(0) as usize;
                        rule1_white = true;
                        rule1_count = 0;
                    }
                    rule1_count += 1;
                }

                if !self.is_occupied(x + 1, y) && Self::get_val_mat(mat, x + 1, y) == cell {
                    if !self.is_occupied(x, y + 1) && Self::get_val_mat(mat, x, y + 1) == cell {
                        if !self.is_occupied(x + 1, y + 1)
                            && Self::get_val_mat(mat, x + 1, y + 1) == cell
                        {
                            rule2_count += 1;
                        }
                    }
                }
            }

            penalty += (rule1_count - 5).max(0) as usize;
            rule1_count = 0;
        }

        // rule 2
        penalty += 3 * rule2_count;

        // rule 4
        penalty += (10.0 * ((0.5 - blacks as f32 / (blacks + whites) as f32).abs() / 0.05).floor())
            as usize;

        // rule 3
        let rule3_seq = vec![true, false, true, true, true, false, true];
        let mut rule3_count = 0;
        let mut px = 0;
        let mut py = 0;

        while py < self.size {
            // horizontal scan
            while px <= self.size - 7 {
                //need space for the 7 modules
                let mut failed = false;
                for x in 0..7 {
                    if self.is_occupied(px + x, py) {
                        px += x;
                        failed = true;
                        break;
                    }
                }

                if failed {
                    px += 1;
                    continue;
                }

                if Self::get_range_x_mat(mat, px, px + 7, py) == rule3_seq {
                    if px - 4 >= 7 && !Self::get_range_x_mat(mat, px - 4, px, py).contains(&true) {
                        rule3_count += 1;
                        px += 7;
                    } else if px + 10 < self.size
                        && !Self::get_range_x_mat(mat, px + 7, px + 11, py).contains(&true)
                    {
                        rule3_count += 1;
                        px += 11;
                    } else {
                        px += 1;
                    }
                } else {
                    px += 1;
                }
            }

            px = 0;
            py += 1;
        }

        px = 0;
        py = 0;
        while px < self.size {
            // vertical scan
            while py <= self.size - 7 {
                //need space for the 7 modules
                let mut failed = false;
                for y in 0..7 {
                    if self.is_occupied(px, py + y) {
                        py += y;
                        failed = true;
                        break;
                    }
                }

                if failed {
                    py += 1;
                    continue;
                }

                if Self::get_range_y_mat(mat, px, py, py + 7) == rule3_seq {
                    if py - 4 >= 7 && !Self::get_range_y_mat(mat, px, py - 4, py).contains(&true) {
                        rule3_count += 1;
                        py += 7;
                    } else if py + 10 < self.size
                        && !Self::get_range_y_mat(mat, px, py + 7, py + 11).contains(&true)
                    {
                        rule3_count += 1;
                        py += 11;
                    } else {
                        py += 1;
                    }
                } else {
                    py += 1;
                }
            }

            py = 0;
            px += 1;
        }

        penalty += 40 * rule3_count;

        penalty
    }

    fn mask_matrix(&mut self) -> u8 {
        let masks = [mask0, mask1, mask2, mask3, mask4, mask5, mask6, mask7];
        let mut min_err = 1e8 as usize;
        let mut mat_best = None;
        let mut mask_idx = 0;
        let mut mask = 0;

        for func in masks {
            let mut mat = self.mat.clone();
            let err = self.perform_mask(&mut mat, func);
            if err < min_err {
                min_err = err;
                mat_best = Some(mat);
                mask = mask_idx;
            }
            mask_idx += 1;
        }

        self.mat = mat_best.unwrap();

        return mask;
    }

    pub fn gen_image(&self, pixel: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let img_size = (self.size as u32 + 8) * pixel;
        let mut image = ImageBuffer::new(img_size, img_size);
        let white: Rgba<u8> = Rgba([255, 255, 255, 255]);
        let black: Rgba<u8> = Rgba([0, 0, 0, 255]);

        for x in 0..img_size {
            for y in 0..(4 * pixel) {
                image.put_pixel(x, y, white);
                image.put_pixel(x, img_size - y - 1, white);
            }
        }

        for y in (4 * pixel)..(img_size - 4 * pixel) {
            for x in 0..(4 * pixel) {
                image.put_pixel(x, y, white);
                image.put_pixel(img_size - x - 1, y, white);
            }
        }

        let mut y = 4 * pixel;
        for vec in &self.mat {
            let mut x = 4 * pixel;
            let mut xi = 0;
            for byte in vec {
                for i in (0..8).rev() {
                    let color = if ((byte & (1 << i)) >> i) == 1 {
                        black
                    } else {
                        white
                    };

                    for px in x..(x + pixel) {
                        for py in y..(y + pixel) {
                            image.put_pixel(px, py, color);
                        }
                    }

                    x += pixel;
                    xi += 1;
                    if xi >= self.size {
                        break;
                    }
                }

                if xi >= self.size {
                    break;
                }
            }
            y += pixel;
        }

        image
    }
}
