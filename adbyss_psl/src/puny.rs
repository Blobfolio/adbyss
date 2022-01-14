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



#[allow(clippy::many_single_char_names)]
/// # Decode!
pub(super) fn decode(input: &str) -> Option<String> {
	if ! input.is_ascii() { return None; }

	let (mut output, input): (Vec<char>, &str) = input.rfind(DELIMITER).map_or_else(
		|| (Vec::new(), input),
		|pos| (input[..pos].chars().collect(), &input[pos + 1..])
	);

	let mut n: u32 = INITIAL_N;
	let mut i: u32 = 0;
	let mut bias: u32 = INITIAL_BIAS;

	let mut it = input.chars().peekable();
	while it.peek() != None {
		let old_i = i;
		let mut weight = 1;

		for k in 1.. {
			let c = it.next()?;
			let k = k * BASE;
			let digit = decode_digit(c);
			if digit == BASE {
				return None;
			}

			if digit > (u32::MAX - i) / weight { return None; }
			i += digit * weight;

			let t =
				if T_MIN + bias >= k { T_MIN }
				else if T_MAX + bias <= k { T_MAX }
				else { k - bias };

			if digit < t { break; }

			if BASE > (u32::MAX - t) / weight { return None; }
			weight *= BASE - t;
		}

		let len = (output.len() + 1) as u32;
		bias = adapt(i - old_i, len, old_i == 0);

		let il = i / len;
		if n > u32::MAX - il { return None; }
		n += il;
		i %= len;

		let c = char::from_u32(n)?;
		output.insert(i as usize, c);

		i += 1;
	}

	Some(output.into_iter().collect())
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

	if basic_length > 0 { output.push(DELIMITER); }

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

fn decode_digit(c: char) -> u32 {
	let cp = c as u32;
	match c {
		'0'..='9' => cp - ('0' as u32) + 26,
		'A'..='Z' => cp - ('A' as u32),
		'a'..='z' => cp - ('a' as u32),
		_ => BASE,
	}
}

#[inline]
fn value_to_digit(value: u32) -> char {
	match value {
		0..=25 => (value as u8 + b'a') as char,
		26..=35 => (value as u8 - 26 + b'0') as char,
		_ => panic!(),
	}
}
