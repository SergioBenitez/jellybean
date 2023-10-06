use jellybean::tree_sitter_highlight::{SerializableHighlightConfig, HighlightConfiguration};
use jellybean::{Highlighter, COMMON_CAPTURES, ALL_LANGUAGES};
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
    let serializable_highlighters = ALL_LANGUAGES.par_iter()
        .filter(|lang| !SLOW_LANGUAGES.contains(&lang.name()))
        .map(|language| language.custom_highlighter(COMMON_CAPTURES))
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
    let serializable_highlighters = ALL_LANGUAGES.par_iter()
        .filter(|lang| !SLOW_LANGUAGES.contains(&lang.name()))
        .map(|language| language.custom_highlighter(COMMON_CAPTURES))
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
    let mut results: Vec<(_, _, u128)> = ALL_LANGUAGES.iter()
        .map(|lang| (lang, lang.highlight_config(COMMON_CAPTURES)))
        .map(|(lang, hl)| (lang, hl.serializable().unwrap()))
        .map(|(lang, data)| (lang, bincode::serialize(&data).unwrap()))
        .map(|(lang, bytes)| {
            let start = std::time::Instant::now();
            let dump: SerializableHighlightConfig = bincode::deserialize(&bytes).unwrap();
            let de_time = start.elapsed().as_micros();

            let start = std::time::Instant::now();
            let hl = HighlightConfiguration::deserialize(dump, lang.raw()).unwrap();
            assert_eq!(hl.language_name, lang.name());

            (lang.name(), de_time, start.elapsed().as_micros())
        })
        .collect::<Vec<_>>();

    results.sort_by_key(|(_, de_time, c_time)| de_time + c_time);
    for (i, (name, de, c)) in results.iter().rev().take(top).enumerate() {
        let i = i + 1;
        let total = de + c;
        println!("{i:>2} {name} took {total}us ({de}us de. / {c}us comp.)");
    }
}
