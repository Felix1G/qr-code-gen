pub struct BitStream {
	bytes: Vec<u8>,
	offset: u8
}

impl BitStream {
	pub fn new() -> Self {
		Self {
			bytes: Vec::new(),
			offset: 0
		}
	}

	pub fn push(&mut self, byte: u8) {
		if self.offset == 0 {
			self.bytes.push(byte);
		} else {
			let last_num = self.bytes.last_mut().unwrap();
			*last_num |= byte >> self.offset;
			self.bytes.push((byte & (0xFF >> (8 - self.offset))) << (8 - self.offset));
		}
	}

	/// @param number number to push
	/// @param size number of bits to push
	pub fn push_bits(&mut self, number: u8, size: u8) {
		if size > 8 {
		   panic!("BitStream: push_bits size, {size} > 8.");
		}

		let num = number & (0xFF >> (8 - size));

		if size == 8 {
			self.push(num);
			return;
		}

		if self.bytes.is_empty() {
			self.bytes.push(num << (8 - size));
			self.offset = size;
			return;
		}
		
		if self.offset + size == 8 {
			let last_num = self.bytes.last_mut().unwrap();
			*last_num |= num;

			self.offset = 0;
		} else if self.offset + size > 8 {
			let last_num = self.bytes.last_mut().unwrap();
			*last_num |= num >> (size - (8 - self.offset));

			self.offset = size - (8 - self.offset); //remaining size
			self.bytes.push((num & (0xFF >> (8 - self.offset))) << (8 - self.offset));
		} else {
			if self.offset == 0 {
				self.bytes.push(num << (8 - size));
				self.offset = size;
				return;
			}

			self.offset += size;
			
			let last_num = self.bytes.last_mut().unwrap();
			*last_num |= num << (8 - self.offset);
		}
	}
	
	/// @param number number to push
	/// @param size number of bits to push
	pub fn push_bits_big(&mut self, number: usize, size: u8) {
		if size <= 8 {
			self.push_bits(number as u8, size);
		} else {
			let rem = size % 8;
			let blocks = size / 8;
			
			if rem != 0 {
				self.push_bits((number >> (8 * blocks)) as u8, rem);
			}
			
			for i in 0..blocks {
				self.push_bits((number >> (8 * (blocks - i - 1))) as u8, 8);
			}
		}
	}

	pub fn consume(self) -> (Vec<u8>, usize) {
		let sub_size = if self.offset == 0 { 0 } else { 8 - self.offset } as usize;
		let size = 8 * self.bytes.len() - sub_size;
		(self.bytes, size)
	}

	#[allow(dead_code)]
	pub fn debug_print(&self) {
		for num in &self.bytes {
			print!("{num:08b} ")
		}
	}
}