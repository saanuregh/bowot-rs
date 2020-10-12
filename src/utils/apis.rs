use reqwest::Client;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct ValorantStatus {
    pub name: String,
    pub regions: Vec<Region>,
}

#[derive(Clone, Deserialize)]
pub struct Region {
    pub name: String,
    pub maintenances: Vec<Incident>,
    pub incidents: Vec<Incident>,
}

#[derive(Clone, Deserialize)]
pub struct Incident {
    pub description: String,
    pub created_at: String,
    pub platforms: Vec<String>,
    pub maintenance_status: Option<String>,
    pub incident_severity: Option<String>,
    pub updates: Vec<Update>,
    pub updated_at: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct Update {
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn get_valorant_status() -> Result<ValorantStatus, String> {
    let client = Client::new();
    let url = "https://riotstatus.vercel.app/valorant";
    match client.get(url).send().await {
        Ok(resp) => match resp.json::<Vec<ValorantStatus>>().await {
            Ok(data) => Ok(data[0].clone()),
            Err(_) => Err("error decoding".to_string()),
        },
        Err(_) => Err("error getting response".to_string()),
    }
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub title: String,
    pub author: String,
    pub lyrics: String,
    pub thumbnail: Thumbnail,
    pub links: Links,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub genius: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub genius: String,
}

pub async fn get_lyrics(title: String) -> Result<Lyrics, ()> {
    let client = Client::new();
    match client
        .get("https://some-random-api.ml/lyrics")
        .query(&[("title", title)])
        .send()
        .await
    {
        Ok(resp) => match resp.json::<Lyrics>().await {
            Ok(data) => Ok(data),
            Err(_) => Err(()),
        },
        Err(_) => Err(()),
    }
}
