use crate::Usage;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}
#[derive(Debug, PartialEq)]
pub struct Spent {
    pub input: u64,
    pub output: u64,
    pub cache_share: u64,
    pub usd: Option<f64>,
}
pub fn spent(usage: Usage, price: Option<Price>) -> Spent {
    let input = usage.input_total();
    Spent {
        input,
        output: usage.output_tokens,
        cache_share: ((u128::from(usage.cache_read_tokens) * 100) / u128::from(input.max(1)))
            as u64,
        usd: price.and_then(|price| cost(usage, price)),
    }
}
fn charge(tokens: u64, rate: Option<f64>) -> Option<f64> {
    if tokens == 0 {
        return Some(0.0);
    }
    rate.filter(|v| v.is_finite() && *v >= 0.0)
        .map(|rate| tokens as f64 * rate / 1_000_000.0)
}
fn cost(usage: Usage, price: Price) -> Option<f64> {
    Some(
        charge(usage.input_tokens, price.input)?
            + charge(usage.output_tokens, price.output)?
            + charge(usage.cache_read_tokens, price.cache_read)?
            + charge(usage.cache_write_tokens, price.cache_write)?,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prices_charge_disjoint_counts_and_never_guess_missing_rates() {
        let usage = Usage {
            input_tokens: 100_000,
            output_tokens: 10_000,
            cache_read_tokens: 900_000,
            ..Default::default()
        };
        let price = Price {
            input: Some(3.0),
            output: Some(15.0),
            cache_read: Some(0.3),
            cache_write: None,
        };
        let total = spent(usage, Some(price));
        assert_eq!(
            (total.input, total.output, total.cache_share),
            (1_000_000, 10_000, 90)
        );
        assert!((total.usd.unwrap() - 0.72).abs() < 0.00001);
        for (usage, price) in [
            (usage, None),
            (
                usage,
                Some(Price {
                    cache_read: None,
                    ..price
                }),
            ),
            (
                Usage {
                    cache_write_tokens: 1,
                    ..usage
                },
                Some(price),
            ),
        ] {
            assert_eq!(spent(usage, price).usd, None);
        }
    }
}

#[cfg(test)]
mod cache_write_tests {
    use super::*;
    #[test]
    fn catalogue_cache_write_prices_are_charged_when_present() {
        let price: Price = serde_json::from_value(serde_json::json!({"cacheWrite":1.25})).unwrap();
        let usage = Usage {
            cache_write_tokens: 1_000_000,
            ..Default::default()
        };
        assert_eq!(spent(usage, Some(price)).usd, Some(1.25));
    }
}
