// Cargo.toml dependencies:
// tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
// reqwest = { version = "0.11", features = ["json", "gzip"] }
// serde = { version = "1.0", features = ["derive"] }
// csv = "1.1"
// dotenv = "0.15"

use std::env;
use dotenvy::dotenv;
use std::collections::HashSet;
use csv::{ReaderBuilder, WriterBuilder};
use serde::Deserialize;
use reqwest::Client;
use tokio::time::{sleep, Duration};
use serde_json::Value;

#[derive(Deserialize)]
struct InputRow {
    #[serde(rename = "Artist")]
    artist: String,
    #[serde(rename = "Song Name")]
    song: String,
    #[serde(rename = "track_id")]
    commontrack_id: String,
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
    let mut rdr = ReaderBuilder::new().from_path("Input.csv")?;
    let mut wtr = WriterBuilder::new().from_path("Results.csv")?;
    // Header: Artist,Song Name,Type,Line,FullTranslation,FullOriginal
    wtr.write_record(&["FullTranslation","FullOriginal", "Artist","Song Name", "ISO 639-1 Language Code",])?;

    let mut seen_lines: HashSet<String> = HashSet::new();

    for result in rdr.deserialize() {
        let row: InputRow = result?;

        let trans_url = format!(
            "https://api.musixmatch.com/ws/1.1/track.lyrics.translation.get?commontrack_id={}&selected_language={}&min_completed=0.5&apikey={}",
            urlencoding::encode(&row.commontrack_id), urlencoding::encode(&row.language_code), urlencoding::encode(&api_key)
        );
        let trans_resp = client.get(&trans_url).send().await?;
        let trans_json: serde_json::Value = trans_resp.json().await?;

let original_raw = trans_json.as_object()
.and_then(|i| i.get("message"))
.and_then(|i| i.get("body"))
.and_then(|i| i.get("lyrics"))
.and_then(|i| i.get("lyrics_body"))
.and_then(|v| v.as_str());

let translated_raw = trans_json.as_object()
.and_then(|i| i.get("message"))
.and_then(|i| i.get("body"))
.and_then(|i| i.get("lyrics"))
.and_then(|i| i.get("lyrics_translated"))
.and_then(|i| i.get("lyrics_body"))
.and_then(|v| v.as_str());

// collect lines so we can index original by the same line index as translated
let original_lines: Vec<String> = original_raw.unwrap_or("")
            .split("\n")
            .map(|s| s.to_string())
            .collect();

let translated_lines: Vec<String> = translated_raw.unwrap_or("")
            .split("\n")
            .map(|s| s.to_string())
            .collect();



for (index, line) in translated_lines.iter().enumerate() {
    println!("{}", line);
    if line.is_empty() { continue; }
    if seen_lines.insert(line.to_string()) {
        // get the corresponding original line by index, if present
        let original_line_for_csv = original_lines.get(index).map(|s| s.clone()).unwrap_or("".to_string());
        if !original_line_for_csv.is_empty() {
            wtr.write_record(&[
                &line,
                &original_line_for_csv,
                &row.artist,
                &row.song,
                &row.language_code,
            ])?;
        } 
    }
    
}
// Write single mapping row pairing full translation to original
if translated_lines.is_empty() {
    continue
};


wtr.write_record(&[
    &translated_raw.unwrap_or(""),
    &original_raw.unwrap_or(""),
    &row.artist.as_str(),
    &row.song.as_str(),
    &row.language_code.as_str(),
])?;

        // be polite with API
        sleep(Duration::from_millis(300)).await;
    }
    


    wtr.flush()?;
    println!("Wrote RESULTS.csv");
    Ok(())
}
