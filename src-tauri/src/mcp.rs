use crate::{model::*, store::Store};
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

fn tool(name: &str, description: &str, schema: Value, readonly: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":schema,"annotations":{"readOnlyHint":readonly,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}
fn tools() -> Value {
    let empty = json!({"type":"object","properties":{},"additionalProperties":false});
    let record: Value =
        serde_json::from_str(include_str!("../../catalog/subscription.schema.json")).unwrap();
    let batch = json!({"type":"object","properties":{"records":{"type":"array","minItems":1,"maxItems":1000,"items":record}},"required":["records"],"additionalProperties":false});
    let mut import = batch.clone();
    import["properties"]["importId"] = json!({"type":"string","description":"Stable unique batch ID. Reuse only with identical records."});
    import["properties"]["allowDuplicates"] = json!({"type":"boolean","default":false});
    import["required"] = json!(["records", "importId"]);
    json!({"tools":[
        tool("get_catalog","Get service categories, official plan names, currencies and the subscription schema. Never guess prices.",empty.clone(),true),
        tool("list_subscriptions","Read saved records with original amounts, currencies and expiry anchors.",empty,true),
        tool("preview_import","Validate records and identify duplicate IDs / possible duplicate names without writing.",batch,true),
        tool("import_subscriptions","Import a validated batch atomically. Preview first. Same IDs are skipped; conflicting IDs fail. Set allowDuplicates only for intentional separate accounts.",import,false),
        tool("get_summary","Read amortized bill and personal costs. Lifetime costs use elapsed calendar days, including purchase day. Missing FX leaves totals null.",json!({"type":"object","properties":{"currency":{"type":"string"},"period":{"enum":["hour","month","year"]}},"additionalProperties":false}),true)
    ]})
}
fn call(name: &str, args: Value) -> Result<Value> {
    let store = Store::system()?;
    let (db, _) = store.read()?;
    if !db.settings.mcp_enabled {
        return Err("MCP is disabled. Enable Agent / MCP in Subbit settings first.".into());
    }
    match name {
        "get_catalog" => {
            let cache = crate::catalog::CatalogCache::system()?;
            let catalog = cache.refresh(false).unwrap_or_else(|_| cache.load()).catalog;
            Ok(json!({"catalog":catalog,"currencies":CURRENCIES,"subscriptionSchema":serde_json::from_str::<Value>(include_str!("../../catalog/subscription.schema.json")).unwrap()}))
        }
        "list_subscriptions" => {
            Ok(json!({"subscriptions":db.subscriptions,"revision":db.revision}))
        }
        "get_summary" => db.summary(
            args["currency"].as_str().unwrap_or(&db.settings.currency),
            args["period"].as_str().unwrap_or(&db.settings.period),
            today(),
        ),
        "preview_import" | "import_subscriptions" => {
            let records: Vec<Subscription> =
                serde_json::from_value(args["records"].clone()).map_err(|e| e.to_string())?;
            if name == "preview_import" {
                Store::preview_batch(&db, &records)
            } else {
                store.import_batch(
                    args["importId"]
                        .as_str()
                        .ok_or("importId is required")?
                        .into(),
                    records,
                    args["allowDuplicates"].as_bool().unwrap_or(false),
                )
            }
        }
        _ => Err("Unknown tool".into()),
    }
}
pub fn run() -> Result<()> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut out = io::stdout().lock();
    let mut initialized = false;
    loop {
        let mut line = String::new();
        let bytes = reader
            .by_ref()
            .take(4_000_001)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if bytes == 0 {
            break;
        }
        if bytes > 4_000_000 {
            return Err("MCP message exceeds 4 MB".into());
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                writeln!(out,"{}",json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})).map_err(|e|e.to_string())?;
                out.flush().map_err(|e| e.to_string())?;
                continue;
            }
        };
        let Some(id) = request.get("id") else {
            continue;
        };
        let method = request["method"].as_str().unwrap_or("");
        let result = if request["jsonrpc"] != "2.0" {
            Err((-32600, "Invalid Request"))
        } else {
            match method {
                "initialize" => {
                    initialized = true;
                    let version = match request["params"]["protocolVersion"].as_str() {
                        Some(v @ ("2024-11-05" | "2025-03-26" | "2025-06-18" | "2025-11-25")) => v,
                        _ => "2025-11-25",
                    };
                    Ok(
                        json!({"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"subbit","version":env!("CARGO_PKG_VERSION")},"instructions":"Local subscription tracker. Enable MCP in app settings. Preview imports before committing; do not invent prices, currency or billing periods."}),
                    )
                }
                "ping" => Ok(json!({})),
                _ if !initialized => Err((-32002, "Initialize first")),
                "tools/list" => Ok(tools()),
                "tools/call" => {
                    let args = request["params"]
                        .get("arguments")
                        .cloned()
                        .unwrap_or(json!({}));
                    if let Some(name) = request["params"]["name"].as_str() {
                        if ![
                            "get_catalog",
                            "list_subscriptions",
                            "preview_import",
                            "import_subscriptions",
                            "get_summary",
                        ]
                        .contains(&name)
                        {
                            Err((-32602, "Unknown tool"))
                        } else {
                            Ok(match call(name, args) {
                                Ok(data) => {
                                    json!({"content":[{"type":"text","text":data.to_string()}],"isError":false})
                                }
                                Err(e) => {
                                    json!({"content":[{"type":"text","text":e}],"isError":true})
                                }
                            })
                        }
                    } else {
                        Err((-32602, "Tool name is required"))
                    }
                }
                _ => Err((-32601, "Method not found")),
            }
        };
        let response = match result {
            Ok(v) => json!({"jsonrpc":"2.0","id":id,"result":v}),
            Err((code, message)) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
            }
        };
        writeln!(out, "{response}").map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}
