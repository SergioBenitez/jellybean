use std::cmp::Ordering;

// pub const fn const_compare(lhs: &[u8], rhs: &[u8]) -> Ordering {
//     let lhs_len = lhs.len();
//     let rhs_len = rhs.len();
//     let min_len = if lhs_len < rhs_len { lhs_len } else { rhs_len };
//
//     let mut i = 0;
//     while i < min_len {
//         if lhs[i] < rhs[i] {
//             return Ordering::Less;
//         }
//         if lhs[i] > rhs[i] {
//             return Ordering::Greater;
//         }
//         i += 1;
//     }
//
//     if lhs_len < rhs_len {
//         Ordering::Less
//     } else if lhs_len > rhs_len {
//         Ordering::Greater
//     } else {
//         Ordering::Equal
//     }
// }

pub fn cmp_ignore_case_ascii(a: &str, b: &str) -> Ordering {
    for (a, b) in a.bytes().zip(b.bytes()) {
        match a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => continue,
        }
    }

    a.len().cmp(&b.len())
}
