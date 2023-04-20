// use mdns-sd to find my elgato keylight air

use std::net::Ipv4Addr;

use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum Power {
    On,
    Off,
}

impl From<Power> for u8 {
    fn from(f: Power) -> u8 {
        match f {
            Power::On => 1,
            Power::Off => 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    on: Power,
    brightness: u8,
    temperature: u16,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Command {
    number_of_lights: u8,
    lights: Vec<Settings>,
}

fn gen_url(address: Ipv4Addr) -> String {
    format!("http://{}:9123/elgato/lights", address)
}

async fn get_light_status(address: Ipv4Addr) -> Result<Command, reqwest::Error>{
    let url = gen_url(address);
    reqwest::get(url)
        .await?
        .json::<Command>()
        .await
}

#[tokio::main]
async fn main() {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    // Browse for a service type.
    let service_type = "_elg._tcp.local.";
    let receiver = mdns.browse(service_type).expect("Failed to browse");

    while let Ok(event) = receiver.recv() {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let addresses = info.get_addresses();
                if let Some(address) = addresses.iter().next().cloned() {
                    println!("Found Elgato Keylight Air at {}", address);

                    // Get the current status of the light
                    // this allows us to fill the struct with the current values
                    // The API requires that we send the entire struct back
                    let mut status: Command = match get_light_status(address).await {
                        Ok(status) => status,
                        Err(error) => panic!("Problem getting light status: {:?}", error),
                    };
                    println!("{:?}", status);

                    // Toggle the light power state
                    match status.lights[0].on {
                        Power::On => { status.lights[0].on = Power::Off },
                        Power::Off => { status.lights[0].on = Power::On },
                    }
                    println!("Lights toggled: {:?}", status.lights[0].on);

                    // Make a PUT request to toggle the light power
                    reqwest::Client::new()
                        .put(&gen_url(address))
                        .json(&status)
                        .send()
                        .await
                        .ok();
                }
                return;
            }
            _ => {}
        }
    }
}
