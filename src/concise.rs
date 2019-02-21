#![allow(overflowing_literals)]

use std::cmp;
use std::num::Wrapping;

#[derive(Debug, Clone)]
pub struct Concise {
    words: Option<Vec<Wrapping<i32>>>,
    last: i32,
    size: i32,
    last_word_index: i32,
}

const MAX_LITERAL_LENGTH: i32 = 31;
const ALL_ZEROS_LITERAL: Wrapping<i32> = Wrapping(0x80000000);
const ALL_ONES_LITERAL: Wrapping<i32> = Wrapping(0xFFFFFFFF);
const SEQUENCE_BIT: Wrapping<i32> = Wrapping(0x40000000);

impl Concise {
    pub fn new() -> Concise {
        return Concise {
            words: None,
            last: -1,
            size: 0,
            last_word_index: -1,
        };
    }

    pub fn words_view(&self) -> &[Wrapping<i32>] {
        &self.words.as_ref().unwrap()[0..=self.last_word_index as usize]
    }

    pub fn size(&self) -> usize {
        (self.words_view().len() * 32) / 8
    }

    pub fn append(&mut self, i: i32) {
        if self.words.is_none() {
            let zero_blocks = i / 31;
            if zero_blocks == 0 {
                self.words = Some(vec![Wrapping(0); 1]);
                self.last_word_index = 0;
            } else if zero_blocks == 1 {
                self.words = Some(vec![Wrapping(0); 2]);
                self.last_word_index = 1;
                self.words.as_mut().unwrap()[0] = ALL_ZEROS_LITERAL;
            } else {
                self.words = Some(vec![Wrapping(0); 2]);
                self.last_word_index = 1;
                self.words.as_mut().unwrap()[0] = Wrapping(zero_blocks - 1);
            }
            self.last = i;
            self.size = 1;
            self.words.as_mut().unwrap()[self.last_word_index as usize] =
                ALL_ZEROS_LITERAL | Wrapping(1 << (i % 31));
            return;
        }

        let mut bit = self.last % 31 + i - self.last;

        if bit >= MAX_LITERAL_LENGTH {
            let zero_blocks = bit / 31 - 1;
            bit %= 31;
            if zero_blocks == 0 {
                self.ensure_capacity((self.last_word_index + 1) as usize);
            } else {
                self.ensure_capacity((self.last_word_index + 2) as usize);
                self.append_fill(Wrapping(zero_blocks), Wrapping(0));
            }
            self.append_literal(ALL_ZEROS_LITERAL | Wrapping(1i32.wrapping_shl(bit as u32)));
        } else {
            self.words.as_mut().unwrap()[self.last_word_index as usize] |=
                Wrapping(1i32.wrapping_shl(bit as u32));
            if self.words.as_mut().unwrap()[self.last_word_index as usize] == ALL_ONES_LITERAL {
                self.last_word_index -= 1;
                self.append_literal(ALL_ONES_LITERAL);
            }
        }

        self.last = i;
        if self.size >= 0 {
            self.size += 1;
        }
    }

    fn ensure_capacity(&mut self, index: usize) {
        let mut capacity = if self.words.is_none() {
            0
        } else {
            self.words.as_mut().unwrap().len()
        };
        if capacity > index {
            return;
        }
        capacity = cmp::max(capacity << 1, index + 1);

        // XXX: This is probably inefficient
        if self.words.is_none() {
            self.words = Some(vec![Wrapping(0); capacity]);
            return;
        }
        let mut new_words = vec![Wrapping(0i32); capacity];
        for (i, word) in self.words.as_mut().unwrap().iter().enumerate() {
            new_words[i] = *word;
        }
        self.words = Some(new_words);
    }

