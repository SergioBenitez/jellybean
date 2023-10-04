use jellybean::{Language, Highlighter, COMMON_CAPTURES};
use rayon::prelude::*;

const SLOW_LANGUAGES: &[&str] = &[
    "racket", "nix", "bass", "scheme", "perl", "make", "pascal", "elixir",
    "glimmer",
    // "svelte", "haskell", "ruby", "python", "php",
];

fn main() {
    println!("-- skipped languages --");
    SLOW_LANGUAGES.iter().for_each(|s| println!("{s}"));

    println!("\n-- divided serialization --");
    run_split();

    println!("\n-- one big blob serialization --");
    run_big_blob();

    println!("\n-- per language timings (top 25) --");
    run_per_language(25);
}

fn run_split() {
    let start = std::time::Instant::now();
    let serializable_highlighters = Language::ALL.par_iter()
        .filter(|lang| !SLOW_LANGUAGES.contains(&lang.name))
        .map(|language| language.highlighter(COMMON_CAPTURES))
        .map(|highlighter| highlighter.serializable())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("generation took {}ms", start.elapsed().as_millis());

    let se_start = std::time::Instant::now();
    let serialized = serializable_highlighters.par_iter()
        .map(|hl| bincode::serialize(&hl))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("parallel serialization took {}us", se_start.elapsed().as_micros());

    let de_start = std::time::Instant::now();
    let highlighters: Vec<Highlighter<'_>> = serialized.par_iter()
        .map(|bytes| bincode::deserialize(&bytes))
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(highlighters.len() == serializable_highlighters.len());
    println!("parallel deserialization took {}us", de_start.elapsed().as_micros());
    println!("complete round-trip time: {}ms", start.elapsed().as_millis());
}

fn run_big_blob() {
    let start = std::time::Instant::now();
    let serializable_highlighters = Language::ALL.par_iter()
        .filter(|lang| !SLOW_LANGUAGES.contains(&lang.name))
        .map(|language| language.highlighter(COMMON_CAPTURES))
        .map(|highlighter| highlighter.serializable())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("generation took {}ms", start.elapsed().as_millis());

    let se_start = std::time::Instant::now();
    let bytes = bincode::serialize(&serializable_highlighters).unwrap();
    let len = bytes.len() >> 10;
    println!("serialization took {}us ({} KiB)", se_start.elapsed().as_micros(), len);

    let de_start = std::time::Instant::now();
    let highlighters: Vec<Highlighter<'_>> = bincode::deserialize(&bytes).unwrap();
    assert!(highlighters.len() == serializable_highlighters.len());
    println!("deserialization took {}us", de_start.elapsed().as_micros());
    println!("complete round-trip time: {}ms", start.elapsed().as_millis());
}

fn run_per_language(top: usize) {
    let mut results: Vec<(_, u128)> = Language::ALL.iter()
        .map(|language| language.highlighter(COMMON_CAPTURES))
        .map(|highlighter| highlighter.serializable().unwrap())
        .map(|data| bincode::serialize(&data).unwrap())
        .map(|bytes| {
            let start = std::time::Instant::now();
            let hl: Highlighter = bincode::deserialize(&bytes).unwrap();
            (hl.into_owned(), start.elapsed().as_micros())
        })
        .collect::<Vec<_>>();

    results.sort_by_key(|(_, duration)| *duration);
    for (i, (hl, microsecond)) in results.iter().rev().take(top).enumerate() {
        let i = i + 1;
        println!("{i:>2} {} took {microsecond}us", hl.language().name);
    }
}
