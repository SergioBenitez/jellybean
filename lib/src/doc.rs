// This is called by `docs!()` above.
#[doc(hidden)]
#[macro_export]
macro_rules! doc_line {
    ($($m:ident, $name:literal, $desc:literal, [$($ft:literal),*])*) => {
        concat!($("| `", $name, "` | ", stringify!($($ft),*), " | ", $desc, " |\n"),*)
    }
}
