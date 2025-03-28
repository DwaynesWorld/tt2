use std::ops::Index;
use std::process;
use std::time::Duration;

use paho_mqtt::{self as mqtt, AsyncClient, Message};
use tokio::signal;

const MAX_RECONNECT_ATTEMPTS: usize = 10;
const PUB_TOPIC: &str = "pong";
const SUB_TOPIC: &str = "ping/+";
const QOS: i32 = 1;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let host = "tcp://localhost:18830";
    println!("Connecting to the MQTT server at '{}'...", host);

    let options = mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id("server")
        .finalize();

    let client = mqtt::AsyncClient::new(options).unwrap_or_else(|err| {
        eprintln!("Error creating the client: {}", err);
        process::exit(1);
    });

    let c = client.clone();
    let listener = tokio::spawn(listen(c));
    let (ctrl_c, terminate) = create_shutdown_signals();

    tokio::select! {
        _ = listener => {},
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
    client.disconnect(None).await?;

    Ok(())
}

async fn listen(client: AsyncClient) -> anyhow::Result<()> {
    let mut client = client;
    let stream = client.get_stream(100);

    let options = mqtt::ConnectOptionsBuilder::new_v3()
        .keep_alive_interval(Duration::from_secs(30))
        .clean_session(false)
        .finalize();

    client.connect(options).await?;
    client.subscribe(SUB_TOPIC, QOS).await?;
    println!("Subscribing to topics: {:?}", SUB_TOPIC);

    while let Ok(result) = stream.recv().await {
        if let Some(message) = result {
            println!("{}", message);
            let topic = message.topic();
            let parts: Vec<&str> = topic.split('/').collect();
            let greeting = parts.index(1);

            client
                .publish(Message::new(PUB_TOPIC, greeting.as_bytes(), QOS))
                .await?;
        } else {
            let _ = reconnect(&client).await;
        }
    }

    Ok(())
}

async fn reconnect(client: &AsyncClient) -> anyhow::Result<()> {
    println!("Lost connection. Attempting reconnect...");

    let mut reconnects = 0;

    while let Err(err) = client.reconnect().await {
        reconnects += 1;
        println!("Error reconnecting on attempt {}: {}", reconnects, err);
        tokio::time::sleep(Duration::from_secs(1)).await;

        // if reconnects > MAX_RECONNECT_ATTEMPTS {
        //     return Err(format!(
        //         "Failed to reconnecting after {} attempts.",
        //         MAX_RECONNECT_ATTEMPTS
        //     ));
        // }
    }

    println!("Reconnected.");
    Ok(())
}

fn create_shutdown_signals() -> (impl Future<Output = ()>, impl Future<Output = ()>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    (ctrl_c, terminate)
}
