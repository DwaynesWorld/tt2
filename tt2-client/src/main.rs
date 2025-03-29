use std::sync::atomic::Ordering;
use std::time::Duration;
use std::{process, sync::atomic::AtomicBool};

use log::{error, info, warn};
use paho_mqtt::{self as mqtt, AsyncClient, AsyncReceiver, Message};
use rand::seq::IndexedRandom;
use tt2_core::{create_shutdown_signals, logger};

const PUB_TOPIC: &str = "ping";
const SUB_TOPIC: &str = "pong";
const QOS: i32 = 1;

static SHUTDOWN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logger::init(&logger::Level::Info);

    let host = "tcp://localhost:1883";
    info!("Connecting to the MQTT server at '{}'...", host);

    let options = mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id("tt2_client")
        .finalize();

    let mut client = mqtt::AsyncClient::new(options).unwrap_or_else(|err| {
        error!("Error creating the client: {}", err);
        process::exit(1);
    });

    let stream = client.get_stream(1000);

    let options = mqtt::ConnectOptionsBuilder::new_v3()
        .keep_alive_interval(Duration::from_secs(30))
        .clean_session(true)
        .finalize();

    client.connect(options).await?;

    let c = client.clone();
    let listener = tokio::spawn(listen(c, stream));

    let c = client.clone();
    let ping = tokio::spawn(ping(c));

    let (ctrl_c, terminate) = create_shutdown_signals();

    tokio::select! {
        _ = listener => {},
        _ = ping => {},
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("signal received, starting graceful shutdown");
    SHUTDOWN_IN_PROGRESS.store(true, Ordering::SeqCst);

    client.disconnect(None).await?;

    Ok(())
}

async fn ping(client: AsyncClient) -> anyhow::Result<()> {
    client.subscribe(SUB_TOPIC, QOS).await?;
    info!("Subscribing to topics: {:?}", SUB_TOPIC);

    let greetings: Vec<&str> = vec!["Hello", "Hi", "Hey", "Greetings", "Salutations", "Howdy"];

    loop {
        let greeting = greetings.choose(&mut rand::rng()).unwrap();

        if !client.is_connected() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        info!("Sending ping: {}", greeting);

        client
            .publish(Message::new(
                format!("{}/{}", PUB_TOPIC, greeting),
                vec![],
                QOS,
            ))
            .await?;

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn listen(client: AsyncClient, stream: AsyncReceiver<Option<Message>>) -> anyhow::Result<()> {
    let mut client = client;
    let mut stream = stream;

    client.subscribe(SUB_TOPIC, QOS).await?;
    info!("Subscribing to topics: {:?}", SUB_TOPIC);

    loop {
        match stream.recv().await {
            Ok(result) if result.is_some() => {
                let message = result.unwrap();
                info!("Received pong: {}", message.payload_str());
            }
            e => {
                error!("err is_connected={} e={:?}", client.is_connected(), e);
                let _ = reconnect(&client).await;
                stream = client.get_stream(1000);
                client.subscribe(SUB_TOPIC, QOS).await?;
            }
        }
    }
}

async fn reconnect(client: &AsyncClient) -> anyhow::Result<()> {
    if SHUTDOWN_IN_PROGRESS.load(Ordering::SeqCst) {
        return Ok(());
    }

    warn!("Lost connection. Attempting reconnect...");
    let mut reconnects = 0;

    loop {
        match client.reconnect().await {
            Ok(r) => {
                info!("Reconnected. {:?}", r);
                break;
            }
            Err(err) => {
                reconnects += 1;
                error!("Error reconnecting on attempt {}: {}", reconnects, err);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}
