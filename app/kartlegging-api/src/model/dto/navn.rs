use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;

#[derive(Eq)]
pub struct Navn {
    fornavn: Option<String>,
    mellomnavn: Option<String>,
    etternavn: Option<String>,
}

impl Navn {
    pub fn new(fornavn: Option<String>, mellomnavn: Option<String>, etternavn: Option<String>) -> Self {
        Self {
            fornavn: fornavn.and_then(Self::normalize),
            mellomnavn: mellomnavn.and_then(Self::normalize),
            etternavn: etternavn.and_then(Self::normalize),
        }
    }

    fn normalize(value: String) -> Option<String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    }

    pub fn fornavn(&self) -> Option<String> {
        self.fornavn.clone()
    }

    pub fn mellomnavn(&self) -> Option<String> {
        self.mellomnavn.clone()
    }

    pub fn etternavn(&self) -> Option<String> {
        self.etternavn.clone()
    }

    fn fornavn_eq(&self, other: &Option<String>) -> bool {
        self.optional_string_eq(&self.fornavn, other)
    }

    fn mellomnavn_eq(&self, other: &Option<String>) -> bool {
        self.optional_string_eq(&self.mellomnavn, other)
    }

    fn etternavn_eq(&self, other: &Option<String>) -> bool {
        self.optional_string_eq(&self.etternavn, other)
    }

    fn optional_string_eq(&self, left: &Option<String>, right: &Option<String>) -> bool {
        left.as_deref().map(str::to_lowercase) == right.as_deref().map(str::to_lowercase)
    }
}

impl Default for Navn {
    fn default() -> Self {
        Self {
            fornavn: None,
            mellomnavn: None,
            etternavn: None,
        }
    }
}

impl PartialEq for Navn {
    fn eq(&self, other: &Self) -> bool {
        self.fornavn_eq(&other.fornavn)
                && self.mellomnavn_eq(&other.mellomnavn)
                && self.etternavn_eq(&other.etternavn)
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl From<&ArbeidssoekerRow> for Navn {
    fn from(row: &ArbeidssoekerRow) -> Self {
        Self::new(row.fornavn.clone(), row.mellomnavn.clone(), row.etternavn.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::Navn;

    fn navn(fields: [Option<&str>; 3]) -> Navn {
        let [fornavn, mellomnavn, etternavn] = fields.map(|value| value.map(str::to_owned));
        Navn {
            fornavn,
            mellomnavn,
            etternavn,
        }
    }

    fn assert_equality(left: &Navn, right: &Navn, expected: bool) {
        assert_eq!(left == right, expected);
        assert_eq!(right == left, expected);
        assert_eq!(left != right, !expected);
        assert_eq!(right != left, !expected);
        assert!(left.eq(left));
        assert!(right.eq(right));
    }

    #[test]
    fn new_normalizes_all_fields() {
        let cases = [
            ("Anne", Some("Anne")),
            (" Anne ", Some("Anne")),
            ("\tAnne\r\n", Some("Anne")),
            ("", None),
            (" ", None),
            ("\t\r\n", None),
            ("\u{a0}", None),
            ("\u{a0}Ægir\u{a0}", Some("Ægir")),
            ("  Anne  Marie  ", Some("Anne  Marie")),
            ("  æØÅ  ", Some("æØÅ")),
        ];

        for (input, expected) in cases {
            let actual = Navn::new(Some(input.to_owned()), Some(input.to_owned()), Some(input.to_owned()));
            assert_eq!(actual.fornavn().as_deref(), expected, "input: {input:?}");
            assert_eq!(actual.mellomnavn().as_deref(), expected, "input: {input:?}");
            assert_eq!(actual.etternavn().as_deref(), expected, "input: {input:?}");
        }
    }

    #[test]
    fn new_preserves_missing_middle_name() {
        let actual = Navn::new(Some(" Anne ".to_owned()), None, Some(" Hansen ".to_owned()));
        assert_eq!(actual.fornavn().as_deref(), Some("Anne"));
        assert_eq!(actual.mellomnavn(), None);
        assert_eq!(actual.etternavn().as_deref(), Some("Hansen"));
    }

    #[test]
    fn new_with_empty_fields_equals_default() {
        let actual = Navn::new(Some(" ".to_owned()), Some("\t".to_owned()), Some(String::new()));
        assert_equality(&actual, &Navn::default(), true);
    }

    #[test]
    fn new_normalizes_before_case_insensitive_comparison() {
        let left = Navn::new(
            Some(" Ægir ".to_owned()),
            Some(" Ørn ".to_owned()),
            Some(" Ås ".to_owned()),
        );
        let right = Navn::new(Some("æGIR".to_owned()), Some("øRN".to_owned()), Some("åS".to_owned()));
        assert_equality(&left, &right, true);
    }

    #[test]
    fn equality_checks_all_option_combinations() {
        for left_mask in 0..8 {
            for right_mask in 0..8 {
                let left = navn(std::array::from_fn(|index| {
                    (left_mask & (1 << index) != 0).then_some("Ægir")
                }));
                let right = navn(std::array::from_fn(|index| {
                    (right_mask & (1 << index) != 0).then_some("æGIR")
                }));
                assert_equality(&left, &right, left_mask == right_mask);
            }
        }
    }

    #[test]
    fn equality_checks_each_field_independently() {
        let cases = [
            (None, None, true),
            (None, Some(""), false),
            (None, Some("Anne"), false),
            (Some(""), Some(""), true),
            (Some("Anne"), Some("Anne"), true),
            (Some("anne"), Some("ANNE"), true),
            (Some("Anne"), Some("aNnE"), true),
            (Some("ÆØÅ"), Some("æøå"), true),
            (Some("Anne"), Some("Anna"), false),
            (Some(""), Some("Anne"), false),
            (Some(" Anne"), Some("Anne"), false),
            (Some("Anne "), Some("Anne"), false),
        ];

        for field in 0..3 {
            for (left_value, right_value, expected) in cases {
                let mut left = [Some("Hansen"); 3];
                let mut right = left;
                left[field] = left_value;
                right[field] = right_value;
                assert_eq!(
                    navn(left) == navn(right),
                    expected,
                    "field: {field}, left: {left_value:?}, right: {right_value:?}"
                );
                assert_equality(&navn(left), &navn(right), expected);
            }
        }
    }

    #[test]
    fn case_insensitive_equality_is_transitive() {
        let lower = navn([Some("æøå"); 3]);
        let upper = navn([Some("ÆØÅ"); 3]);
        let mixed = navn([Some("ÆøÅ"); 3]);
        assert_equality(&lower, &upper, true);
        assert_equality(&upper, &mixed, true);
        assert_equality(&lower, &mixed, true);
    }
}
