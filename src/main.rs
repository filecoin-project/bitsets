#![feature(wrapping_int_impl)]

#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate lazy_static;

use bitvec::*;
use croaring::Bitmap;
use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::Compression;
use prettytable::{format, Table};
use rand::distributions::Uniform;
use rand::prelude::*;
use std::io::prelude::*;

use bitsets::concise::Concise;
use bitsets::rleplus;

lazy_static! {
    static ref MARKDOWN_TABLE_FORMAT: format::TableFormat = format::FormatBuilder::new()
        .column_separator('|')
        .borders('|')
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('-', '|', '|', '|'),
        )
        .padding(1, 1)
        .build();
}

fn main() {
    bench_random();
    bench_cont();
}

fn bench_random() {
    let mut rng = rand::thread_rng();

    for &total_sectors in &[10_000, 100_000, 1_000_000] {
        for part in &[2, 5, 10] {
            let mut raw_sizes = Vec::new();
            let mut rle_sizes = Vec::new();
            let mut rle_plus_sizes = Vec::new();
            let mut roaring_sizes = Vec::new();
            let mut con_sizes = Vec::new();
            let mut gz_sizes = Vec::new();
            let mut zlib_sizes = Vec::new();

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
                let rle_plus_enc = rleplus::encode(&raw);

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

            println!(
                "## Random selections (up to {}% - ({}/{}))",
                part,
                average(&selected_sectors_vec),
                total_sectors
            );

            let mut table = Table::new();
            table.set_format(*MARKDOWN_TABLE_FORMAT);
            table.set_titles(row!["variant", "size (in bytes)", "reduction"]);
            table.add_row(row!["raw", r -> average(&raw_sizes), r -> diff(&raw_sizes, &raw_sizes)]);
            table.add_row(row!["rle", r -> average(&rle_sizes), r -> diff(&raw_sizes, &rle_sizes)]);
            table.add_row(
                row!["rle+", r -> average(&rle_plus_sizes), r -> diff(&raw_sizes, &rle_plus_sizes)],
            );
            table.add_row(row!["roaring", r -> average(&roaring_sizes), r -> diff(&raw_sizes, &roaring_sizes)]);
            table.add_row(
                row!["concise", r -> average(&con_sizes), r -> diff(&raw_sizes, &con_sizes)],
            );
            table.add_row(row!["gz", r -> average(&gz_sizes), r -> diff(&raw_sizes, &gz_sizes)]);
            table.add_row(
                row!["zlib", r -> average(&zlib_sizes), r -> diff(&raw_sizes, &zlib_sizes)],
            );

            table.printstd();
            println!("");
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
                    let rle_plus_enc = rleplus::encode(&raw);

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

                println!(
                    "## Multiple {} contigous selections (each up to {}% - ({}/{}))",
                    count,
                    part,
                    average(&total_selected_sectors_vec),
                    total_sectors
                );

                let mut table = Table::new();
                table.set_format(*MARKDOWN_TABLE_FORMAT);
                table.set_titles(row!["variant", "size (in bytes)", "reduction"]);
                table.add_row(
                    row!["raw", r -> average(&raw_sizes), r -> diff(&raw_sizes, &raw_sizes)],
                );
                table.add_row(
                    row!["rle", r -> average(&rle_sizes), r -> diff(&raw_sizes, &rle_sizes)],
                );
                table.add_row(
                row!["rle+", r -> average(&rle_plus_sizes), r -> diff(&raw_sizes, &rle_plus_sizes)],
            );
                table.add_row(row!["roaring", r -> average(&roaring_sizes), r -> diff(&raw_sizes, &roaring_sizes)]);
                table.add_row(
                    row!["concise", r -> average(&con_sizes), r -> diff(&raw_sizes, &con_sizes)],
                );
                table
                    .add_row(row!["gz", r -> average(&gz_sizes), r -> diff(&raw_sizes, &gz_sizes)]);
                table.add_row(
                    row!["zlib", r -> average(&zlib_sizes), r -> diff(&raw_sizes, &zlib_sizes)],
                );

                table.printstd();
                println!("");
            }
        }
    }
}

fn average(numbers: &[usize]) -> String {
    let avg = numbers.iter().sum::<usize>() as f32 / numbers.len() as f32;
    format!("{:.0}", avg)
}

fn diff(old: &[usize], new: &[usize]) -> String {
    let old_avg = old.iter().sum::<usize>() as f32 / old.len() as f32;
    let new_avg = new.iter().sum::<usize>() as f32 / new.len() as f32;

    let diff_abs = new_avg - old_avg;
    format!("{:.2}%", diff_abs / old_avg * 100.)
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
