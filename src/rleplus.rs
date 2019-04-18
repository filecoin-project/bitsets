use bitvec::*;

/// Encode the given bitset into their RLE+ encoded representation.
pub fn encode(raw: &BitVec<LittleEndian, u8>) -> BitVec<LittleEndian, u8> {
    let mut encoding = BitVec::new();

    if raw.is_empty() {
        return encoding;
    }

    // encode the very first bit
    // the first block contains this, then alternating
    encoding.push(raw.get(0).unwrap());

    // varint blocks
    // - Typ0 - length 1                      : 1
    // - Typ1 - length fits in a single varint: 01 - varint, fits in 4 bits
    // - Typ2 - length is a regular varint    : 00 - varint
    //
    // Final encoding
    //
    // [ k, b0, b1, ..., bn]
    // k = initial bit, determines the ordering of the actual values
    // bi = varint blocks of Typ{0|1|2}

    let mut count = 1;
    let mut current = raw.get(0);
    let last = raw.len();
    for i in 1..=raw.len() {
        if raw.get(i) != current || i == last {
            if i == last && raw.get(i) == current {
                count += 1;
            }

            if count == 1 {
                // single bits are encoded as "1" bit
                encoding.push(true);
            } else if count < 16 {
                // 4 bits
                let s_vec: BitVec<LittleEndian, u8> = BitVec::from(&[count as u8][..]);

                // prefix: 01
                encoding.push(false);
                encoding.push(true);
                encoding.extend(s_vec.iter().take(4));
                count = 1;
            } else {
                let mut v = [0u8; 10];
                let s = unsigned_varint::encode::u64(count, &mut v);
                let s_vec: BitVec<LittleEndian, u8> = BitVec::from(s);

                // prefix: 00
                encoding.push(false);
                encoding.push(false);

                encoding.extend(s_vec.iter());
                count = 1;
            }
            current = raw.get(i);
        } else {
            count += 1;
        }
    }

    encoding
}

/// Decode an RLE+ encoded bitset into its original form.
pub fn decode(enc: &BitVec<LittleEndian, u8>) -> BitVec<LittleEndian, u8> {
    let mut decoded = BitVec::new();

    if enc.is_empty() {
        return decoded;
    }

    // read the inital bit
    let mut cur = enc.get(0).unwrap();

    // pointer into the encoded bitvec
    let mut i = 1;

    let len = enc.len();

    while i < len {
        // read the next prefix
        match enc.get(i).unwrap() {
            false => {
                // multiple bits
                match enc.get(i + 1) {
                    // prefix: 00
                    Some(false) => {
                        let buf = enc
                            .iter()
                            .skip(i + 2)
                            .take(10 * 8)
                            .collect::<BitVec<LittleEndian, u8>>();
                        let buf_ref: &[u8] = buf.as_ref();
                        let (len, rest) = unsigned_varint::decode::u64(buf_ref).unwrap();

                        // insert this many bits
                        decoded.extend((0..len).map(|_| cur));

                        // prefix
                        i += 2;
                        // this is how much space the varint took in bits
                        i += (buf_ref.len() * 8) - (rest.len() * 8);
                    }
                    // prefix: 01
                    Some(true) => {
                        let buf = enc
                            .iter()
                            .skip(i + 2)
                            .take(4)
                            .collect::<BitVec<LittleEndian, u8>>();
                        let res: Vec<u8> = buf.into();
                        assert_eq!(res.len(), 1);
                        let len = res[0] as usize;

                        // prefix
                        i += 2;
                        // length of the encoded number
                        i += 4;

                        decoded.extend((0..len).map(|_| cur));
                    }
                    None => {
                        panic!("premature end");
                    }
                }
            }
            true => {
                // single bit
                decoded.push(cur);
                i += 1;
            }
        }

        // swith the cur value
        cur = !cur;
    }

    decoded
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::{Rng, RngCore, SeedableRng};
    use rand_xorshift::XorShiftRng;

    #[test]
    fn test_rle_plus_basics() {
        let cases = vec![
            (
                bitvec![LittleEndian; 0; 8],
                bitvec![LittleEndian;
                        0, // starts with 0
                        0, 1, // fits into 4 bits
                        0, 0, 0, 1, // 8
                ],
            ),
            (
                bitvec![LittleEndian; 0, 0, 0, 0, 1, 0, 0, 0],
                bitvec![LittleEndian;
                        0, // starts with 0
                        0, 1, // fits into 4 bits
                        0, 0, 1, 0, // 4 - 0
                        1, // 1 - 1
                        0, 1, // fits into 4 bits
                        1, 1, 0, 0 // 3 - 0
                ],
            ),
        ];

        for (i, case) in cases.into_iter().enumerate() {
            assert_eq!(encode(&case.0), case.1, "case: {}", i);
        }
    }

    #[test]
    fn test_rle_plus_roundtrip_small() {
        let mut rng = XorShiftRng::from_seed([1u8; 16]);

        for _i in 0..10000 {
            let len: usize = rng.gen_range(0, 1000);

            let mut src = vec![0u8; len];
            rng.fill_bytes(&mut src);

            let original: BitVec<LittleEndian, u8> = src.into();

            let encoded = encode(&original);
            let decoded = decode(&encoded);

            assert_eq!(original, decoded);
        }
    }

    #[test]
    fn test_rle_plus_roundtrip_large() {
        let mut rng = XorShiftRng::from_seed([2u8; 16]);

        for _i in 0..100 {
            let len: usize = rng.gen_range(0, 100000);

            let mut src = vec![0u8; len];
            rng.fill_bytes(&mut src);

            let original: BitVec<LittleEndian, u8> = src.into();

            let encoded = encode(&original);
            let decoded = decode(&encoded);

            assert_eq!(original, decoded);
        }
    }
}
