use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use clap::Parser;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    config_file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    ipmi_address: String,
    username: String,
    password: String,
    listen_port: u16,
}
impl Config {
    fn from_yaml_file(file: &str) -> anyhow::Result<Self> {
        let file = std::fs::File::open(file)?;
        let reader = std::io::BufReader::new(file);
        let config = serde_yaml::from_reader(reader)?;
        Ok(config)
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = Config::from_yaml_file(&args.config_file).expect("Failed to read config file");
    let app = Router::new()
        .route("/status", get(get_power_status))
        .route("/on", post(power_on))
        .route("/off", post(power_off))
        .with_state(config.clone())
        .fallback(default_404);
    let addr = format!("0.0.0.0:{}", config.listen_port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
    info!("Server started on port {}", config.listen_port);
}
enum PowerAction {
    On,
    Off,
    Status,
}
enum PowerStatus {
    On,
    Off,
}
fn power_action(action: PowerAction, config: &Config) -> Option<PowerStatus> {
    let action_str = match action {
        PowerAction::On => "on".to_string(),
        PowerAction::Off => "off".to_string(),
        PowerAction::Status => "status".to_string(),
    };
    let command = format!(
        "ipmitool -I lanplus -H {} -U {} -P {} power {}",
        config.ipmi_address, config.username, config.password, action_str
    );
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("Failed to run command");
    if !output.status.success() {
        error!(
            "Failed to run command: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
    let command_out = output.stdout;
    let output = String::from_utf8_lossy(&command_out);
    let output = output.trim();
    match output {
        "Chassis Power is on" => Some(PowerStatus::On),
        "Chassis Power is off" => Some(PowerStatus::Off),
        _ => {
            warn!("Unexpected output from ipmitool: {}", output);
            None
        }
    }
}

async fn get_power_status(State(config): State<Config>) -> impl IntoResponse {
    info!("Got request for power status");
    let resp = match power_action(PowerAction::Status, &config) {
        Some(PowerStatus::On) => (axum::http::StatusCode::OK, "on"),
        Some(PowerStatus::Off) => (axum::http::StatusCode::OK, "off"),
        None => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "error"),
    };
    info!("Returning status: {}", resp.1);
    resp
}
async fn power_on(State(config): State<Config>) -> String {
    info!("Got request to power on");
    let _status = power_action(PowerAction::On, &config);

    return "ok".to_string();
}
async fn power_off(State(config): State<Config>) -> String {
    info!("Got request to power off");
    let _status = power_action(PowerAction::Off, &config);
    return "ok".to_string();
}
async fn default_404() -> impl IntoResponse {
    info!("Got request for unknown path");
    axum::http::StatusCode::NOT_FOUND
}
