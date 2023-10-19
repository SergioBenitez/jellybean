use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent};
use tree_sitter_highlight::Highlighter as TsHighlighter;

use crate::Language;

type Result<T, E = tree_sitter_highlight::Error> = std::result::Result<T, E>;

pub struct Highlighter {
    language: &'static Language,
    captures: Captures,
    config: Source<HighlightConfiguration>,
    inner: TsHighlighter,
    // TODO: Make injection configurable.
    // injector: Option<Box<dyn FnMut(&str) -> Option<&HighlightConfiguration>>>,
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

type Captures = Source<Vec<String>, &'static [&'static str]>;

/// Iterator of highlight events coupled with
struct FusedEvents<'a, I> {
    captures: &'a Captures,
    source: &'a str,
    events: I,
    done: bool,
}

impl Highlighter {
    pub(crate) fn new(
        language: &'static Language,
        config: impl Into<Source<HighlightConfiguration>>,
        captures: impl Into<Source<Vec<String>, &'static [&'static str]>>,
    ) -> Self {
        Self {
            language,
            config: config.into(),
            captures: captures.into(),
            inner: TsHighlighter::new(),
            // injector: None,
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
        let ts_config = self.config.inner();
        let events = self.inner.highlight(ts_config, source.as_bytes(), None, |name| {
            #[cfg(feature = "precached")] {
                Language::find(name).and_then(|l| l.highlighter().config.cached().copied())
            }

            #[cfg(not(feature = "precached"))] None
        });

        FusedEvents { captures, source, events, done: false }
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
                    start, end
                },
                HighlightEvent::HighlightStart(h) => Highlight::Start {
                    group: &self.captures.get(h.0).expect("have capture"),
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
    use serde::{Serialize, Serializer, Deserialize, Deserializer, de::Error};

    use super::*;

    type SerializationData = (Captures, SerializableHighlightConfig);

    impl Highlighter {
        pub fn serializable(self) -> Result<impl Serialize, SerializationError> {
            let config = match self.config {
                Source::Custom(config) => config,
                #[cfg(feature = "precached")]
                Source::Cached(_) => crate::dumps::fetch_config(self.language),
                #[cfg(not(feature = "precached"))]
                Source::Cached(_) => unreachable!(),
            };

            Ok((self.captures, config.serializable()?) as SerializationData)
        }
    }

    impl<'de> Deserialize<'de> for Highlighter {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            let (captures, config) = SerializationData::deserialize(de)?;
            let language_name = &config.metadata.language_name;
            let language = Language::find_by_name(language_name)
                .ok_or_else(|| <D::Error>::custom("missing language"))?;

            let config = HighlightConfiguration::deserialize(config, (language.language)())
                .map_err(|e| <D::Error>::custom(format!("{e:?}")))?;

            Ok(Highlighter::new(language, config, captures))
        }
    }

    impl<'de> Deserialize<'de> for Captures {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            <Vec<String>>::deserialize(de).map(Captures::Custom)
        }
    }

    impl Serialize for Captures {
        fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            match self {
                Captures::Custom(v) => v.serialize(ser),
                Captures::Cached(v) => v.serialize(ser),
            }
        }
    }
}

pub(crate) enum Source<A, B = &'static A> {
    Custom(A),
    Cached(B),
}

impl<A, B> Source<A, B> {
    pub fn cached(&self) -> Option<&B> {
        match self {
            Source::Custom(_) => None,
            Source::Cached(b) => Some(b),
        }
    }
}

impl<T> Source<T> {
    pub fn inner(&self) -> &T {
        match self {
            Source::Custom(v) => v,
            Source::Cached(v) => *v,
        }
    }
}

impl Captures {
    pub fn get(&self, i: usize) -> Option<&str> {
        match self {
            Self::Custom(v) => v.get(i).map(|v| v.as_str()),
            Self::Cached(v) => v.get(i).copied(),
        }
    }
}

impl<A> From<A> for Source<A, &'static A> {
    fn from(value: A) -> Self {
        Source::Custom(value)
    }
}

impl<A, B> From<B> for Source<A, B> {
    fn from(value: B) -> Self {
        Source::Cached(value)
    }
}
