use std::borrow::Cow;

use tree_sitter::SerializationError;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, SerializedHighlightConfig};
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

    pub fn serializable(self) -> Result<impl serde::Serialize + 'c, SerializationError> {
        Ok((self.language.name, self.highlights, self.config.serialize()?))
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

impl<'de> serde::Deserialize<'de> for Highlighter<'de> {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        type Data<'a> = (&'a str, Vec<&'a str>, SerializedHighlightConfig);

        let (language, highlights, config): Data<'_> = Data::deserialize(de)?;
        let language = Language::find_by_name(language).unwrap();
        let config = HighlightConfiguration::deserialize(config, (language.language)()).unwrap();
        Ok(Highlighter {
            language,
            config,
            highlights: highlights.into(),
            inner: TsHighlighter::new(),
        })
    }
}
