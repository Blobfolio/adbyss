/*!
# Adbyss PSL - IDNA

This module includes a `decode` method for converting PUNYCODE into Unicode,
and an `encode_into` method that converts Unicode into PUNYCODE. This library
ultimately only cares about ASCII, but validation requires checking the
underlying Unicode, so we have to do both when the source contains PUNYCODE.
*/

#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_truncation)]



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
///
/// This method decodes an existing PUNYCODE chunk, converting it back into the
/// original Unicode.
///
/// Note: this method has to allocate because PUNYCODE requires some weird
/// shuffling; we can't just pop things directly where they need to go.
pub(super) fn decode(input: &[char]) -> Option<Vec<char>> {
	let (mut output, input): (Vec<char>, &[char]) = input.iter()
		.rposition(|c| DELIMITER.eq(c))
		.map_or_else(
			|| (Vec::new(), input),
			|pos| (input[..pos].to_vec(), &input[pos + 1..])
		);

	let mut n: u32 = INITIAL_N;
	let mut i: u32 = 0;
	let mut bias: u32 = INITIAL_BIAS;

	let mut it = input.iter().copied().peekable();
	while it.peek().is_some() {
		let old_i = i;
		let mut weight = 1;

		for k in 1.. {
			let c = it.next().filter(char::is_ascii)?;
			let k = k * BASE;
			let digit = decode_digit(c);
			if digit == BASE {
				return None;
			}

			if digit > (u32::MAX - i).wrapping_div(weight) { return None; }
			i += digit * weight;

			let t =
				if T_MIN + bias >= k { T_MIN }
				else if T_MAX + bias <= k { T_MAX }
				else { k - bias };

			if digit < t { break; }

			if BASE > (u32::MAX - t).wrapping_div(weight) { return None; }
			weight *= BASE - t;
		}

		let len = (output.len() + 1) as u32;
		bias = adapt(i - old_i, len, old_i == 0);

		let il = i.wrapping_div(len);
		if n > u32::MAX - il { return None; }
		n += il;
		i %= len;

		let c = char::from_u32(n)?;
		output.insert(i as usize, c);

		i += 1;
	}

	Some(output)
}

#[allow(clippy::comparison_chain)] // We're only matching 2/3 arms.
/// # Encode!
///
/// This converts Unicode into PUNYCODE ASCII, writing the output directly to
/// the specified buffer.
///
/// This method is a reworking of the encoding methods provided by the `idna`
/// crate. It is close enough to the original that it bears mention. Their
/// license is as follows:
///
/// Copyright 2016 The rust-url developers.
///
/// Licensed under the Apache License, Version 2.0 [LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0)
/// or the [MIT license](http://opensource.org/licenses/MIT) at your option.
/// This file may not be copied, modified, or distributed except according to
/// those terms.
pub(super) fn encode_into(input: &[char], output: &mut String) -> bool {
	let mut written: u8 = 0;
	for c in input {
		if c.is_ascii() {
			output.push(*c);
			written += 1;
		}
	}

	let basic_length: u32 = written as u32;
	if basic_length > 0 {
		output.push(DELIMITER);
		written += 1;
	}

	let mut code_point: u32 = INITIAL_N;
	let mut delta: u32 = 0;
	let mut bias: u32 = INITIAL_BIAS;
	let mut processed: u32 = basic_length;
	let input_length: u32 = input.len() as u32;
	while processed < input_length {
		// Find the next largest codepoint.
		let min_code_point = input.iter()
			.filter_map(|c| {
				let c = *c as u32;
				if c >= code_point { Some(c) }
				else { None }
			})
			.min()
			.unwrap();
		if min_code_point - code_point > (u32::MAX - delta).wrapping_div(processed + 1) {
			return false;
		}

		// Increase delta to advance the decoder’s <code_point,i> state to
		// <min_code_point,0>.
		delta += (min_code_point - code_point) * (processed + 1);
		code_point = min_code_point;

		for c in input.iter().map(|c| *c as u32) {
			if c < code_point { delta += 1; }

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
					written += 1;
					q = (q - t).wrapping_div(BASE - t);
					k += BASE;
				}
				output.push(value_to_digit(q));
				written += 1;
				bias = adapt(delta, processed + 1, processed == basic_length);
				delta = 0;
				processed += 1;
			}
		}

		delta += 1;
		code_point += 1;
	}

	// Make sure the chunk is appropriately sized. (The upper limit is 63, but
	// we already wrote "xn--", so -4 is 59.)
	0 < written && written <= 59
}



#[inline]
fn adapt(mut delta: u32, num_points: u32, first_time: bool) -> u32 {
	delta /= if first_time { DAMP } else { 2 };
	delta += delta.wrapping_div(num_points);
	let mut k = 0;
	while delta > ((BASE - T_MIN) * T_MAX).wrapping_div(2) {
		delta /= BASE - T_MIN;
		k += BASE;
	}
	k + ((BASE - T_MIN + 1) * delta).wrapping_div(delta + SKEW)
}

#[inline]
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
const fn value_to_digit(value: u32) -> char {
	match value {
		0..=25 => (value as u8 + b'a') as char,
		26..=35 => (value as u8 - 26 + b'0') as char,
		_ => panic!(),
	}
}
