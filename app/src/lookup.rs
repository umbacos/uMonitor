use crate::model::SourceKind;
use serde_json::Value;
use std::{sync::mpsc, time::Duration};

#[derive(Clone)]
pub struct Match {
    pub label: String,
    pub symbol: String,
    pub coordinates: Option<(f64, f64)>,
}
#[derive(Default)]
pub struct Lookup {
    pub source: Option<u64>,
    pub query: String,
    pub results: Vec<Match>,
    pub message: String,
    pending: Option<mpsc::Receiver<Result<Vec<Match>, String>>>,
}
impl Lookup {
    pub fn select(&mut self, id: u64) {
        if self.source != Some(id) {
            *self = Self {
                source: Some(id),
                ..Self::default()
            };
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn poll(&mut self) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(matches) => {
                            self.message = if matches.is_empty() {
                                "No matches found. Try another name.".into()
                            } else {
                                String::new()
                            };
                            self.results = matches;
                        }
                        Err(error) => self.message = error,
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.message = "Search could not finish. Please try again.".into();
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    pub fn search(&mut self, kind: SourceKind, token: String, ctx: egui_context::Context) {
        if self.busy() {
            return;
        }
        let query = self.query.trim().to_owned();
        self.results.clear();
        if query.is_empty() || (kind != SourceKind::Stock && query.chars().count() < 2) {
            self.message = "Enter at least two characters.".into();
            return;
        }
        if kind == SourceKind::Stock && token.trim().is_empty() {
            self.message = "Enter your Alpha Vantage API key first.".into();
            return;
        }
        self.message = "Searching…".into();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(search(&kind, &query, &token));
            ctx.request_repaint();
        });
    }
}
// Keep the UI type explicit while the network search remains independent of Studio.
use eframe::egui as egui_context;
fn search(kind: &SourceKind, query: &str, token: &str) -> Result<Vec<Match>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "Could not start search.")?;
    let request = if *kind == SourceKind::Weather {
        client
            .get("https://geocoding-api.open-meteo.com/v1/search")
            .query(&[
                ("name", query),
                ("count", "10"),
                ("language", "en"),
                ("format", "json"),
            ])
    } else {
        client.get(crate::stocks::URL).query(&[
            ("function", "SYMBOL_SEARCH"),
            ("keywords", query),
            ("apikey", token.trim()),
        ])
    };
    let response = request
        .send()
        .map_err(|_| "Search could not connect. Check your internet connection.")?;
    if !response.status().is_success() {
        return Err(format!(
            "Search returned {}. Check your API key or try again later.",
            response.status()
        ));
    }
    let value: Value = response
        .json()
        .map_err(|_| "Search returned an unreadable response.")?;
    if *kind == SourceKind::Stock {
        crate::stocks::check_response(&value)?;
        if !value["bestMatches"].is_array() {
            return Err("Alpha Vantage returned an unreadable search result.".into());
        }
    }
    Ok(parse_results(kind, &value))
}
fn parse_results(kind: &SourceKind, value: &Value) -> Vec<Match> {
    value[if *kind == SourceKind::Weather {
        "results"
    } else {
        "bestMatches"
    }]
    .as_array()
    .into_iter()
    .flatten()
    .take(20)
    .filter_map(|item| {
        if *kind == SourceKind::Weather {
            let lat = item["latitude"].as_f64()?;
            let lon = item["longitude"].as_f64()?;
            let parts: Vec<_> = ["name", "admin1", "country"]
                .iter()
                .filter_map(|k| item[k].as_str())
                .filter(|s| !s.is_empty())
                .collect();
            Some(Match {
                label: format!("{}  ({lat:.4}, {lon:.4})", parts.join(", ")),
                symbol: String::new(),
                coordinates: Some((lat, lon)),
            })
        } else {
            let symbol = item["1. symbol"].as_str()?.trim().to_owned();
            if symbol.is_empty() {
                return None;
            }
            Some(Match {
                label: format!(
                    "{} — {} ({}, {})",
                    symbol,
                    item["2. name"].as_str().unwrap_or(""),
                    item["4. region"].as_str().unwrap_or(""),
                    item["8. currency"].as_str().unwrap_or("")
                ),
                symbol,
                coordinates: None,
            })
        }
    })
    .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_provider_search_results_without_losing_coordinates_or_exchange_symbols() {
        let places = parse_results(
            &SourceKind::Weather,
            &serde_json::json!({"results":[{"name":"Rome","country":"Italy","latitude":41.89,"longitude":12.48}]}),
        );
        assert_eq!(places[0].coordinates, Some((41.89, 12.48)));
        let stocks = parse_results(
            &SourceKind::Stock,
            &serde_json::json!({"bestMatches":[{"1. symbol":"BRK.B","2. name":"Berkshire Hathaway","4. region":"United States","8. currency":"USD"}]}),
        );
        assert_eq!(stocks[0].symbol, "BRK.B");
        assert!(parse_results(&SourceKind::Weather, &serde_json::json!({})).is_empty());
    }
}
