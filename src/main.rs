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
            let mut roaring_sizes = Vec::new();
            let mut con_sizes = Vec::new();
            let mut gz_sizes = Vec::new();
            let mut zlib_sizes = Vec::new();

            println!("-- Random selections (up to {}%) --", part);
            // nothing selected
            let mut raw = bitvec![LittleEndian; 0u8; total_sectors];

            // select some, randomly
            let selected_sectors = rng.gen_range(1, total_sectors / part);
            let sector_dist = Uniform::new(0, total_sectors);
            let mut bm = Bitmap::create();
            let mut con = Concise::new();

            for _ in 0..selected_sectors {
                let v = rng.sample(sector_dist);
                raw.set(v, true);
                bm.add(v as u32);
                con.append(v as i32);
            }
            let rle_enc = rle(&raw);

            bm.run_optimize();

            let mut gz = GzEncoder::new(Vec::new(), Compression::best());
            gz.write_all(raw.as_ref()).unwrap();
            let gz_enc = gz.finish().unwrap();
            let mut zlib = ZlibEncoder::new(Vec::new(), Compression::best());
            zlib.write_all(raw.as_ref()).unwrap();
            let zlib_enc = zlib.finish().unwrap();

            raw_sizes.push(raw.as_ref().len());
            rle_sizes.push(rle_enc.as_ref().len());
            roaring_sizes.push(bm.get_serialized_size_in_bytes());
            con_sizes.push(con.size());
            gz_sizes.push(gz_enc.len());
            zlib_sizes.push(zlib_enc.len());

            let len = raw_sizes.len();
            // println!("random selection: ({} count)", len);
            println!("sectors: {}/{}", selected_sectors, total_sectors);
            // println!("raw:     {} bytes", raw.len() / 8);
            // println!("rle:     {} bytes", rle_enc.len() / 8);
            // println!("roaring: {} bytes", bm.get_serialized_size_in_bytes());
            // println!("concise: {} bytes", con.size());
            // println!("gz:      {} bytes", gz_enc.len());
            // println!("zlib:    {} bytes", zlib_enc.len());

            println!("  raw:     {} bytes (avg)", average(&raw_sizes));
            println!("  rle:     {} bytes (avg)", average(&rle_sizes));
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
                let mut roaring_sizes = Vec::new();
                let mut con_sizes = Vec::new();
                let mut gz_sizes = Vec::new();
                let mut zlib_sizes = Vec::new();

                println!(
                    "-- Multiple {} contigous selections (each up to {}%) --",
                    count, part
                );
                // nothing selected
                let mut raw = bitvec![LittleEndian; 0u8; total_sectors];
                let mut bm = Bitmap::create();
                let mut con = Concise::new();
                // select some, randomly, but contigous
                let mut total_selected_sectors = 0;
                for _ in 0..*count {
                    let selected_sectors = rng.gen_range(1, total_sectors / part);
                    total_selected_sectors += selected_sectors;
                    let start = rng.gen_range(0, total_sectors - selected_sectors);

                    for i in start..start + selected_sectors {
                        raw.set(i, true);
                        bm.add(i as u32);
                        con.append(i as i32);
                    }
                }
                let rle_enc = rle(&raw);

                bm.run_optimize();
                let mut gz = GzEncoder::new(Vec::new(), Compression::best());
                gz.write_all(raw.as_ref()).unwrap();
                let gz_enc = gz.finish().unwrap();
                let mut zlib = ZlibEncoder::new(Vec::new(), Compression::best());
                zlib.write_all(raw.as_ref()).unwrap();
                let zlib_enc = zlib.finish().unwrap();

                raw_sizes.push(raw.as_ref().len());
                rle_sizes.push(rle_enc.as_ref().len());
                roaring_sizes.push(bm.get_serialized_size_in_bytes());
                con_sizes.push(con.size());
                gz_sizes.push(gz_enc.len());
                zlib_sizes.push(zlib_enc.len());

                //
                // println!("raw:     {} bytes", raw.len() / 8);
                // println!("rle:     {} bytes", rle_enc.len() / 8);
                // println!("roaring: {} bytes", bm.get_serialized_size_in_bytes());
                // println!("concise: {} bytes", con.size());
                // println!("gz:      {} bytes", gz_enc.len());
                // println!("zlib:    {} bytes", zlib_enc.len());

                let len = raw_sizes.len();
                // println!("contingous slices: ({} count)", len);
                println!("sectors: {}/{}", total_selected_sectors, total_sectors);
                println!("  raw:     {} bytes (avg)", average(&raw_sizes));
                println!("  rle:     {} bytes (avg)", average(&rle_sizes));
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
    for i in 1..raw.len() {
        let prev = raw.get(i - 1);
        if raw.get(i) != prev {
            let mut v = [0u8; 5];
            let s = unsigned_varint::encode::u32(count, &mut v);
            let s_vec: BitVec<LittleEndian, u8> = BitVec::from(s);
            encoding.extend(s_vec.iter());
            encoding.push(prev);
            count = 1;
        } else {
            count += 1;
        }
    }

    encoding
}
