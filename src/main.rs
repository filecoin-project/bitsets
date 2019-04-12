#![feature(wrapping_int_impl)]

use bitvec::*;
use croaring::Bitmap;
use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::Compression;
use rand::distributions::Uniform;
use rand::prelude::*;
use std::io::prelude::*;

mod concise;

use self::concise::Concise;

fn main() {
    bench_random();
    bench_cont();
}

fn bench_random() {
    let mut rng = rand::thread_rng();

    // let sample_count = 100;

    for &total_sectors in &[10_000, 100_000, 1_000_000] {
        for part in &[2, 5, 10] {
            let mut raw_sizes = Vec::new();
            let mut rle_sizes = Vec::new();
            let mut rle_plus_sizes = Vec::new();
            let mut roaring_sizes = Vec::new();
            let mut con_sizes = Vec::new();
            let mut gz_sizes = Vec::new();
            let mut zlib_sizes = Vec::new();

            println!("-- Random selections (up to {}%) --", part);

            let mut selected_sectors_vec = Vec::new();
            for _ in 0..10 {
                // nothing selected
                let mut raw = bitvec![LittleEndian; 0u8; total_sectors];

                // select some, randomly
                let selected_sectors = rng.gen_range(1, (total_sectors / 100) * part);
                selected_sectors_vec.push(selected_sectors);
                let sector_dist = Uniform::new(0, total_sectors);
                let mut bm = Bitmap::create();
                let mut con = Concise::new();

                for _ in 0..selected_sectors {
                    let v = rng.sample(sector_dist);
                    raw.set(v, true);
                    bm.add(v as u32);
                    con.append(v as i32);
                }
                assert!(raw.count_ones() <= selected_sectors);

                let rle_enc = rle(&raw);
                let rle_plus_enc = rle_plus(&raw);

                bm.run_optimize();

                let mut gz = GzEncoder::new(Vec::new(), Compression::best());
                gz.write_all(raw.as_ref()).unwrap();
                let gz_enc = gz.finish().unwrap();
                let mut zlib = ZlibEncoder::new(Vec::new(), Compression::best());
                zlib.write_all(raw.as_ref()).unwrap();
                let zlib_enc = zlib.finish().unwrap();

                raw_sizes.push(raw.len() / 8);
                rle_sizes.push(rle_enc.len() / 8);
                rle_plus_sizes.push(rle_plus_enc.len() / 8);
                roaring_sizes.push(bm.get_serialized_size_in_bytes());
                con_sizes.push(con.size());
                gz_sizes.push(gz_enc.len());
                zlib_sizes.push(zlib_enc.len());
            }

            let len = raw_sizes.len();
            // println!("random selection: ({} count)", len);
            println!(
                "sectors: {}/{}",
                average(&selected_sectors_vec),
                total_sectors
            );
            // println!("raw:     {} bytes", raw.len() / 8);
            // println!("rle:     {} bytes", rle_enc.len() / 8);
            // println!("roaring: {} bytes", bm.get_serialized_size_in_bytes());
            // println!("concise: {} bytes", con.size());
            // println!("gz:      {} bytes", gz_enc.len());
            // println!("zlib:    {} bytes", zlib_enc.len());

            println!("  raw:     {} bytes (avg)", average(&raw_sizes));
            println!("  rle:     {} bytes (avg)", average(&rle_sizes));
            println!("  rle+:    {} bytes (avg)", average(&rle_plus_sizes));
            println!("  roaring: {} bytes (avg)", average(&roaring_sizes),);
            println!("  concise: {} bytes (avg)", average(&con_sizes));
            println!("  gz:      {} bytes (avg)", average(&gz_sizes));
            println!("  zlib:    {} bytes (avg)", average(&zlib_sizes));
        }
    }
}

