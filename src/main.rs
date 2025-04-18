mod generator;
use std::process::exit;

use generator::{ECCLevel, Flag, Generator};

fn usage_str() -> String {
	String::from("
Usage: qr-gen [OPTIONS (optional)] <text OR data file path> [generated-image-path (default: qr_code.png)] [pixel size (default: 5)]
Options:
	-f: data file path is provided | (default is false)
	-b: convert data to bytes, encoding types (eci headers) will not be considered | (default is false)
	-v[number]: minimum version of QR code being version 'number' from 1 to 40. (eg: -v3) | (default is 1)
	-e[number]: error correction, takes up about ~x% space. 0 = low (~7%), 1 = medium (~15%), 2 = quartile (~25%), 3 = high (~30%) | (default is quartile)
")
}

fn set_options(op: &String, flag: &mut Flag) {
	match op.as_str() {
		"-f" => flag.data = true,
		"-b" => flag.bytes = true,
		_ => {
			if op.starts_with("-v") {
				let num = String::from(op).split_off(2);
				if num.is_empty() {
					println!("Error: minimum version must be provided. (eg: -v1)");
					exit(0);
				}

				match num.parse::<u32>() {
					Ok(x) => {
						if x > 0 && x <= 40 {
							flag.min_vers = x as u8;
						} else {
							println!(
								"Error: minimum version must be 1 to 40 inclusive but given '{}'.",
								num
							);
							exit(0);
						}
					}
					Err(_) => {
						println!(
							"Error: minimum version must be integer but given '{}'.",
							num
						);
						exit(0);
					}
				}
			} else if op.starts_with("-e") {
				let num = String::from(op).split_off(2);
				if num.is_empty() {
					println!("Error: error correction configuration must be provided. (eg: -e2)");
					exit(0);
				}

				match num.parse::<u32>() {
					Ok(x) => {
						if x <= 3 {
							flag.ecc = match x {
								0 => ECCLevel::Low,
								1 => ECCLevel::Medium,
								2 => ECCLevel::Quartile,
								3 => ECCLevel::High,
								_ => ECCLevel::Quartile,
							}
						} else {
							println!("Error: use -h to see how to use the '-e' flag.");
							exit(0);
						}
					}
					Err(_) => {
						println!("Error: use -h to see how to use the '-e' flag.");
						exit(0);
					}
				}
			} else {
				println!("Unknown flag: {}", op);
				exit(0);
			}
		}
	}
}

fn main() {
	let args = std::env::args().collect::<Vec<String>>();

	if args.len() > 1 {
		if args[1] == "-h" {
			println!("{}", usage_str());
		} else {
			let mut idx = 1;
			let mut flag = Flag::new();

			while args[idx].starts_with("-") {
				set_options(&args[idx], &mut flag);
				idx += 1;
			}

			let text = args[idx].clone();
			let path = if args.len() > idx + 1 {
				args[idx + 1].clone()
			} else {
				String::from("qr_code.png")
			};
			let size = if args.len() > idx + 2 {
				match args[idx + 2].parse::<u32>() {
					Ok(x) => x,
					Err(_) => {
						println!(
							"Error: pixel size must be integer but given '{}'.",
							args[idx + 2]
						);
						exit(0);
					}
				}
			} else {
				5
			};

			//println!("{} {} {} {} {} {}", text, path, size, flag.data, flag.bytes, flag.min_vers);

			let gen = Generator::new(text, path, size, flag);
			gen.run();
		}
	} else {
		println!("use -h to see help.");
	}
}
