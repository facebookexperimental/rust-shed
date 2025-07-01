/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::BTreeSet;
use std::collections::HashSet;

use minibench::bench;
use minibench::elapsed;
use sorted_vector_map::SortedVectorSet;

static WORDS: &[&str] = &[
    "cat",
    "goldfish",
    "dog",
    "badger",
    "porcupine",
    "hedgehog",
    "terrapin",
    "bird",
    "squirrel",
    "wombat",
];

#[inline(never)]
fn consume<T>(_t: T) {}

fn build_set_data(start: usize, size: usize) -> Vec<String> {
    let mut index = 0;
    let mut data = Vec::with_capacity(size);
    for n in start.. {
        for word1 in WORDS.iter() {
            for word2 in WORDS.iter() {
                data.push(format!("{}.{}.{}", word1, n, word2));
                index += 1;
                if index >= size {
                    return data;
                }
            }
        }
    }
    unreachable!()
}

macro_rules! make_set_bench {
    ($name:ident, $set:ident, [ $(,)? ]) => {};
    ($name:ident, $set:ident, [ $(,)? $count:literal $( $counts:tt )* ]) => {
        let mut data = build_set_data(0, $count);
        bench(concat!(stringify!($name), " (", stringify!($count), ") build unordered"), || {
            elapsed(|| { consume(data.iter().cloned().collect::<$set<_>>()); })
        });

        data.sort();
        // Ordered build of the whole set in a single call to extend.
        bench(concat!(stringify!($name), " (", stringify!($count), ") build ordered full"), || {
            elapsed(|| { consume(data.iter().cloned().collect::<$set<_>>()); })
        });

        // Ordered build of the set in 100 chunks.
        bench(concat!(stringify!($name), " (", stringify!($count), ") build ordered chunks"), || {
            elapsed(|| {
                let mut set = $set::new();
                for chunk in data.chunks(($count / 100).max(10)) {
                    set.extend(chunk.iter().cloned());
                }
                consume(set);
            })
        });

        // Ordered build of the set in 100 chunks, where each chunk contains a
        // duplicate at the midpoint.
        bench(concat!(stringify!($name), " (", stringify!($count), ") build ordered chunks dup"), || {
            elapsed(|| {
                let mut set = $set::new();
                for chunk in data.chunks(($count / 100).max(10)) {
                    let mid = $count / 200;
                    set.extend(chunk.iter().take(mid).chain(chunk.iter().skip(mid - 1)).cloned());
                }
                consume(set);
            })
        });

        // Ordered build of the set by extending a single element at a time.
        // This simulates stream collection.
        bench(concat!(stringify!($name), " (", stringify!($count), ") build ordered single"), || {
            elapsed(|| {
                let mut set = $set::new();
                for item in data.iter().cloned() {
                    set.extend(Some(item));
                }
                consume(set);
            })
        });

        let set = data.iter().cloned().collect::<$set<_>>();
        bench(concat!(stringify!($name), " (", stringify!($count), ") iterate"), || {
            elapsed(|| set.iter().for_each(|item| { consume(item); }))
        });

        let mut contains_items = Vec::with_capacity(1000);
        for n in 0..10 {
            for word1 in WORDS.iter() {
                for word2 in WORDS.iter() {
                    contains_items.push(format!("{}.{}.{}", word1, n, word2));
                }
            }
        }
        bench(concat!(stringify!($name), " (", stringify!($count), ") contains-thousand"), || {
            elapsed(|| {
                for item in contains_items.iter() {
                    consume(set.contains(item));
                }
            })
        });

        let mut insert_items = Vec::with_capacity(1000);
        for n in 0..10 {
            for word1 in WORDS.iter() {
                for word2 in WORDS.iter() {
                    insert_items.push(format!("{}.{}.{}.additional", word1, n, word2));
                }
            }
        }
        bench(concat!(stringify!($name), " (", stringify!($count), ") insert-thousand"), || {
            let mut set = set.clone();
            elapsed(|| {
                for item in insert_items.iter().cloned() {
                    set.insert(item);
                }
            })
        });

        let more_data = build_set_data($count, $count);
        bench(concat!(stringify!($name), " (", stringify!($count), ") extend"), || {
            let mut set = set.clone();
            elapsed(|| {
                set.extend(more_data.iter().cloned());
            })
        });

        let mut data2 = build_set_data($count/200, $count);
        data2.sort();
        let set2 = data.iter().cloned().collect::<$set<_>>();
        bench(concat!(stringify!($name), " (", stringify!($count), ") intersection"), || {
            elapsed(|| {
                consume(set.intersection(&set2).count());
            })
        });

        bench(concat!(stringify!($name), " (", stringify!($count), ") difference"), || {
            elapsed(|| {
                consume(set.difference(&set2).count());
            })
        });

        bench(concat!(stringify!($name), " (", stringify!($count), ") symmetric_difference"), || {
            elapsed(|| {
                consume(set.symmetric_difference(&set2).count());
            })
        });

        bench(concat!(stringify!($name), " (", stringify!($count), ") union"), || {
            elapsed(|| {
                consume(set.union(&set2).count());
            })
        });

        make_set_bench!($name, $set, [$( $counts )*]);
    };
}

fn main() {
    make_set_bench!(sorted_vector_set, SortedVectorSet, [1000, 10000, 100000]);
    make_set_bench!(btreeset, BTreeSet, [1000, 10000, 100000]);
    make_set_bench!(hashset, HashSet, [1000, 10000, 100000]);
}
