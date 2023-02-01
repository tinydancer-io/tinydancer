#[macro_export]
macro_rules! convert_to_websocket {
    ($test:expr) => {
        if $test.contains("https") {
            $test.replace("https://", "wss://")
        } else {
            String::from("ws://0.0.0.0:8900")
        }
    };
}
#[macro_export]
macro_rules! send_rpc_call {
    ($url:expr, $body:expr) => {{
        let req_client = reqwest::Client::new();
        let res = req_client
            .post($url)
            .body(body)
            .send()
            .await?
            .text()
            .await?;
        res
    }};
}
