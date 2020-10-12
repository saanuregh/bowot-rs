use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, error::Error};

pub type ApiError = Box<dyn Error + Send + Sync>;

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

pub async fn get_valorant_status() -> Result<ValorantStatus, ApiError> {
    Ok(Client::new()
        .get(Url::parse("https://riotstatus.vercel.app/valorant")?)
        .send()
        .await?
        .json::<Vec<ValorantStatus>>()
        .await?[0]
        .clone())
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

pub async fn get_lyrics<S: Into<String>>(title: S) -> Result<Lyrics, ApiError> {
    Ok(Client::new()
        .get(Url::parse_with_params(
            "https://some-random-api.ml/lyrics",
            &[("title", title.into())],
        )?)
        .send()
        .await?
        .json::<Lyrics>()
        .await?)
}

// Structs used to deserialize the output of the urban dictionary api call.
#[derive(Deserialize, Clone)]
pub struct UrbanDict {
    pub definition: String,
    pub permalink: String,
    pub thumbs_up: u32,
    pub thumbs_down: u32,
    pub author: String,
    pub written_on: String,
    pub example: String,
    pub word: String,
}

#[derive(Deserialize)]
pub struct UrbanList {
    pub list: Vec<UrbanDict>,
}

pub async fn urban_dict<S: Into<String>>(term: S) -> Result<UrbanList, ApiError> {
    Ok(Client::new()
        .get(Url::parse_with_params(
            "http://api.urbandictionary.com/v0/define",
            &[("term", term.into())],
        )?)
        .send()
        .await?
        .json::<UrbanList>()
        .await?)
}

// Structs used to deserialize the output of the dictionary api call.
#[derive(Debug, Deserialize)]
pub struct DictionaryElement {
    pub word: String,
    pub phonetic: Option<String>,
    pub origin: Option<String>,
    pub meanings: Vec<Meaning>,
}

#[derive(Debug, Deserialize)]
pub struct Meaning {
    #[serde(rename = "partOfSpeech")]
    pub part_of_speech: Option<String>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Deserialize)]
pub struct Definition {
    pub definition: String,
    pub synonyms: Option<Vec<String>>,
    pub example: Option<String>,
}

pub async fn define_term<S: Into<String>>(
    word: S,
    lang: S,
) -> Result<Vec<DictionaryElement>, ApiError> {
    Ok(Client::new()
        .get(
            Url::parse("https://api.dictionaryapi.dev/api/v2/entries/")?
                .join(&(lang.into() + "/"))?
                .join(&word.into())?,
        )
        .send()
        .await?
        .json::<Vec<DictionaryElement>>()
        .await?)
}

// Structs used to deserialize the output of the chuck norris joke api call.
#[derive(Debug, Deserialize)]
pub struct ChuckResponse {
    pub categories: Option<Vec<String>>,
    pub value: Option<String>,
}

pub async fn get_chuck() -> Result<ChuckResponse, ApiError> {
    Ok(Client::new()
        .get(Url::parse("https://api.chucknorris.io/jokes/random")?)
        .send()
        .await?
        .json::<ChuckResponse>()
        .await?)
}

pub async fn neko_api<S: Into<String>>(
    endpoint: S,
    img: bool,
) -> Result<HashMap<String, String>, ApiError> {
    let mut url = Url::parse("https://nekos.life/api/v2/")?;
    if img {
        url = url.join("img/")?;
    }
    url = url.join(&endpoint.into())?;
    Ok(Client::new()
        .get(url)
        .send()
        .await?
        .json::<HashMap<String, String>>()
        .await?)
}

pub async fn get_translate<S: Into<String>>(target: S, text: S) -> Result<String, ApiError> {
    Ok(Client::new()
        .get(Url::parse_with_params(
            "https://translate.googleapis.com/translate_a/single",
            &[
                ("client", "gtx"),
                ("ie", "UTF-8"),
                ("oe", "UTF-8"),
                ("dt", "t"),
                ("sl", "auto"),
                ("tl", &target.into()),
                ("q", &text.into()),
            ],
        )?)
        .send()
        .await?
        .json::<JsonValue>()
        .await?
        .as_array()
        .unwrap()[0]
        .as_array()
        .unwrap()[0]
        .as_array()
        .unwrap()[0]
        .as_str()
        .unwrap()
        .to_string())
}
