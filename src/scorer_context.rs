#[non_exhaustive]
pub struct ScorerContext<'a, I, O, R = ()> {
    pub input: &'a I,
    pub output: &'a O,
    pub reference: Option<&'a R>,
}

impl<'a, I, O, R> ScorerContext<'a, I, O, R> {
    pub fn new(input: &'a I, output: &'a O, reference: Option<&'a R>) -> Self {
        Self { input, output, reference }
    }
}

#[cfg(test)]
mod tests {
    use super::ScorerContext;

    #[test]
    fn scorer_context_holds_input_output_and_reference() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let reference = String::from("golden");
        let ctx = ScorerContext {
            input: &input,
            output: &output,
            reference: Some(&reference),
        };

        assert_eq!(ctx.input, &input);
        assert_eq!(ctx.output, &output);
        assert_eq!(ctx.reference, Some(&reference));
    }

    #[test]
    fn scorer_context_supports_absent_reference() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let ctx: ScorerContext<'_, String, String, String> = ScorerContext {
            input: &input,
            output: &output,
            reference: None,
        };

        assert!(ctx.reference.is_none());
    }

    #[test]
    fn scorer_context_defaults_reference_type_to_unit() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let ctx: ScorerContext<'_, String, String> = ScorerContext {
            input: &input,
            output: &output,
            reference: None,
        };

        assert_eq!(ctx.input, &input);
        assert_eq!(ctx.output, &output);
        assert!(ctx.reference.is_none());
    }

    #[test]
    fn scorer_context_supports_non_string_generics() {
        let input = vec![1_u8, 2, 3];
        let output = 0.75_f64;
        let reference = true;
        let ctx = ScorerContext {
            input: &input,
            output: &output,
            reference: Some(&reference),
        };

        assert_eq!(ctx.input, &vec![1_u8, 2, 3]);
        assert_eq!(ctx.output, &0.75_f64);
        assert_eq!(ctx.reference, Some(&true));
    }
}
