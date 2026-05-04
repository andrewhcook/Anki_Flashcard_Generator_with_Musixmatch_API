
use std::env;
use dotenvy::dotenv;
use std::collections::HashSet;
use csv::{ReaderBuilder, WriterBuilder};
use serde::Deserialize;
use reqwest::Client;
use tokio::time::{sleep, Duration};

#[derive(Deserialize)]
struct InputRow {
    #[serde(rename = "Artist")]
    artist: String,
    #[serde(rename = "Artist_Id")]
    artist_id: String,
    #[serde(rename = "ISO 639-1 Language Code")]
    language_code: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // prefer dotenvy crate for Windows reliability
    dotenv().ok();
    //dotenvy::from_path("./env")?;
    let key = "MUSIXMATCH_API_KEY";
    match env::var(key) {
        Ok(val) => println!("{}, successfully loaded", key) ,
        Err(e) => println!("couldn't interpret {} {}", key, e),
    }
    let api_key = env::var(key)
        .expect("MUSIXMATCH_API_KEY must be set in .env");
    let client = Client::new();
    let mut rdr = ReaderBuilder::new().from_path("Artists_to_Check.csv")?;
    let mut wtr = WriterBuilder::new().from_path("TranslatableSongs.csv")?;
    // Header: Artist,Song Name,Type,Line,FullTranslation,FullOriginal
    wtr.write_record(&["Artist", "Song Name", "ISO 639-1 Language Code", "track_id", "Translation Available?"])?;

    let mut seen_lines: HashSet<String> = HashSet::new();

    for result in rdr.deserialize() {
        let row: InputRow = result?;
        // 1) Search track to get track_id / commontrack_id
        let q = format!("{}", row.artist);
        let search_url = format!(
            "https://api.musixmatch.com/ws/1.1/track.search?&q_artist={}&page_size=500&page=1&apikey={}",
            urlencoding::encode(&row.artist),
            api_key
        );

        let search_resp = client.get(&search_url).send().await?;
        let search_json: serde_json::Value = search_resp.json().await?;
        // Try to extract first track_id
        // after you parse `search_json`
use serde_json::Value;

if let Some(track_list) = search_json["message"]["body"]["track_list"].as_array() {
    for item in track_list {
        // item is like { "track": { ... } }
        let track_obj = item.get("track");

        // commontrack_id as Option<i64>
        let track_id: Option<i64> = track_obj
            .and_then(|t| t.get("commontrack_id"))
            .and_then(|v| v.as_i64());

        if track_id.is_none() {
            // skip items without an id
            continue;
        }
        let track_id = track_id.unwrap().to_string();

        // track name as Option<String>
        let song_name: Option<String> = track_obj
            .and_then(|t| t.get("track_name"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        // translation availability as Option<&Vec<Value>>
        let translation_available: Option<&Vec<Value>> = track_obj
            .and_then(|t| t.get("track_lyrics_translation_status"))
            .and_then(|v| v.as_array());
            
            let mut length_of_vec = 0;
            if let Some(vec) = translation_available {
                length_of_vec = vec.len();
            }
        let translation_available_entry = match length_of_vec > 0 {
            true => "True".to_owned(),
            false => "False".to_owned(),
        };

        // avoid unwrap on song_name to prevent panic
        let song_name_str = song_name.as_deref().unwrap_or("<unknown>");

        // dedupe if you want
        if seen_lines.insert(format!("{}|{}", &row.artist, song_name_str)) {
            wtr.write_record(&[
                &row.artist,
                song_name_str,
                &row.language_code,
                &track_id,
                &translation_available_entry,
            ])?;
        }

        // be polite with API
        sleep(Duration::from_millis(350)).await;
    }
} else {
    // no tracks found for this artist
    println!("No track_list for artist {}", row.artist);
}
wtr.flush()?;

        }
    println!("Wrote TranslatableSongs.csv");
    Ok(())
}
