use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

#[non_exhaustive]
pub struct ScorerContext<'a, I, O, R = ()> {
    pub run_id: &'a str,
    pub sample_id: &'a str,
    pub trial_index: usize,
    pub cancel: &'a CancellationToken,
    pub metadata: &'a HashMap<String, Value>,
    pub input: &'a I,
    pub output: &'a O,
    pub reference: Option<&'a R>,
}

impl<'a, I, O, R> ScorerContext<'a, I, O, R> {
    pub fn new(input: &'a I, output: &'a O, reference: Option<&'a R>) -> Self {
        Self {
            run_id: "",
            sample_id: "",
            trial_index: 0,
            cancel: default_cancel(),
            metadata: empty_metadata(),
            input,
            output,
            reference,
        }
    }

    pub fn with_scope(
        run_id: &'a str,
        sample_id: &'a str,
        trial_index: usize,
        cancel: &'a CancellationToken,
        metadata: &'a HashMap<String, Value>,
        input: &'a I,
        output: &'a O,
        reference: Option<&'a R>,
    ) -> Self {
        Self {
            run_id,
            sample_id,
            trial_index,
            cancel,
            metadata,
            input,
            output,
            reference,
        }
    }
}

fn default_cancel() -> &'static CancellationToken {
    static DEFAULT: OnceLock<CancellationToken> = OnceLock::new();
    DEFAULT.get_or_init(CancellationToken::new)
}

fn empty_metadata() -> &'static HashMap<String, Value> {
    static EMPTY: OnceLock<HashMap<String, Value>> = OnceLock::new();

    EMPTY.get_or_init(HashMap::new)
}

#[cfg(test)]
mod tests {
    use super::ScorerContext;
    use std::collections::HashMap;

    #[test]
    fn scorer_context_holds_input_output_and_reference() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let reference = String::from("golden");
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        assert_eq!(ctx.input, &input);
        assert_eq!(ctx.output, &output);
        assert_eq!(ctx.reference, Some(&reference));
    }

    #[test]
    fn scorer_context_supports_absent_reference() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let ctx: ScorerContext<'_, String, String, String> =
            ScorerContext::new(&input, &output, None);

        assert!(ctx.reference.is_none());
    }

    #[test]
    fn scorer_context_defaults_reference_type_to_unit() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        assert_eq!(ctx.input, &input);
        assert_eq!(ctx.output, &output);
        assert!(ctx.reference.is_none());
        assert_eq!(ctx.run_id, "");
        assert_eq!(ctx.sample_id, "");
        assert_eq!(ctx.trial_index, 0);
        assert!(ctx.metadata.is_empty());
    }

    #[test]
    fn scorer_context_supports_non_string_generics() {
        let input = vec![1_u8, 2, 3];
        let output = 0.75_f64;
        let reference = true;
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        assert_eq!(ctx.input, &vec![1_u8, 2, 3]);
        assert_eq!(ctx.output, &0.75_f64);
        assert_eq!(ctx.reference, Some(&true));
    }

    #[test]
    fn scorer_context_can_carry_run_scope_and_metadata() {
        use tokio_util::sync::CancellationToken;
        let input = String::from("prompt");
        let output = String::from("answer");
        let metadata = HashMap::from([("topic".to_string(), serde_json::json!("math"))]);
        let cancel = CancellationToken::new();
        let ctx: ScorerContext<'_, String, String> =
            ScorerContext::with_scope("run-1", "sample-1", 2, &cancel, &metadata, &input, &output, None);

        assert_eq!(ctx.run_id, "run-1");
        assert_eq!(ctx.sample_id, "sample-1");
        assert_eq!(ctx.trial_index, 2);
        assert_eq!(ctx.metadata.get("topic"), Some(&serde_json::json!("math")));
    }

    #[test]
    fn scorer_context_default_cancel_is_not_cancelled() {
        let input = String::from("p");
        let output = String::from("a");
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);
        assert!(!ctx.cancel.is_cancelled());
    }
}
