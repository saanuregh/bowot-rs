use std::collections::HashMap;

use lazy_static::lazy_static;
use rand::{seq::SliceRandom, thread_rng, Rng};
use regex::Regex;
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use strum_macros::{EnumString, ToString};

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

pub async fn get_valorant_status() -> anyhow::Result<ValorantStatus> {
	Ok(CLIENT
		.get(Url::parse("https://riotstatus.vercel.app/valorant")?)
		.send()
		.await?
		.json::<Vec<ValorantStatus>>()
		.await?[0]
		.clone())
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

pub async fn urban_dict<S: Into<String>>(term: S) -> anyhow::Result<UrbanList> {
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
) -> anyhow::Result<Vec<DictionaryElement>> {
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

pub async fn get_chuck() -> anyhow::Result<ChuckResponse> {
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
) -> anyhow::Result<HashMap<String, String>> {
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

pub async fn get_translate<S: Into<String>>(target: S, text: S) -> anyhow::Result<String> {
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

#[derive(Copy, Clone, Deserialize, Debug, EnumString)]
pub enum TriviaCategory {
	Any,
	GeneralKnowledge,
	EntertainmentBooks,
	EntertainmentFilm,
	EntertainmentMusic,
	EntertainmentMusicalsAndTheatres,
	EntertainmentTelevision,
	EntertainmentVideoGames,
	EntertainmentBoardGames,
	ScienceNature,
	ScienceComputers,
	ScienceMathematics,
	Mythology,
	Sports,
	Geography,
	History,
	Politics,
	Art,
	Celebrities,
	Animals,
	Vehicles,
	EntertainmentComics,
	ScienceGadgets,
	EntertainmentJapaneseAnimeAndManga,
	EntertainmentCartoonAndAnimations,
}

#[derive(Copy, Clone, Deserialize, Debug, EnumString, ToString)]
pub enum TriviaDifficulty {
	Any,
	Easy,
	Medium,
	Hard,
}

pub async fn get_trivia(
	amount: usize,
	category: TriviaCategory,
	difficulty: TriviaDifficulty,
) -> anyhow::Result<TriviaResponse> {
	let mut difficulty_str: String = difficulty.to_string().to_lowercase();
	if difficulty_str.contains("any") {
		difficulty_str = "0".to_string();
	}
	Ok(CLIENT
		.get("https://opentdb.com/api.php")
		.query(&[
			("amount", amount.to_string()),
			("category", (category as u8).to_string()),
			("difficulty", difficulty_str),
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
pub async fn reddit_random_post(subreddits: &[&str], image: bool) -> anyhow::Result<RedditPost> {
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
	let mut idx: i64 = rng.gen_range(0..data.data.dist);
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
		idx = rng.gen_range(0..data.data.dist);
	}
	Err(anyhow::anyhow!("No result found"))
}

pub async fn generate_triggered_avatar<S: Into<String>>(avatar: S) -> anyhow::Result<Vec<u8>> {
	Ok(CLIENT
		.get("https://some-random-api.ml/canvas/triggered")
		.query(&[("avatar", avatar.into())])
		.send()
		.await?
		.bytes()
		.await?
		.to_vec())
}
