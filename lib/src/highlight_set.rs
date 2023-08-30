// use tree_sitter_highlight::{HighlightConfiguration, SerializedHighlightConfig};
//
// use crate::{Language, LanguageSet, Highlighter};
//
// pub struct HighlighterSet {
//     pub(crate) languages: Vec<&'static Language>,
//     pub(crate) configs: Vec<HighlightConfiguration>
// }
//
// type SerializedElement<'a> = (&'a str, SerializedHighlightConfig);
//
// impl HighlighterSet {
//     pub(crate) fn with_capacity(n: usize) -> Self {
//         HighlighterSet { languages: Vec::with_capacity(n), configs: Vec::with_capacity(n) }
//     }
//
//     #[inline]
//     pub fn all(recognize: &[&str]) -> Self {
//         HighlighterSet::from_language_set(LanguageSet::ALL, recognize)
//     }
//
//     pub fn from_language_set(languages: &LanguageSet, recognize: &[&str]) -> Self {
//         let mut set = HighlighterSet::with_capacity(languages.len());
//         for def in languages.iter() {
//             set.languages.push(def);
//             set.configs.push(def.highlight_config(recognize));
//         }
//
//         set
//     }
//
//     pub fn language_set(&self) -> &LanguageSet {
//         LanguageSet::new(&self.languages)
//     }
//
//     pub fn find_by_name(&self, language_name: &str) -> Option<&HighlightConfiguration> {
//         self.language_set()
//             .position_by_name(language_name)
//             .map(|i| &self.configs[i])
//     }
//
//     pub fn highlighter(&self, language: &str) -> Option<Highlighter<'_>> {
//         self.find_by_name(language).map(Highlighter::from)
//     }
// }
//
// impl HighlighterSet {
//     pub fn serialize_with<S: serde::Serializer>(self, serializer: S) -> Result<S::Ok, S::Error> {
//         use serde::ser::SerializeSeq;
//
//         let mut s = serializer.serialize_seq(Some(self.languages.len()))?;
//         for (lang, config) in self.languages.iter().zip(self.configs.into_iter()) {
//             let element: SerializedElement = (lang.name, config.serialize().unwrap());
//             s.serialize_element(&element)?;
//         }
//
//         s.end()
//     }
// }
//
// impl<'de> serde::Deserialize<'de> for HighlighterSet {
//     fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
//         let data = Vec::<SerializedElement<'_>>::deserialize(de)?;
//         let mut set = HighlighterSet::with_capacity(data.len());
//
//         for (i, (lang, config)) in data.into_iter().enumerate() {
//             let def = match LanguageSet::ALL.get(i) {
//                 Some(def) if def.name == lang => def,
//                 _ => Language::find_by_name(lang).unwrap(),
//             };
//
//             let config = HighlightConfiguration::deserialize(config, (def.language)()).unwrap();
//             set.languages.push(def);
//             set.configs.push(config);
//         }
//
//         Ok(set)
//     }
// }