    fn append_fill(&mut self, length: Wrapping<i32>, mut fill_type: Wrapping<i32>) {
        // XXX: Are these really necessary?
        assert!(length > Wrapping(0));
        assert!(self.last_word_index >= -1);

        fill_type &= SEQUENCE_BIT;

        if length == Wrapping(1) {
            self.append_literal(if fill_type == Wrapping(0) {
                ALL_ZEROS_LITERAL
            } else {
                ALL_ONES_LITERAL
            });
            return;
        }

        if self.last_word_index < 0 {
            self.words.as_mut().unwrap()[self.last_word_index as usize] =
                fill_type | (length - Wrapping(1));
            return;
        }

        let last_word = self.words.as_mut().unwrap()[self.last_word_index as usize];
        if self.is_literal(last_word) {
            if fill_type == Wrapping(0) && last_word == ALL_ZEROS_LITERAL {
                self.words.as_mut().unwrap()[self.last_word_index as usize] = length;
            } else if fill_type == SEQUENCE_BIT && last_word == ALL_ONES_LITERAL {
                self.words.as_mut().unwrap()[self.last_word_index as usize] = SEQUENCE_BIT | length;
            } else {
                if fill_type == Wrapping(0)
                    && self.contains_only_one_bit(self.get_literal_bits(last_word))
                {
                    self.words.as_mut().unwrap()[self.last_word_index as usize] =
                        length | Wrapping((1 + last_word.trailing_zeros() as i32) << 25);
                } else if fill_type == SEQUENCE_BIT && self.contains_only_one_bit(!last_word) {
                    self.words.as_mut().unwrap()[self.last_word_index as usize] = SEQUENCE_BIT
                        | length
                        | Wrapping((1 + (!last_word).trailing_zeros() as i32) << 25);
                } else {
                    self.last_word_index += 1;
                    self.words.as_mut().unwrap()[self.last_word_index as usize] =
                        fill_type | (length - Wrapping(1));
                }
            }
        } else {
            if last_word & Wrapping(0xC0000000) == fill_type {
                self.words.as_mut().unwrap()[self.last_word_index as usize] += length;
            } else {
                self.last_word_index += 1;
                self.words.as_mut().unwrap()[self.last_word_index as usize] =
                    fill_type | (length - Wrapping(1));
            }
        }
    }

    fn append_literal(&mut self, word: Wrapping<i32>) {
        if self.last_word_index == 0
            && word == ALL_ZEROS_LITERAL
            && self.words.as_mut().unwrap()[0] == Wrapping(0x01FFFFFF)
        {
            return;
        }

        if self.last_word_index < 0 {
            self.last_word_index = 0;
            self.words.as_mut().unwrap()[self.last_word_index as usize] = word;
            return;
        }

        let last_word = self.words.as_mut().unwrap()[self.last_word_index as usize];
        if word == ALL_ZEROS_LITERAL {
            if last_word == ALL_ZEROS_LITERAL {
                self.words.as_mut().unwrap()[self.last_word_index as usize] = Wrapping(1);
            } else if self.is_zero_sequence(last_word) {
                self.words.as_mut().unwrap()[self.last_word_index as usize] += Wrapping(1);
            } else if self.contains_only_one_bit(self.get_literal_bits(last_word)) {
                self.words.as_mut().unwrap()[self.last_word_index as usize] =
                    Wrapping(1 | ((1 + last_word.trailing_zeros() as i32) << 25));
            } else {
                self.last_word_index += 1;
                self.words.as_mut().unwrap()[self.last_word_index as usize] = word;
            }
        } else if word == ALL_ONES_LITERAL {
            if last_word == ALL_ONES_LITERAL {
                self.words.as_mut().unwrap()[self.last_word_index as usize] =
                    SEQUENCE_BIT | Wrapping(1);
            } else if self.is_one_sequence(last_word) {
                self.words.as_mut().unwrap()[self.last_word_index as usize] += Wrapping(1);
            } else if self.contains_only_one_bit(!last_word) {
                self.words.as_mut().unwrap()[self.last_word_index as usize] = SEQUENCE_BIT
                    | Wrapping(1)
                    | Wrapping((1 + (!last_word).trailing_zeros() as i32) << 25);
            } else {
                self.last_word_index += 1;
                self.words.as_mut().unwrap()[self.last_word_index as usize] = word;
            }
        } else {
            self.last_word_index += 1;
            self.words.as_mut().unwrap()[self.last_word_index as usize] = word;
        }
    }

    fn is_zero_sequence(&self, word: Wrapping<i32>) -> bool {
        return (word & Wrapping(0xC0000000)) == Wrapping(0);
    }

    fn is_one_sequence(&self, word: Wrapping<i32>) -> bool {
        return (word & Wrapping(0xC0000000)) == SEQUENCE_BIT;
    }

    fn is_literal(&self, word: Wrapping<i32>) -> bool {
        return (word & Wrapping(0x80000000)) != Wrapping(0);
    }

    fn contains_only_one_bit(&self, literal: Wrapping<i32>) -> bool {
        return (literal & (literal - Wrapping(1))) == Wrapping(0);
    }

    fn get_literal_bits(&self, word: Wrapping<i32>) -> Wrapping<i32> {
        return Wrapping(0x7FFFFFFF) & word;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_iterator_next1() {
        let mut concise = Concise::new();
        for i in 1..=5 {
            concise.append(i);
        }

        let words = concise.words.unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0], Wrapping(0x8000003E));
    }

    #[test]
    fn word_iterator_next2() {
        let mut concise = Concise::new();
        for i in 0..100000 {
            concise.append(i);
        }

        let words = concise.words.unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0], Wrapping(0x40000C98));
        assert_eq!(words[1], Wrapping(0x81FFFFFF));
    }
}
