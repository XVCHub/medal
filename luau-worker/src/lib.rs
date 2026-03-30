use futures_util::StreamExt;
extern crate console_error_panic_hook;

use base64::prelude::*;
use luau_lifter::decompile_bytecode;
use serde::{Deserialize, Serialize};
use worker::*;



#[derive(Deserialize)]
struct DecompileMessage {
    id: String,
    encoded_bytecode: String,
}

#[derive(Serialize)]
struct DecompileResponse {
    id: String,
    decompilation: String,
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let router = Router::new();
    router
        .get("/", |_, _| {
            let html = include_str!("../../index.html");
            Response::ok(html).map(|mut res| {
                res.headers_mut().set("Content-Type", "text/html").unwrap();
                res
            })
        })
        .get_async("/decompile_ws", |req, _ctx| async move {
            let pair = WebSocketPair::new()?;
            let server = pair.server;
            server.accept()?;

            wasm_bindgen_futures::spawn_local(async move {
                let mut event_stream = server.events().expect("could not open stream");
                while let Some(event) = event_stream.next().await {
                    if let WebsocketEvent::Message(msg) =
                        event.expect("received error in websocket")
                    {
                        if let Ok(msg) = msg.json::<DecompileMessage>() {
                            let bytecode = BASE64_STANDARD
                                .decode(msg.encoded_bytecode)
                                .expect("bytecode must be base64 encoded");
                            let resp = DecompileResponse {
                                id: msg.id,
                                decompilation: decompile_bytecode(&bytecode, 1),
                            };
                            server
                                .send_with_str(serde_json::to_string(&resp).unwrap())
                                .unwrap();
                        }
                    }
                }
            });

            Response::from_websocket(pair.client)
        })
        .post_async("/decompile", |mut req, _ctx| async move {
            let encoded_bytecode = req.text().await?;
            match BASE64_STANDARD.decode(encoded_bytecode.trim()) {
                Ok(bytecode) => Response::ok(decompile_bytecode(&bytecode, 203)),
                Err(_) => Response::error("invalid bytecode (base64 expected)", 400),
            }
        })
        .run(req, env)
        .await
}
