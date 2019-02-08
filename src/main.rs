use bitvec::*;
use croaring::Bitmap;
use rand::distributions::Uniform;
use rand::prelude::*;

fn main() {
    let mut rng = rand::thread_rng();

    // let sample_count = 100;

    // let mut raw_sizes = Vec::new();
    // let mut rle_sizes = Vec::new();
    // let mut roaring_sizes = Vec::new();

    for total_sectors in (1_000..1_000_000).step_by(10_000) {
        {
            println!("-- Random selections (up to 10%) --");
            // nothing selected
            let mut raw = bitvec![LittleEndian; 0u8; total_sectors];

            // select some, randomly
            let selected_sectors = rng.gen_range(1, total_sectors / 10);
            let sector_dist = Uniform::new(0, total_sectors);
            let mut bm = Bitmap::create();
            for j in 0..selected_sectors {
                let v = rng.sample(sector_dist);
                raw.set(v, true);
                bm.add(v as u32);
            }
            let rle_enc = rle(&raw);

            bm.run_optimize();

            println!("sectors: {}/{}", selected_sectors, total_sectors);
            println!("raw: {} bytes", raw.len() / 8);
            println!("rle: {} bytes", rle_enc.len() / 8);
            println!("roaring: {} bytes", bm.get_serialized_size_in_bytes());
        }

        {
            println!("-- Multiple contigous selections (each up to 3%) --");
            // nothing selected
            let mut raw = bitvec![LittleEndian; 0u8; total_sectors];
            let mut bm = Bitmap::create();
            // select some, randomly, but contigous
            let mut total_selected_sectors = 0;
            for j in 0..2 {
                let selected_sectors = rng.gen_range(1, total_sectors / 3);
                total_selected_sectors += selected_sectors;
                let start = rng.gen_range(0, total_sectors - selected_sectors);

                for i in start..start + selected_sectors {
                    raw.set(i, true);
                    bm.add(i as u32);
                }
            }
            let rle_enc = rle(&raw);

            bm.run_optimize();

            println!("sectors: {}/{}", total_selected_sectors, total_sectors);
            println!("raw: {} bytes", raw.len() / 8);
            println!("rle: {} bytes", rle_enc.len() / 8);
            println!("roaring: {} bytes", bm.get_serialized_size_in_bytes());
        }
    }
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
