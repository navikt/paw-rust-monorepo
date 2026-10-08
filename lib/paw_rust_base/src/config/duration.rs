pub mod iso8601 {
    use serde::{Deserialize, Deserializer};
    use std::time::Duration;

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let s = String::deserialize(d)?;
        parse(&s).map_err(serde::de::Error::custom)
    }

    fn parse(s: &str) -> Result<Duration, String> {
        let rest = s
            .strip_prefix('P')
            .ok_or_else(|| format!("ISO 8601 duration must start with 'P': '{s}'"))?;

        let (date_part, time_part) = match rest.split_once('T') {
            Some((d, t)) => (d, t),
            None => (rest, ""),
        };

        let mut secs: u64 = 0;

        if !date_part.is_empty() {
            secs += parse_components(date_part, &[('W', 604_800), ('D', 86_400)])
                .map_err(|e| format!("in date part of '{s}': {e}"))?;
        }
        if !time_part.is_empty() {
            secs += parse_components(time_part, &[('H', 3_600), ('M', 60), ('S', 1)])
                .map_err(|e| format!("in time part of '{s}': {e}"))?;
        }

        Ok(Duration::from_secs(secs))
    }

    fn parse_components(s: &str, designators: &[(char, u64)]) -> Result<u64, String> {
        let mut total = 0u64;
        let mut num_start = 0;
        for (i, c) in s.char_indices() {
            if c.is_ascii_digit() {
                continue;
            }
            match designators.iter().find(|&&(d, _)| d == c) {
                Some(&(_, factor)) => {
                    let n: u64 = s[num_start..i]
                        .parse()
                        .map_err(|_| format!("invalid number before '{c}'"))?;
                    total += n * factor;
                    num_start = i + c.len_utf8();
                }
                None => return Err(format!("unexpected designator '{c}'")),
            }
        }
        if num_start != s.len() {
            return Err(format!("trailing characters: '{}'", &s[num_start..]));
        }
        Ok(total)
    }
}
