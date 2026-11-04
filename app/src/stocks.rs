use serde_json::Value;

pub const URL: &str = "https://www.alphavantage.co/query";

pub fn check_response(value: &Value) -> Result<(), String> {
    if value.get("Note").is_some() || value.get("Information").is_some() {
        return Err("Alpha Vantage could not serve this request. Check your API key, request allowance and plan, then try again later.".into());
    }
    if value.get("Error Message").is_some() {
        return Err(
            "Alpha Vantage rejected this request. Check your API key and stock symbol.".into(),
        );
    }
    Ok(())
}
