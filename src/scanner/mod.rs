use std::{fs::File, io::Write, path::Path, process::exit};

use image::ImageReader;
use rqrr::PreparedImage;

#[derive(Debug)]
pub struct Scanner {
    input: String,
    output: String
}

impl Scanner {
    pub fn new(input: String, output: String) -> Self {
        Self {
            input,
            output
        }
    }

    pub fn scan(&self) {
        if !Path::new(&self.input).exists() {
            eprintln!("Error: path does not exist.");
            exit(0);
        }

        let img = ImageReader::open(&self.input)
            .expect("Error: failed to open image.")
            .decode()
            .map_err(|e| format!("Error: failed to decode image '{}'.", e))
            .expect("Error: failed to decode image.")
            .to_luma8();
        let mut prep_img = PreparedImage::prepare(img);
        let grids = prep_img.detect_grids();

        if grids.is_empty() {
            eprintln!("Error: QR code not found.");
            exit(0);
        }

        let mut str = String::new();
        let mut idx = 0;
        for grid in grids {
            match grid.decode() {
                Ok((_meta_, content)) => {
                    str.push_str(format!("Content #{} ---\n", idx).as_str());
                    str.push_str(&content);
                    idx += 1;
                }
                Err(_) => {}
            }
        }

        if idx == 0 {
            eprintln!("Error: QR code cannot be parsed.");
            exit(0);
        }

        let mut file = File::create(&self.output).unwrap();
        match file.write(str.as_bytes()) {
            Ok(_) => {
                println!("QR code(s) parsed successfully, written into {}.", self.output);
            }
            Err(x) => {
                eprintln!("Error: parsed text cannot be written into {}", self.output);
                eprintln!("Error: {}", x);
                exit(0);
            }
        }
    }
}