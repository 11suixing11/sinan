pub(crate) mod aliyun;
pub(crate) mod signing;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
pub(crate) mod transport;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Failure {
    pub code: &'static str,
    pub retry_after: i64,
}

impl From<&'static str> for Failure {
    fn from(code: &'static str) -> Self {
        Self {
            code,
            retry_after: 0,
        }
    }
}

pub(crate) fn credential(value: &str) -> bool {
    (8..=256).contains(&value.len())
        && value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"-_/+=.".contains(&v))
}
