use crate::ResourceUsage;

#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Budget {
    pub max_cost_usd: Option<f64>,
    pub max_tokens: Option<u64>,
}

impl Budget {
    pub fn max_cost_usd(mut self, max_cost_usd: f64) -> Self {
        self.max_cost_usd = Some(max_cost_usd);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Whether `additional` resource usage on top of nothing would exceed any
    /// configured cap. Advisory only — the kernel does not enforce.
    pub fn would_exceed(&self, additional: &ResourceUsage) -> bool {
        if let Some(max_cost) = self.max_cost_usd {
            if let Some(cost) = additional.cost_usd {
                if cost > max_cost {
                    return true;
                }
            }
        }
        if let Some(max_tokens) = self.max_tokens {
            let total = additional.token_usage.input + additional.token_usage.output;
            if total > max_tokens {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TokenUsage;

    #[test]
    fn would_exceed_cost_cap() {
        let budget = Budget::default().max_cost_usd(0.10);
        let usage = ResourceUsage::default().cost_usd(0.15);
        assert!(budget.would_exceed(&usage));
    }

    #[test]
    fn would_not_exceed_below_cap() {
        let budget = Budget::default().max_cost_usd(0.10);
        let usage = ResourceUsage::default().cost_usd(0.05);
        assert!(!budget.would_exceed(&usage));
    }

    #[test]
    fn would_exceed_token_cap() {
        let budget = Budget::default().max_tokens(100);
        let usage = ResourceUsage::default()
            .token_usage(TokenUsage { input: 60, output: 60, cache_read: 0, cache_write: 0 });
        assert!(budget.would_exceed(&usage));
    }

    #[test]
    fn no_caps_never_exceeds() {
        let budget = Budget::default();
        let usage = ResourceUsage::default().cost_usd(99.0);
        assert!(!budget.would_exceed(&usage));
    }
}
