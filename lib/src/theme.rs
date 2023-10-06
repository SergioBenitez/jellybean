use std::borrow::Cow;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Theme<T: 'static> {
    Static(Set<T>),
    Dynamic(Map<T>)
}

type Set<T> = &'static [(&'static str, T)];

type Map<T> = BTreeMap<Cow<'static, str>, T>;

impl<T> Theme<T> {
    pub const fn new(set: &'static [(&'static str, T)]) -> Self {
        let mut i = 1;
        while i < set.len() {
            let (a, b) = (set[i - 1].0, set[i].0);
            if crate::util::const_compare(a.as_bytes(), b.as_bytes()).is_gt() {
                panic!("theme set must be sorted by capture name");
            }

            if crate::util::const_compare(a.as_bytes(), b.as_bytes()).is_eq() {
                panic!("theme set cannot contain duplicate captures");
            }

            i += 1;
        }

        Theme::Static(set)
    }

    pub fn find_exact(&self, capture: &str) -> Option<&T> {
        match self {
            Theme::Dynamic(map) => map.get(capture),
            Theme::Static(list) => {
                list.binary_search_by_key(&capture, |(name, _)| name).ok()
                    .and_then(|i| list.get(i))
                    .map(|(_name, item)| item)
            }
        }
    }

    pub fn find(&self, capture: &str) -> Option<&T> {
        fn _find<'a, T, S, F>(capture: &str, set: &'a S, getter: F) -> Option<&'a T>
            where F: Fn(&'a S, &str) -> Option<&'a T>
        {
            let mut candidate = capture;
            loop {
                if capture.is_empty() {
                    return None;
                }

                if let Some(value) = getter(set, candidate) {
                    return Some(value);
                }

                candidate = &candidate[..candidate.rfind('.')?];
            }
        }

        match self {
            Theme::Dynamic(map) => _find(capture, map, |map, k| map.get(k)),
            Theme::Static(list) => _find(capture, list, |list, k| {
                list.binary_search_by_key(&k, |(name, _)| name).ok()
                    .and_then(|i| list.get(i))
                    .map(|(_name, item)| item)
            }),
        }
    }
}

impl<S, T> FromIterator<(S, T)> for Theme<T>
    where S: Into<Cow<'static, str>>
{
    fn from_iter<I: IntoIterator<Item = (S, T)>>(iter: I) -> Self {
        let map = iter.into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();

        Self::Dynamic(map)
    }
}
