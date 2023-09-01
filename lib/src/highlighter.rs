use std::borrow::Cow;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent};
use tree_sitter_highlight::Highlighter as TsHighlighter;

use crate::{Language, Result};

pub struct Highlighter<'c> {
    language: &'static Language,
    highlights: Cow<'c, [&'c str]>,
    config: HighlightConfiguration,
    inner: TsHighlighter,
}

struct FusedEvents<'a, I> {
    highlights: &'a [&'a str],
    source: &'a str,
    events: I,
    done: bool
}

pub enum Highlight<'a> {
    Start {
        /// The name of the matched highlight.
        highlight: &'a str,
        /// The index of the matched highlight.
        index: usize,
    },
    Source {
        text: &'a str,
        start: usize,
        end: usize,
    },
    End,
}

impl<'c> Highlighter<'c> {
    pub fn new(language: &'static Language, highlights: &'c [&'c str]) -> Self {
        Self {
            language,
            highlights: highlights.into(),
            config: language.highlight_config(highlights),
            inner: TsHighlighter::new(),
        }
    }

    pub fn language(&self) -> &'static Language {
        self.language
    }

    pub fn highlight<'a>(
        &'a mut self,
        source: &'a str,
    ) -> impl Iterator<Item = Result<Highlight<'a>>> + 'a {
        let highlights = &*self.highlights;
        let events = self.inner.highlight(&self.config, source.as_bytes(), None, move |_| None);
        FusedEvents { highlights, source, events, done: false }
    }
}

impl<'a, I> Iterator for FusedEvents<'a, Result<I>>
    where I: Iterator<Item = Result<HighlightEvent>> + 'a
{
    type Item = Result<Highlight<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        use tree_sitter_highlight::Error;

        if self.done {
            return None;
        }

        match self.events {
            Ok(ref mut v) => v.next().map(|e| e.map(|v| match v {
                HighlightEvent::Source { start, end } => Highlight::Source {
                    text: &self.source[start..end],
                    start, end,
                },
                HighlightEvent::HighlightStart(h) => Highlight::Start {
                    highlight: &self.highlights[h.0],
                    index: h.0,
                },
                HighlightEvent::HighlightEnd => Highlight::End,
            })),
            Err(ref e) => {
                self.done = true;
                Some(Err(match e {
                    Error::Cancelled => Error::Cancelled,
                    Error::InvalidLanguage => Error::InvalidLanguage,
                    Error::Unknown => Error::Unknown,
                }))
            }
        }
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use tree_sitter::SerializationError;
    use tree_sitter_highlight::{HighlightConfiguration, SerializableHighlightConfig};
    use serde::{Serialize, Deserialize, Deserializer, de::Error};

    use super::*;

    type SerializationData<'a> = (Cow<'a, [&'a str]>, SerializableHighlightConfig);

    type DeserializationData<'a> = (Vec<&'a str>, SerializableHighlightConfig);

    impl<'c> Highlighter<'c> {
        pub fn serializable(self) -> Result<impl Serialize + 'c, SerializationError> {
            Ok((self.highlights, self.config.serializable()?) as SerializationData<'_>)
        }
    }

    impl<'de> Deserialize<'de> for Highlighter<'de> {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            let (highlights, config) = DeserializationData::deserialize(de)?;
            let language_name = &config.metadata.language_name;
            let language = Language::find_by_name(language_name)
                .ok_or_else(|| <D::Error>::custom(format!("missing language: {language_name}")))?;

            let config = HighlightConfiguration::deserialize(config, (language.language)())
                .map_err(|e| <D::Error>::custom(format!("{e:?}")))?;

            Ok(Highlighter {
                language,
                config,
                highlights: highlights.into(),
                inner: TsHighlighter::new(),
            })
        }
    }
}