fn bench_cont() {
    let mut rng = rand::thread_rng();

    // let sample_count = 100;

    for &total_sectors in &[10_000, 100_000, 1_000_000] {
        for count in &[2, 5, 10] {
            for part in &[2, 5, 10] {
                let mut raw_sizes = Vec::new();
                let mut rle_sizes = Vec::new();
                let mut rle_plus_sizes = Vec::new();
                let mut roaring_sizes = Vec::new();
                let mut con_sizes = Vec::new();
                let mut gz_sizes = Vec::new();
                let mut zlib_sizes = Vec::new();

                println!(
                    "-- Multiple {} contigous selections (each up to {}%) --",
                    count, part
                );
                let mut total_selected_sectors_vec = Vec::new();

                for _ in 0..10 {
                    // nothing selected
                    let mut raw = bitvec![LittleEndian; 0u8; total_sectors];
                    let mut bm = Bitmap::create();
                    let mut con = Concise::new();
                    // select some, randomly, but contigous
                    let mut total_selected_sectors = 0;
                    for _ in 0..*count {
                        let selected_sectors = rng.gen_range(1, (total_sectors / 100) * part);
                        total_selected_sectors += selected_sectors;
                        let start = rng.gen_range(0, total_sectors - selected_sectors);

                        for i in start..start + selected_sectors {
                            raw.set(i, true);
                            bm.add(i as u32);
                            con.append(i as i32);
                        }
                    }
                    total_selected_sectors_vec.push(total_selected_sectors);
                    let rle_enc = rle(&raw);
                    let rle_plus_enc = rle_plus(&raw);

                    bm.run_optimize();
                    let mut gz = GzEncoder::new(Vec::new(), Compression::best());
                    gz.write_all(raw.as_ref()).unwrap();
                    let gz_enc = gz.finish().unwrap();
                    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::best());
                    zlib.write_all(raw.as_ref()).unwrap();
                    let zlib_enc = zlib.finish().unwrap();

                    raw_sizes.push(raw.len() / 8);
                    rle_sizes.push(rle_enc.len() / 8);
                    rle_plus_sizes.push(rle_plus_enc.len() / 8);
                    roaring_sizes.push(bm.get_serialized_size_in_bytes());
                    con_sizes.push(con.size());
                    gz_sizes.push(gz_enc.len());
                    zlib_sizes.push(zlib_enc.len());
                }
                //
                // println!("raw:     {} bytes", raw.len() / 8);
                // println!("rle:     {} bytes", rle_enc.len() / 8);
                // println!("roaring: {} bytes", bm.get_serialized_size_in_bytes());
                // println!("concise: {} bytes", con.size());
                // println!("gz:      {} bytes", gz_enc.len());
                // println!("zlib:    {} bytes", zlib_enc.len());

                let len = raw_sizes.len();
                // println!("contingous slices: ({} count)", len);
                println!(
                    "sectors: {}/{}",
                    average(&total_selected_sectors_vec),
                    total_sectors
                );
                println!("  raw:     {} bytes (avg)", average(&raw_sizes));
                println!("  rle:     {} bytes (avg)", average(&rle_sizes));
                println!("  rle+:    {} bytes (avg)", average(&rle_plus_sizes));
                println!("  roaring: {} bytes (avg)", average(&roaring_sizes),);
                println!("  concise: {} bytes (avg)", average(&con_sizes));
                println!("  gz:      {} bytes (avg)", average(&gz_sizes));
                println!("  zlib:    {} bytes (avg)", average(&zlib_sizes));
            }
        }
    }
}

fn average(numbers: &[usize]) -> f32 {
    numbers.iter().sum::<usize>() as f32 / numbers.len() as f32
}

/// Simple run-length encoding.
fn rle(raw: &BitVec<LittleEndian, u8>) -> BitVec<LittleEndian, u8> {
    let mut encoding = BitVec::new();

    let mut count = 1;
    let last = raw.len() - 1;
    for i in 1..raw.len() {
        let prev = raw.get(i - 1);
        if raw.get(i) != prev || i == last {
            if i == last {
                count += 1;
            }
            let mut v = [0u8; 5];
            let s = unsigned_varint::encode::u32(count, &mut v);

            let s_vec: BitVec<LittleEndian, u8> = BitVec::from(s);
            // println!("s_vec: {:?}, {}, ({})", &s_vec, s_vec.len(), count);
            encoding.extend(s_vec.iter());
            encoding.push(prev.unwrap());
            count = 1;
        } else {
            count += 1;
        }
    }

    encoding
}

fn rle_plus(raw: &BitVec<LittleEndian, u8>) -> BitVec<LittleEndian, u8> {
    let mut encoding = BitVec::new();

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
    for i in 1..raw.len() {
        if raw.get(i) != current {
            if count == 1 {
                // single bits are encoded as "1" bit
                encoding.push(true);
            } else {
                let mut v = [0u8; 5];
                let s = unsigned_varint::encode::u32(count, &mut v);
                let s_vec: BitVec<LittleEndian, u8> = BitVec::from(s);

                encoding.push(false);
                if s.len() == 1 && s[0].leading_zeros() > 3 {
                    encoding.push(true);
                    encoding.extend(s_vec.iter().skip(4));
                } else {
                    encoding.push(false);
                    encoding.extend(s_vec.iter());
                }
                count = 1;
            }
            current = raw.get(i);
        } else {
            count += 1;
        }
    }

    encoding
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_basics() {
        let cases = vec![
            (
                bitvec![LittleEndian; 0; 8],
                bitvec![LittleEndian;
                        0, 0, 0, 1, 0, 0, 0, 0,
                        0
                ],
            ),
            (
                bitvec![LittleEndian; 0, 0, 0, 0, 1, 0, 0, 0],
                bitvec![LittleEndian;
                        0, 0, 1, 0, 0, 0, 0, 0,
                        0, 1, 0, 0, 0, 0, 0, 0,
                        0, 1, 1, 1, 0, 0, 0, 0,
                        0, 0, 0
                ],
            ),
        ];

        for case in cases.into_iter() {
            assert_eq!(rle(&case.0), case.1);
        }
    }
}
