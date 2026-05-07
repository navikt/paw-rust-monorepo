pub struct ArbeidssoekerId(pub i64);

impl From<ArbeidssoekerId> for i64 {
    fn from(arbeidssoeker_id: ArbeidssoekerId) -> Self {
        arbeidssoeker_id.0
    }
}
