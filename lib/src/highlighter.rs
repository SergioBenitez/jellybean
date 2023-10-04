use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent};
use tree_sitter_highlight::Highlighter as TsHighlighter;

use crate::Language;

type Result<T, E = tree_sitter_highlight::Error> = std::result::Result<T, E>;

pub struct Highlighter<'a> {
    language: &'static Language,
    captures: Captures<'a>,
    config: HighlightConfiguration,
    inner: TsHighlighter,
}

#[derive(Debug)]
pub enum Highlight<'a> {
    Start {
        /// The name of the matched highlight.
        group: &'a str,
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

/// Array of capture groups to match during highlighting.
#[derive(Debug)]
enum Captures<'a> {
    Owned(Vec<String>),
    Borrowed(&'a [&'a str]),
    #[cfg_attr(not(feature = "serde"), allow(dead_code))]
    Partial(Vec<&'a str>),
}

/// Iterator of highlight events coupled with
struct FusedEvents<'a, I> {
    captures: &'a Captures<'a>,
    source: &'a str,
    events: I,
    done: bool
}

impl<'c> Highlighter<'c> {
    pub fn new(language: &'static Language, captures: &'c [&'c str]) -> Self {
        Self {
            language,
            captures: Captures::Borrowed(captures),
            config: language.highlight_config(captures),
            inner: TsHighlighter::new(),
        }
    }

    pub fn into_owned(self) -> Highlighter<'static> {
        Highlighter {
            language: self.language,
            captures: self.captures.into_owned(),
            config: self.config,
            inner: self.inner,
        }
    }

    pub fn language(&self) -> &'static Language {
        self.language
    }

    pub fn highlight<'a>(
        &'a mut self,
        source: &'a str,
    ) -> impl Iterator<Item = Result<Highlight<'a>>> + 'a {
        let captures = &self.captures;
        let events = self.inner.highlight(&self.config, source.as_bytes(), None, move |_| None);
        FusedEvents { captures, source, events, done: false }
    }
}

impl Captures<'_> {
    fn into_owned(self) -> Captures<'static> {
        match self {
            Captures::Owned(v) => Captures::Owned(v),
            Captures::Borrowed(s) => {
                Captures::Owned(s.into_iter().map(|s| s.to_string()).collect())
            }
            Captures::Partial(v) => {
                Captures::Owned(v.into_iter().map(|s| s.to_string()).collect())
            }
        }
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
                    group: &self.captures[h.0],
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

impl<'a> std::ops::Index<usize> for Captures<'a> {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Captures::Owned(v) => &v[index],
            Captures::Borrowed(s) => &s[index],
            Captures::Partial(v) => &v[index],
        }
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use tree_sitter::SerializationError;
    use tree_sitter_highlight::{HighlightConfiguration, SerializableHighlightConfig};
    use serde::{Serialize, Serializer, Deserialize, Deserializer, de::Error};

    use super::*;

    type SerializationData<'a> = (Captures<'a>, SerializableHighlightConfig);

    impl<'c> Highlighter<'c> {
        pub fn serializable(self) -> Result<impl Serialize + 'c, SerializationError> {
            Ok((self.captures, self.config.serializable()?) as SerializationData<'_>)
        }
    }

    impl<'de> Deserialize<'de> for Highlighter<'de> {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            let (captures, config) = SerializationData::deserialize(de)?;
            let language_name = &config.metadata.language_name;
            let language = Language::find_by_name(language_name)
                .ok_or_else(|| <D::Error>::custom(format!("missing language: {language_name}")))?;

            let config = HighlightConfiguration::deserialize(config, (language.language)())
                .map_err(|e| <D::Error>::custom(format!("{e:?}")))?;

            Ok(Highlighter {
                language,
                config,
                captures: captures.into(),
                inner: TsHighlighter::new(),
            })
        }
    }

    impl<'de> Deserialize<'de> for Captures<'de> {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            <Vec<&'de str>>::deserialize(de).map(Captures::Partial)
        }
    }

    impl Serialize for Captures<'_> {
        fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            match self {
                Captures::Owned(v) => v.serialize(ser),
                Captures::Borrowed(v) => v.serialize(ser),
                Captures::Partial(v) => v.serialize(ser),
            }
        }
    }
}
