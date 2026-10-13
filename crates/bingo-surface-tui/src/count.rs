//! Compact counts share one precision wherever the surface reads them.

pub fn count(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..10_000 => format!("{}.{:01}k", n / 1_000, n % 1_000 / 100),
        10_000..1_000_000 => format!("{}k", n / 1_000),
        1_000_000..10_000_000 => format!("{}.{:01}M", n / 1_000_000, n % 1_000_000 / 100_000),
        _ => format!("{}M", n / 1_000_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rough_edges_counts_have_one_precision() {
        for (n, expected) in [
            (0, "0"),
            (12, "12"),
            (812, "812"),
            (999, "999"),
            (1_000, "1.0k"),
            (1_200, "1.2k"),
            (9_999, "9.9k"),
            (10_000, "10k"),
            (13_812, "13k"),
            (999_999, "999k"),
            (1_200_000, "1.2M"),
            (12_000_000, "12M"),
        ] {
            assert_eq!(count(n), expected);
        }
    }
}
