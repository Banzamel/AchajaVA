//! Home Assistant's REST API: states, services and templates. The address comes from
//! config.json (home_assistant.url, or the one in .mcp.json), the token only from
//! state\ha_token.txt; the token never appears in output, not even in errors.

use serde_json::{json, Value};

use crate::config;
use crate::error::{AppError, Result};
use crate::json::{read_text, read_value};
use crate::paths::Paths;
use crate::retry::{self, Fail};
use crate::text::fold;

pub struct Ha {
    pub url: String,
    token: String,
    pub dry_run: bool,
    http: reqwest::Client,
    states: Option<Vec<Value>>,
}

fn ha_err(text: impl Into<String>) -> AppError {
    AppError::Http(text.into())
}

impl Ha {
    pub fn new(paths: &Paths, dry_run: bool) -> Result<Ha> {
        let cfg = config::load_or_default(paths);
        let mut url = cfg.str_or("home_assistant.url", "");
        if url.is_empty() {
            let mcp = read_value(&paths.mcp());
            url = mcp["mcpServers"]["home-assistant"]["url"].as_str().unwrap_or("").split("/api/").next().unwrap_or("").to_string();
        }
        if url.is_empty() {
            return Err(ha_err("Brak adresu Home Assistant (config.json: home_assistant.url)."));
        }
        let token = read_text(&paths.ha_token()).map(|t| t.trim().to_string()).ok_or_else(|| ha_err("Brak tokenu Home Assistant (state/ha_token.txt)."))?;
        if token.is_empty() {
            return Err(ha_err("Plik state/ha_token.txt jest pusty."));
        }
        Ok(Ha { url: url.trim_end_matches('/').to_string(), token, dry_run, http: crate::runtime::http(15)?, states: None })
    }

    /// A text without the token (in case it ended up in an error message).
    pub fn safe(&self, text: &str) -> String {
        if self.token.is_empty() {
            text.to_string()
        } else {
            text.replace(&self.token, "***")
        }
    }

    pub async fn request(&self, method: reqwest::Method, path: &str, body: Option<&Value>) -> Result<Value> {
        // reads are tried again after a dropped connection or a 5xx; a service call never is,
        // so a light is not switched twice
        let read = method == reqwest::Method::GET || path == "/api/template";
        let policy = if read { &retry::NETWORK } else { &retry::ONCE };
        let method = &method;
        let text = retry::run(policy, move || async move {
            let mut req = self.http.request(method.clone(), format!("{}{path}", self.url)).bearer_auth(&self.token);
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await.map_err(|e| Fail::Transient(ha_err(self.safe(&format!("Brak polaczenia z Home Assistant ({}): {e}", self.url)))))?;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                let detail: String = text.chars().take(300).collect();
                let e = ha_err(self.safe(&format!("Home Assistant: {method} {path} -> HTTP {} {detail}", status.as_u16())));
                return Err(if retry::transient_status(status.as_u16()) { Fail::Transient(e) } else { Fail::Fatal(e) });
            }
            Ok(text)
        })
        .await?;
        if text.is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    /// Calls a service; in --dry-run only prints the request.
    pub async fn service(&self, domain: &str, name: &str, data: &Value) -> Result<Value> {
        let path = format!("/api/services/{domain}/{name}");
        if self.dry_run {
            println!("[dry-run] POST {path} {data}");
            return Ok(json!([]));
        }
        self.request(reqwest::Method::POST, &path, Some(data)).await
    }

    pub async fn states(&mut self) -> Result<Vec<Value>> {
        if self.states.is_none() {
            let v = self.request(reqwest::Method::GET, "/api/states", None).await?;
            self.states = Some(v.as_array().cloned().unwrap_or_default());
        }
        Ok(self.states.clone().unwrap_or_default())
    }

    pub async fn state(&self, entity_id: &str) -> Result<Value> {
        self.request(reqwest::Method::GET, &format!("/api/states/{entity_id}"), None).await
    }

    pub async fn template(&self, text: &str) -> Result<Value> {
        self.request(reqwest::Method::POST, "/api/template", Some(&json!({ "template": text }))).await
    }

    /// A template that renders a JSON list of entity ids.
    pub async fn template_ids(&self, text: &str) -> Result<Vec<String>> {
        let out = self.template(text).await?;
        let list = match out {
            Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),
            other => other,
        };
        Ok(list.as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default())
    }

    /// Entities of the area whose id or name matches (its own and its devices'); [] when none.
    pub async fn area_entities(&self, name: &str) -> Result<Vec<String>> {
        let listing = self.template("{% for a in areas() %}{{ a }}\t{{ area_name(a) }}\n{% endfor %}").await?;
        let wanted = fold(name);
        for line in listing.as_str().unwrap_or("").lines() {
            let (id, area) = line.split_once('\t').unwrap_or((line, ""));
            if wanted == fold(id) || wanted == fold(area) {
                let id_q = serde_json::to_string(id)?;
                return self
                    .template_ids(&format!(
                        "{{{{ (area_entities({id_q}) + (area_devices({id_q}) | map('device_entities') | sum(start=[]))) | unique | list | tojson }}}}"
                    ))
                    .await;
            }
        }
        Ok(Vec::new())
    }

    /// Entities matching an entity id, a name or an area (exact matches first).
    pub async fn find(&mut self, query: &str, domain: Option<&str>) -> Result<Vec<Value>> {
        let all: Vec<Value> = self
            .states()
            .await?
            .into_iter()
            .filter(|s| domain.map_or(true, |d| eid(s).starts_with(&format!("{d}."))))
            .collect();
        if let Some(hit) = all.iter().find(|s| eid(s) == query) {
            return Ok(vec![hit.clone()]);
        }
        let wanted = fold(query);
        let exact: Vec<Value> = all.iter().filter(|s| fold(&friendly(s)) == wanted).cloned().collect();
        if !exact.is_empty() {
            return Ok(exact);
        }
        let partial: Vec<Value> = all.iter().filter(|s| fold(&friendly(s)).contains(&wanted) || fold(&eid(s)).contains(&wanted)).cloned().collect();
        if !partial.is_empty() {
            return Ok(partial);
        }
        let in_area = self.area_entities(query).await?;
        Ok(all.into_iter().filter(|s| in_area.contains(&eid(s))).collect())
    }
}

pub fn eid(s: &Value) -> String {
    s["entity_id"].as_str().unwrap_or("").to_string()
}

pub fn friendly(s: &Value) -> String {
    s["attributes"]["friendly_name"].as_str().unwrap_or("").to_string()
}
