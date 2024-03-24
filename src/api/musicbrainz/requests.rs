use crate::config::client;
use serde::Deserialize;

use super::types::*;

use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::time::sleep;

fn request_throttle() -> Duration {
    pub static LAST_REQUEST: Mutex<Option<Instant>> = Mutex::new(None);
    pub static BURST_ALLOWED: Mutex<bool> = Mutex::new(true);

    let mut last_request = LAST_REQUEST.lock().unwrap();
    let mut burst_allowed = BURST_ALLOWED.lock().unwrap();

    let duration;
    if last_request.is_some() {
        let last_request = last_request.unwrap();
        let elapsed = last_request.elapsed();

        // this is a bit hacky - since the musicbrainz api checks the average request rate, we can
        // allow a burst of requests through if there was a longer pause since the last request.
        // this allows a run through of the audio lookup pipeline to complete a lot faster.
        if *burst_allowed {
            duration = Duration::from_secs(0);
            *burst_allowed = false;
        } else if elapsed < Duration::from_secs(1) {
            duration = Duration::from_secs(1) - elapsed;
            println!("Throttling request for {:?}", duration);
        } else {
            if elapsed > Duration::from_secs(3) {
                *burst_allowed = true;
            }
            duration = Duration::from_secs(0);
        }
    } else {
        duration = Duration::from_secs(0);
    }

    last_request.replace(Instant::now());

    duration
}

#[derive(Debug)]
pub enum MBError {
    LookupFailed(String),
    RateLimited,
    NotFound,
}
impl std::fmt::Display for MBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MBError::LookupFailed(e) => write!(f, "Lookup failed: {}", e),
            MBError::RateLimited => write!(f, "Rate limited"),
            MBError::NotFound => write!(f, "Not found"),
        }
    }
}

pub async fn track_by_id(mbid: &str) -> Result<MBTrack, MBError> {
    let res: MBTrack = do_request("recording", &format!("{}?inc=artists+releases", mbid)).await?;

    Ok(res)
}

async fn do_request<T>(req_type: &str, query: &str) -> Result<T, MBError>
where
    T: for<'de> Deserialize<'de>,
{
    sleep(request_throttle()).await;
    println!("Requesting {}?{}", req_type, query);

    let res = client()
        .get(&format!(
            "https://musicbrainz.org/ws/2/{}/{}&fmt=json",
            req_type, query,
        ))
        .send()
        .await
        .map_err(|e| MBError::LookupFailed(e.to_string()))?;

    match res.status().as_u16() {
        200 => {}
        404 => return Err(MBError::NotFound),
        503 => return Err(MBError::RateLimited),
        _ => {
            return Err(MBError::LookupFailed(format!(
                "HTTP status code: {}",
                res.status()
            )))
        }
    }

    let res = res
        .json::<T>()
        .await
        .map_err(|e| MBError::LookupFailed(e.to_string()))?;

    Ok(res)
}
