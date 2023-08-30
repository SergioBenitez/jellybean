use chlorophyll::{LanguageSet, Highlighter, BASE_HIGHLIGHTS};

const SLOW_LANGUAGES: &[&str] = &[
    "nix", "racket", "glimmer", "bass", "scheme", "pascal", "svelte", "elixir",
    "yuck", "make",
    // "haskell", "ruby", "python", "php",
];

fn main() {
    let start = std::time::Instant::now();
    let serializable_highlighters = LanguageSet::ALL.iter()
        .filter(|lang| !SLOW_LANGUAGES.contains(&lang.name))
        .map(|language| language.highlighter(&BASE_HIGHLIGHTS))
        .map(|highlighter| highlighter.serializable())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("generation took {}ms", start.elapsed().as_millis());

    let se_start = std::time::Instant::now();
    let bytes = bincode::serialize(&serializable_highlighters).unwrap();
    println!("serialization took {}ms ({} bytes)", se_start.elapsed().as_millis(), bytes.len());

    let de_start = std::time::Instant::now();
    let highlighters: Vec<Highlighter<'_>> = bincode::deserialize(&bytes).unwrap();
    assert!(highlighters.len() == serializable_highlighters.len());
    println!("deserialization took {}ms", de_start.elapsed().as_millis());
    println!("complete round-trip time: {}ms", start.elapsed().as_millis());

    let mut results: Vec<(&str, u128)> = LanguageSet::ALL.iter()
        .filter(|lang| !SLOW_LANGUAGES.contains(&lang.name))
        .map(|language| language.highlighter(&BASE_HIGHLIGHTS))
        .map(|highlighter| highlighter.serializable().unwrap())
        .map(|data| bincode::serialize(&data).unwrap())
        .map(|bytes| {
            let start = std::time::Instant::now();
            let hl: Highlighter = bincode::deserialize(&bytes).unwrap();
            (hl.language().name, start.elapsed().as_micros())
        })
        .collect::<Vec<_>>();

    results.sort_by_key(|(_, duration)| *duration);
    for (language, microsecond) in results.iter().rev() {
        println!("{language} took {microsecond}us");
    }
}
