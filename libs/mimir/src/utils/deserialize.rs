use serde::de::Deserializer;
use serde::Deserialize;
use std::time::Duration;

pub fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let ms: u64 = Deserialize::deserialize(deserializer)?;
    Ok(Duration::from_millis(ms))
}
