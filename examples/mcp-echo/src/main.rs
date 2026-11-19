use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

// A standalone native server. No shell, user files, environment or network access.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let Some(id) = request.get("id") else {
            continue;
        };
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => json!({
                "protocolVersion": request["params"]["protocolVersion"].as_str().unwrap_or("2024-11-05"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "tutorial-echo", "version": "0.1.0"}
            }),
            "tools/list" => json!({"tools": [{
                "name": "echo", "description": "Return the supplied synthetic text",
                "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}},
                    "required": ["text"], "additionalProperties": false}
            }]}),
            "tools/call" if request["params"]["name"] == "echo" => {
                match request["params"]["arguments"]["text"].as_str() {
                    Some(text) => {
                        json!({"content": [{"type": "text", "text": format!("echo:{text}")}]})
                    }
                    None => {
                        json!({"isError": true, "content": [{"type": "text", "text": "text must be a string"}]})
                    }
                }
            }
            "ping" => json!({}),
            _ => {
                writeln!(
                    output,
                    "{}",
                    json!({"jsonrpc":"2.0", "id":id,
                    "error":{"code":-32601, "message":"Method not found"}})
                )?;
                output.flush()?;
                continue;
            }
        };
        writeln!(
            output,
            "{}",
            json!({"jsonrpc":"2.0", "id":id, "result":result})
        )?;
        output.flush()?;
    }
    Ok(())
}
