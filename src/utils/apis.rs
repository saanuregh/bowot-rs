use lazy_static::lazy_static;
use rand::{seq::SliceRandom, thread_rng, Rng};
use regex::Regex;
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, error::Error, str::FromStr};

pub type ApiError = Box<dyn Error + Send + Sync>;

lazy_static! {
    static ref CLIENT: Client = Client::builder().gzip(true).brotli(true).build().unwrap();
    static ref IMAGE_EXT_RE: Regex = Regex::new(r"^.*(png|gif|jpeg|jpg)$").unwrap();
}

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
    Ok(CLIENT
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
    Ok(CLIENT
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
    Ok(CLIENT
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
    Ok(CLIENT
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
    Ok(CLIENT
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
    Ok(CLIENT
        .get(url)
        .send()
        .await?
        .json::<HashMap<String, String>>()
        .await?)
}

pub async fn get_translate<S: Into<String>>(target: S, text: S) -> Result<String, ApiError> {
    Ok(CLIENT
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

#[derive(Clone, Deserialize, Debug)]
pub struct TriviaResponse {
    pub response_code: u8,
    pub results: Vec<TriviaResult>,
}
#[derive(Clone, Deserialize, Debug)]
pub struct TriviaResult {
    pub category: String,
    #[serde(rename = "type")]
    pub question_type: String,
    pub difficulty: String,
    pub question: String,
    pub correct_answer: String,
    pub incorrect_answers: Vec<String>,
}

#[derive(Copy, Clone, Deserialize, Debug)]
pub enum TriviaCategory {
    Any = 0,
    GeneralKnowledge = 9,
    EntertainmentBooks = 10,
    EntertainmentFilm = 11,
    EntertainmentMusic = 12,
    EntertainmentMusicalsAndTheatres = 13,
    EntertainmentTelevision = 14,
    EntertainmentVideoGames = 15,
    EntertainmentBoardGames = 16,
    ScienceNature = 17,
    ScienceComputers = 18,
    ScienceMathematics = 19,
    Mythology = 20,
    Sports = 21,
    Geography = 22,
    History = 23,
    Politics = 24,
    Art = 25,
    Celebrities = 26,
    Animals = 27,
    Vehicles = 28,
    EntertainmentComics = 29,
    ScienceGadgets = 30,
    EntertainmentJapaneseAnimeAndManga = 31,
    EntertainmentCartoonAndAnimations = 32,
}

impl FromStr for TriviaCategory {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let i = s.parse::<usize>()?;
        Ok(match i {
            0 => TriviaCategory::Any,
            9 => TriviaCategory::GeneralKnowledge,
            10 => TriviaCategory::EntertainmentBooks,
            11 => TriviaCategory::EntertainmentFilm,
            12 => TriviaCategory::EntertainmentMusic,
            13 => TriviaCategory::EntertainmentMusicalsAndTheatres,
            14 => TriviaCategory::EntertainmentTelevision,
            15 => TriviaCategory::EntertainmentVideoGames,
            16 => TriviaCategory::EntertainmentBoardGames,
            17 => TriviaCategory::ScienceNature,
            18 => TriviaCategory::ScienceComputers,
            19 => TriviaCategory::ScienceMathematics,
            20 => TriviaCategory::Mythology,
            21 => TriviaCategory::Sports,
            22 => TriviaCategory::Geography,
            23 => TriviaCategory::History,
            24 => TriviaCategory::Politics,
            25 => TriviaCategory::Art,
            26 => TriviaCategory::Celebrities,
            27 => TriviaCategory::Animals,
            28 => TriviaCategory::Vehicles,
            29 => TriviaCategory::EntertainmentComics,
            30 => TriviaCategory::ScienceGadgets,
            31 => TriviaCategory::EntertainmentJapaneseAnimeAndManga,
            32 => TriviaCategory::EntertainmentCartoonAndAnimations,
            _ => {
                return Err("Invalid digit".into());
            }
        })
    }
}
#[derive(Copy, Clone, Deserialize, Debug)]
pub enum TriviaDifficulty {
    Any,
    Easy,
    Medium,
    Hard,
}

impl FromStr for TriviaDifficulty {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "any" => TriviaDifficulty::Any,
            "easy" => TriviaDifficulty::Easy,
            "medium" => TriviaDifficulty::Medium,
            "hard" => TriviaDifficulty::Hard,
            _ => {
                return Err("Parse error".into());
            }
        })
    }
}

impl TriviaDifficulty {
    pub fn value(&self) -> &str {
        match *self {
            TriviaDifficulty::Any => "0",
            TriviaDifficulty::Easy => "easy",
            TriviaDifficulty::Medium => "medium",
            TriviaDifficulty::Hard => "hard",
        }
    }
}

pub async fn get_trivia(
    amount: usize,
    category: TriviaCategory,
    difficulty: TriviaDifficulty,
) -> Result<TriviaResponse, ApiError> {
    Ok(CLIENT
        .get("https://opentdb.com/api.php")
        .query(&[
            ("amount", amount.to_string()),
            ("category", (category as u8).to_string()),
            ("difficulty", difficulty.value().to_string()),
        ])
        .send()
        .await?
        .json::<TriviaResponse>()
        .await?)
}

// Structs used to deserialize the output of the reddit api.
#[derive(Deserialize, Clone)]
pub struct RedditPost {
    pub title: String,
    pub subreddit_name_prefixed: String,
    pub selftext: String,
    pub downs: i64,
    pub ups: i64,
    pub created: f64,
    pub url: String,
    pub over_18: bool,
    pub permalink: String,
}
#[derive(Deserialize)]
struct RedditDataChild {
    data: RedditPost,
}

#[derive(Deserialize)]
struct RedditData {
    dist: i64,
    children: Vec<RedditDataChild>,
}

#[derive(Deserialize)]
struct RedditResponse {
    data: RedditData,
}

// Gets a random post from a vector of subreddit.
pub async fn reddit_random_post(
    subreddits: Vec<&str>,
    image: bool,
) -> Result<RedditPost, ApiError> {
    let subreddit = subreddits.choose(&mut thread_rng()).unwrap();
    let url = Url::parse(&format!(
        r"https://www.reddit.com/r/{}/hot/.json?sort=top&t=week&limit=25",
        subreddit
    ))?;
    let data = CLIENT
        .get(url)
        .header("User-Agent", "bowot")
        .send()
        .await?
        .json::<RedditResponse>()
        .await?;
    let posts = data.data.children;
    let mut rng = thread_rng();
    let mut idx: i64 = rng.gen_range(0, data.data.dist);
    let mut post: RedditPost;
    for _ in 0..10 {
        post = posts[idx as usize].data.clone();
        if !post.over_18 {
            if image {
                if IMAGE_EXT_RE.is_match(&post.url) {
                    return Ok(post);
                }
            } else {
                if post.selftext != "" && post.selftext.len() < 2048 {
                    return Ok(post);
                }
            }
        }
        idx = rng.gen_range(0, data.data.dist);
    }
    Err("No result found".into())
}

#[derive(Debug, Deserialize)]
struct Artist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SpotifyTrack {
    artists: Vec<Artist>,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Item {
    track: SpotifyTrack,
}

#[derive(Debug, Deserialize)]
struct Tracks {
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct SpotifyData {
    name: String,
    tracks: Tracks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotifyToken {
    client_id: String,
    access_token: String,
    access_token_expiration_timestamp_ms: i64,
    is_anonymous: bool,
}

async fn spotify_get_access_token<S: Into<String>>(spotify_url: S) -> Result<String, ApiError> {
    Ok(CLIENT
        .get("https://open.spotify.com/get_access_token?reason=transport&productType=web_player")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:84.0) Gecko/20100101 Firefox/84.0",
        )
        .header(
            "Accept",
            "application/json",
        )
        .header("Accept-Language", "en")
        .header("app-platform", "WebPlayer")
        .header("spotify-app-version", "1.1.50.23.gb3574ed3")
        .header("DNT", "1")
        .header("Connection", "keep-alive")
        .header("Cookie", "sp_t=7965d077939bf34fc3e922d08601ec9b; sp_landing=https%3A%2F%2Fopen.spotify.com%2Fplaylist%2F7FK4Xae9oH4IdTCAZ2otdT")
        .header("TE", "Trailers")
        .header("Referer", spotify_url.into().clone())
        .send().await?.json::<SpotifyToken>().await?.access_token)
}

pub async fn get_spotify_tracks<S: Into<String> + Copy + std::fmt::Debug>(
    spotify_url: S,
) -> Result<(String, Vec<String>), ApiError> {
    let access_token = spotify_get_access_token(spotify_url.into().clone()).await?;
    let playlist_id = spotify_url
        .into()
        .strip_prefix("https://open.spotify.com/playlist/")
        .unwrap()
        .to_string();
    let spotify_data = CLIENT
        .get(Url::parse(&format!(
            "https://api.spotify.com/v1/playlists/{}?type=track%2Cepisode&market=US",
            playlist_id
        ))?)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:84.0) Gecko/20100101 Firefox/84.0",
        )
        .header("Accept", "application/json")
        .header("Accept-Language", "en")
        .header("Referer", "https://open.spotify.com/")
        .bearer_auth(access_token)
        .send()
        .await?
        .json::<SpotifyData>()
        .await?;
    let tracks: Vec<String> = spotify_data
        .tracks
        .items
        .iter()
        .map(|t| format!("{} - {}", t.track.artists[0].name, t.track.name))
        .collect();
    return Ok((spotify_data.name, tracks));
}

pub async fn generate_triggered_avatar<S: Into<String>>(avatar: S) -> Result<Vec<u8>, ApiError> {
    Ok(CLIENT
        .get("https://some-random-api.ml/canvas/triggered")
        .query(&[("avatar", avatar.into())])
        .send()
        .await?
        .bytes()
        .await?
        .to_vec())
}
