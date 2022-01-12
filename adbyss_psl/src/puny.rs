/*!
# Adbyss PSL - IDNA

This is a reworking of the punycode handling provided by the excellent [idna](https://github.com/servo/rust-url/) crate.

Unused functionality has been removed, types and arguments have been adjusted
and tweaked based on our specific use cases, etc.

The original license:

Copyright 2016 The rust-url developers.

Licensed under the Apache License, Version 2.0 [LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0)
or the [MIT license](http://opensource.org/licenses/MIT) at your option.
This file may not be copied, modified, or distributed except according to
those terms.
*/

#![allow(clippy::cast_lossless)]
#![allow(clippy::integer_division)]
#![allow(clippy::cast_possible_truncation)]



use std::str::Chars;



// Bootstring parameters for Punycode
static BASE: u32 = 36;
static T_MIN: u32 = 1;
static T_MAX: u32 = 26;
static SKEW: u32 = 38;
static DAMP: u32 = 700;
static INITIAL_BIAS: u32 = 72;
static INITIAL_N: u32 = 0x80;
static DELIMITER: char = '-';



#[derive(Default)]
/// # Decoder!
pub(super) struct Decoder {
	inserts: Vec<(u32, char)>,
}

impl Decoder {
	/// # Decode Iterator
	///
	/// Split the input string and return a vector with encoded character
	/// insertions.
	pub(super) fn decode<'a>(&'a mut self, input: &'a str) -> Option<Decode<'a>> {
		self.inserts.clear();

		// Handle basic ASCII codepoints, which are encoded as-are before the
		// last delimiter.
		let (base, input): (&str, &str) = match input.rfind(DELIMITER) {
			None => ("", input),
			Some(pos) => (
				&input[..pos],
				if pos > 0 { &input[pos + 1..] }
				else { input },
			),
		};

		if ! base.is_ascii() { return None; }

		let base_len: usize = base.len();
		let mut length: u32 = base_len as u32;
		let mut code_point: u32 = INITIAL_N;
		let mut bias: u32 = INITIAL_BIAS;
		let mut i: u32 = 0;
		let mut iter = input.bytes();
		loop {
			let previous_i: u32 = i;
			let mut weight: u32 = 1;
			let mut k: u32 = BASE;
			let mut byte: u8 = match iter.next() {
				None => break,
				Some(byte) => byte,
			};

			// Decode a generalized variable-length integer into delta,
			// which gets added to i.
			loop {
				let digit = match byte {
					byte @ b'0'..=b'9' => byte - b'0' + 26,
					byte @ b'A'..=b'Z' => byte - b'A',
					byte @ b'a'..=b'z' => byte - b'a',
					_ => return None,
				} as u32;

				// Overflow.
				if digit > (u32::MAX - i) / weight {
					return None;
				}

				i += digit * weight;
				let t =
					if k <= bias { T_MIN }
					else if k >= bias + T_MAX { T_MAX }
					else { k - bias };

				if digit < t { break; }

				// Overflow.
				if weight > u32::MAX / (BASE - t) {
					return None;
				}

				weight *= BASE - t;
				k += BASE;
				byte = iter.next()?;
			}

			bias = adapt(i - previous_i, length + 1, previous_i == 0);

			// Overflow.
			if i / (length + 1) > u32::MAX - code_point {
				return None;
			}

			code_point += i / (length + 1);
			i %= length + 1;
			let c = char::from_u32(code_point)?;

			// Move earlier inserts farther out into the string.
			for (idx, _) in &mut self.inserts {
				if *idx >= i {
					*idx += 1;
				}
			}
			self.inserts.push((i, c));
			length += 1;
			i += 1;
		}

		self.inserts.sort_by_key(|(i, _)| *i);
		Some(Decode {
			base: base.chars(),
			inserts: &self.inserts,
			inserted: 0,
			pos: 0,
			len: base_len + self.inserts.len(),
		})
	}
}

/// # Decode Iterator.
pub(super) struct Decode<'a> {
	base: Chars<'a>,
	inserts: &'a [(u32, char)],
	inserted: u32,
	pos: u32,
	len: usize,
}

impl<'a> Iterator for Decode<'a> {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.inserts.get(self.inserted as usize) {
				Some((p, c)) if self.pos.eq(p) => {
					self.inserted += 1;
					self.pos += 1;
					return Some(*c);
				},
				_ => {},
			}

			if let Some(c) = self.base.next() {
				self.pos += 1;
				return Some(c);
			}

			if self.inserted as usize >= self.inserts.len() {
				return None;
			}
		}
	}

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len - self.pos as usize;
		(len, Some(len))
	}
}

impl<'a> ExactSizeIterator for Decode<'a> {
	#[inline]
	fn len(&self) -> usize { self.len - self.pos as usize }
}

#[allow(clippy::comparison_chain)] // We're only matching 2/3 arms.
/// # Encode!
pub(super) fn encode_into(input: &Chars, output: &mut String) -> bool {
	// We can gather a lot of preliminary information in a single iteration.
	let mut input_length: u32 = 0;
	let mut basic_length: u32 = 0;
	for c in input.clone() {
		input_length += 1;
		if c.is_ascii() {
			output.push(c);
			basic_length += 1;
		}
	}

	if basic_length > 0 {
		output.push('-');
	}

	let mut code_point: u32 = INITIAL_N;
	let mut delta: u32 = 0;
	let mut bias: u32 = INITIAL_BIAS;
	let mut processed: u32 = basic_length;
	while processed < input_length {
		// Find the next largest codepoint.
		let min_code_point = input.clone()
			.filter_map(|c| {
				let c = c as u32;
				if c >= code_point { Some(c) }
				else { None }
			})
			.min()
			.unwrap();
		if min_code_point - code_point > (u32::MAX - delta) / (processed + 1) {
			return false;
		}

		// Increase delta to advance the decoder’s <code_point,i> state to
		// <min_code_point,0>.
		delta += (min_code_point - code_point) * (processed + 1);
		code_point = min_code_point;

		for c in input.clone().map(|c| c as u32) {
			if c < code_point {
				delta += 1;
				if delta == 0 { return false; }
			}

			else if c == code_point {
				let mut q = delta;
				let mut k = BASE;
				loop {
					let t =
						if k <= bias { T_MIN }
						else if k >= bias + T_MAX { T_MAX }
						else { k - bias };

					if q < t { break; }

					let value = t + ((q - t) % (BASE - t));
					output.push(value_to_digit(value));
					q = (q - t) / (BASE - t);
					k += BASE;
				}
				output.push(value_to_digit(q));
				bias = adapt(delta, processed + 1, processed == basic_length);
				delta = 0;
				processed += 1;
			}
		}

		delta += 1;
		code_point += 1;
	}

	true
}



#[inline]
fn adapt(mut delta: u32, num_points: u32, first_time: bool) -> u32 {
	delta /= if first_time { DAMP } else { 2 };
	delta += delta / num_points;
	let mut k = 0;
	while delta > ((BASE - T_MIN) * T_MAX) / 2 {
		delta /= BASE - T_MIN;
		k += BASE;
	}
	k + (((BASE - T_MIN + 1) * delta) / (delta + SKEW))
}

#[inline]
fn value_to_digit(value: u32) -> char {
	match value {
		0..=25 => (value as u8 + b'a') as char,
		26..=35 => (value as u8 - 26 + b'0') as char,
		_ => panic!(),
	}
}
