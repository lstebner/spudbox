use serde_json::{json, Value};

use super::SyncError;

pub struct TursoClient {
    http: reqwest::Client,
    url: String,
    token: String,
}

pub struct QueryResult {
    pub rows: Vec<Vec<Value>>,
}

impl TursoClient {
    pub fn new(db_url: &str, token: &str) -> Self {
        // Turso shows both libsql:// and https:// in the dashboard; the HTTP
        // pipeline API needs https://.
        let base = db_url
            .trim_end_matches('/')
            .replace("libsql://", "https://");
        Self {
            http: reqwest::Client::new(),
            url: format!("{base}/v2/pipeline"),
            token: token.to_string(),
        }
    }

    pub async fn query(&self, sql: &str, args: Vec<Value>) -> Result<QueryResult, SyncError> {
        let mut results = self.pipeline(vec![execute_stmt(sql, args)]).await?;
        results.pop().ok_or_else(|| SyncError::UnexpectedResponse("empty pipeline result".into()))
    }

    pub async fn execute(&self, sql: &str, args: Vec<Value>) -> Result<(), SyncError> {
        self.pipeline(vec![execute_stmt(sql, args)]).await?;
        Ok(())
    }

    pub async fn query_scalar_int(&self, sql: &str, args: Vec<Value>) -> Result<i64, SyncError> {
        let result = self.query(sql, args).await?;
        result
            .rows
            .first()
            .and_then(|row| row.first())
            .and_then(row_int)
            .ok_or_else(|| SyncError::UnexpectedResponse("expected scalar integer result".into()))
    }

    pub async fn execute_batch(&self, stmts: Vec<(String, Vec<Value>)>) -> Result<(), SyncError> {
        if stmts.is_empty() {
            return Ok(());
        }
        let reqs: Vec<Value> = stmts.into_iter().map(|(sql, args)| execute_stmt(&sql, args)).collect();
        self.pipeline(reqs).await?;
        Ok(())
    }

    async fn pipeline(&self, mut requests: Vec<Value>) -> Result<Vec<QueryResult>, SyncError> {
        requests.push(json!({"type": "close"}));
        let body = json!({"requests": requests});

        let resp = self
            .http
            .post(&self.url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(SyncError::Api(format!("HTTP {status}: {text}")));
        }

        let body: Value = resp.json().await?;
        let results = body["results"]
            .as_array()
            .ok_or_else(|| SyncError::UnexpectedResponse("missing results array".into()))?;

        let mut out = Vec::new();
        for r in results {
            match r["type"].as_str() {
                Some("error") => {
                    let msg = r["error"]["message"].as_str().unwrap_or("unknown api error");
                    return Err(SyncError::Api(msg.to_string()));
                }
                Some("ok") => {
                    let resp_type = r["response"]["type"].as_str().unwrap_or("");
                    if resp_type == "execute" {
                        let result = &r["response"]["result"];
                            let rows: Vec<Vec<Value>> = result["rows"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .map(|row| row.as_array().cloned().unwrap_or_default())
                                    .collect()
                            })
                            .unwrap_or_default();
                        out.push(QueryResult { rows });
                    }
                }
                _ => {}
            }
        }
        Ok(out)
    }
}

fn execute_stmt(sql: &str, args: Vec<Value>) -> Value {
    json!({"type": "execute", "stmt": {"sql": sql, "args": args}})
}

// Helpers for encoding Rust types as Turso positional args.

pub fn text_arg(v: &str) -> Value {
    json!({"type": "text", "value": v})
}

pub fn int_arg(v: i64) -> Value {
    json!({"type": "integer", "value": v.to_string()})
}

pub fn float_arg(v: f64) -> Value {
    json!({"type": "float", "value": v})
}

pub fn null_arg() -> Value {
    json!({"type": "null"})
}

pub fn opt_int_arg(v: Option<i64>) -> Value {
    match v {
        Some(n) => int_arg(n),
        None => null_arg(),
    }
}

// Helpers for decoding values from query result rows.

pub fn row_text(v: &Value) -> Option<&str> {
    v["value"].as_str()
}

pub fn row_int(v: &Value) -> Option<i64> {
    v["value"].as_str()?.parse().ok()
}

pub fn row_float(v: &Value) -> Option<f64> {
    v["value"].as_f64().or_else(|| v["value"].as_str()?.parse().ok())
}
