
use librespot_connect::spirc::SpircCommand;
use librespot_playback::player::PlayerEventChannel;
use log::{info, warn};
use paho_mqtt;
use std::thread;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct MQTTHandler {
    mqtt_client: paho_mqtt::AsyncClient,
    topic: String,
}

impl MQTTHandler {
    // TODO: options?
    const QOS: i32 = 0;
    pub fn new(
        create_opts: paho_mqtt::CreateOptions,
        conn_opts: paho_mqtt::ConnectOptions,
        topic: String,
    ) -> Self {

        let mqtt_client = paho_mqtt::AsyncClient::new(create_opts).unwrap();
        let cloned_topic = topic.clone();
        mqtt_client.set_connected_callback(move |mqtt_client| { mqtt_client.subscribe(format!("{}/sub", cloned_topic), Self::QOS); });
        mqtt_client.connect(conn_opts);


        Self {
            mqtt_client,
            topic
        }
    }

    pub fn run(
        mut self,
        mut player_events: PlayerEventChannel,
        spirc_commands: UnboundedSender<SpircCommand>
    ) {

        let subscribe_stream = self.mqtt_client.get_stream(1);
        thread::spawn(move|| loop {
            if let Some(event) = player_events.blocking_recv() {
                let event_fields = event.get_player_event_fields().unwrap();
                self.mqtt_client.publish(paho_mqtt::Message::new(format!("{}/pub", self.topic), serde_json::to_vec(&event_fields).unwrap(), Self::QOS));
            }
        });
        thread::spawn(move || loop {
            if let Ok(Some(message)) = subscribe_stream.recv_blocking() {
                info!("{}", message);
                match String::from_utf8(message.payload().to_vec()) {
                    Err(_) => warn!("Could not parse mqtt message!"),
                    Ok(message) => {
                        if let Ok(spirc_command) = message.as_str().try_into() {
                            spirc_commands.send(spirc_command).unwrap();
                        }
                    }
                }
            }
        });
    }
}
