use std::time::Duration;

pub fn endpoint() -> Option<&'static str> {
    option_env!("MURATING_REPORT_URL").filter(|u| u.starts_with("https://"))
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("bug reports are not set up in this build yet")]
    NotConfigured,
    #[error("the report server said {0}")]
    Status(u16),
    #[error("network: {0}")]
    Net(String),
}

pub fn post(body: &str, user_agent: &str) -> Result<(), ReportError> {
    let url = endpoint().ok_or(ReportError::NotConfigured)?;
    let resp = ureq::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(20))
        .build()
        .post(url)
        .set("Content-Type", "application/json")
        .send_string(body);
    match resp {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, _)) => Err(ReportError::Status(code)),
        Err(e) => Err(ReportError::Net(e.to_string())),
    }
}

pub fn cooldown_left(last_sent: Option<i64>, now: i64) -> i64 {
    match last_sent {
        Some(t) if t <= now => (t + COOLDOWN_SECS - now).max(0),
        Some(_) => 0,
        None => 0,
    }
}

pub const COOLDOWN_SECS: i64 = 15 * 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_is_fifteen_minutes() {
        assert_eq!(cooldown_left(None, 1000), 0);
        assert_eq!(cooldown_left(Some(1000), 1000), 900);
        assert_eq!(cooldown_left(Some(1000), 1600), 300);
        assert_eq!(cooldown_left(Some(1000), 1900), 0);
        assert_eq!(cooldown_left(Some(5000), 1000), 0, "a future stamp never locks it");
    }
}
