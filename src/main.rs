// use mdns-sd to find my elgato keylight air

use std::net::Ipv4Addr;

use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Copy)]
#[repr(u8)]
enum Power {
    Off = 0,
    On = 1,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct Settings {
    on: Power,
    brightness: u8,
    temperature: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LightState {
    number_of_lights: u8,
    lights: Vec<Settings>,
}

fn gen_url(address: Ipv4Addr) -> String {
    format!("http://{}:9123/elgato/lights", address)
}

async fn get_lights_status(addresses: &[Ipv4Addr]) -> Result<Vec<LightState>, reqwest::Error> {
    let mut statuses: Vec<LightState> = Vec::new();

    for address in addresses.iter().cloned() {
        // Get the current status of each light
        // this allows us to fill the struct with the current values
        // The API requires that we send the entire struct back
        let url = gen_url(address);

        match reqwest::get(url).await?.json::<LightState>().await {
            Ok(status) => statuses.push(status),
            Err(error) => panic!("Problem getting light status: {:?}", error),
        };
    }

    Ok(statuses)
}

fn get_service_addresses(service_type: &str) -> Vec<Ipv4Addr> {
    // Create a daemon
    let mdns: ServiceDaemon = ServiceDaemon::new().expect("Failed to create daemon");

    // Browse for a service type.
    let receiver: mdns_sd::Receiver<ServiceEvent> =
        mdns.browse(service_type).expect("Failed to browse");

    let mut service_found = false;
    let mut services: Vec<Ipv4Addr> = Vec::new();
    while let Ok(event) = receiver.recv() {
        match event {
            ServiceEvent::ServiceFound(..) => {
                service_found = true;
            }
            ServiceEvent::ServiceResolved(info) => {
                let addresses = info.get_addresses();
                // Append the service address to the list of services
                if let Some(address) = addresses.iter().next().cloned() {
                    services.push(address);
                }
            }
            ServiceEvent::SearchStarted(..) => {
                // If we found a service and we're still searching, we can stop
                if service_found {
                    break;
                }
            }
            _ => {}
        }
    }

    services
}

async fn update_lights_state(addresses: Vec<Ipv4Addr>, statuses: Vec<LightState>) {
    for (address, status) in addresses.iter().zip(statuses.iter()) {
        // Make a PUT request to toggle the light power
        reqwest::Client::new()
            .put(&gen_url(*address))
            .json(&status)
            .send()
            .await
            .ok();
    }
}

fn toggle_light_state(statuses: Vec<LightState>) -> Vec<LightState> {
    let mut local_statuses = statuses;

    for status in local_statuses.iter_mut() {
        // Toggle the light power state
        match status.lights[0].on {
            Power::On => status.lights[0].on = Power::Off,
            Power::Off => status.lights[0].on = Power::On,
        }
    }

    local_statuses
}

#[tokio::main]
async fn main() {
    let addresses = get_service_addresses("_elg._tcp.local.");
    let statuses = get_lights_status(&addresses).await;

    match statuses {
        Ok(statuses) => {
            update_lights_state(addresses, toggle_light_state(statuses)).await;
        }
        Err(error) => panic!("Problem getting light status: {:?}", error),
    }
}
