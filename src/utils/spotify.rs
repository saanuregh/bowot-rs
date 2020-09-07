use reqwest::{Client};
use serde::Deserialize;
use tracing::error;

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

async fn get_access_token(spotify_url: String) -> Result<String,()> {
	let client = Client::new();
	match client
	.get("https://open.spotify.com/get_access_token?reason=transport&productType=web_player")
	.header(
		"User-Agent",
		"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:78.0) Gecko/20100101 Firefox/78.0",
	)
	.header(
		"Accept",
		"application/json",
	)
	.header("Accept-Language", "en")
	.header("app-platform", "WebPlayer")
	.header("spotify-app-version", "1594400977")
	.header("DNT", "1")
	.header("Connection", "keep-alive")
	.header("Cookie", "sp_t=d0c5682a3f4f329ae1c0c3ec240d1117; sp_landing=https%3A%2F%2Fopen.spotify.com%2Fplaylist%2F7FK4Xae9oH4IdTCAZ2otdT")
	.header("TE", "Trailers")
	.header("Host", "open.spotify.com")
	.header("Referer", &spotify_url)
	.send()
	.await{
		Ok(resp) => {
			match resp.json::<SpotifyToken>().await{
				Ok(spotify_data) => {
					return Ok(spotify_data.access_token);
				},
				Err(why) => {
					error!("Error getting spotify token {}",why);
					return Err(());
				}
			}
		},
		Err(why) => {
			error!("Error getting spotify token {}",why);
			return Err(());
		}
	
}
}

pub async fn get_spotify_tracks(spotify_url: String) -> Result<(String,Vec<String>), ()> {
	if !spotify_url.starts_with("https://open.spotify.com/playlist/") {
		return Err(());
	}
	if let Ok(access_token) = get_access_token(spotify_url.clone()).await {
		let playlist_id = spotify_url
			.strip_prefix("https://open.spotify.com/playlist/")
			.unwrap()
			.to_string();
		let client = Client::new();
		if let Ok(resp) = client
			.get(&format!(
				"https://api.spotify.com/v1/playlists/{}?type=track%2Cepisode&market=US",
				playlist_id
			))
			.header(
				"User-Agent",
				"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:78.0) Gecko/20100101 Firefox/78.0",
			)
			.header(
				"Accept",
				"application/json",
			)
			.header("Accept-Language", "en")
			.header("Referer", "https://open.spotify.com/")
			.bearer_auth(access_token)
			.send()
			.await
		{
			if let Ok(spotify_data) = resp.json::<SpotifyData>().await {
				if spotify_data.tracks.items.is_empty() {
					return Err(());
				}
				let tracks: Vec<String> = spotify_data
					.tracks
					.items
					.iter()
					.take(20)
					.map(|t| format!("{} - {}", t.track.artists[0].name, t.track.name))
					.collect();
				return Ok((spotify_data.name,tracks));
			}
		}
	}
	error!("Error getting spotify tracks from the playlist");
	Err(())
}
