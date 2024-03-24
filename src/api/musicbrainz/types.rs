use serde::{Deserialize, Deserializer};

fn non_empty_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let o: Option<String> = Option::deserialize(d)?;
    Ok(o.filter(|s| !s.is_empty()))
}

#[derive(Debug, Deserialize)]
pub struct MBTrack {
    pub id: String,
    pub title: String,
    pub length: u32,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<MBArtistCredit>,
    pub releases: Vec<MBRelease>,
}

#[derive(Debug, Deserialize)]
pub struct MBArtistCredit {
    pub name: String,
    #[serde(deserialize_with = "non_empty_str")]
    pub joinphrase: Option<String>,
    pub artist: MBArtist,
}

#[derive(Debug, Deserialize)]
pub struct MBArtist {
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Debug, Deserialize)]
pub struct MBRelease {
    pub id: String,
    pub title: String,
    pub quality: String,
    pub status: String,
    #[serde(rename = "packaging-id")]
    pub packaging_id: String,
    pub country: String,
}
