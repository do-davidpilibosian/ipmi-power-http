use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use axum_auth::AuthBearer;
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
    tokens: Vec<String>,
}
impl Config {
    fn from_yaml_file(file: &str) -> anyhow::Result<Self> {
        let file = std::fs::File::open(file)?;
        let reader = std::io::BufReader::new(file);
        let config = serde_yaml::from_reader(reader)?;
        Ok(config)
    }
    fn validate_token(&self, token: &str) -> bool {
        self.tokens.contains(&token.to_string())
    }
}

#[tokio::main]
async fn main() {
    // setup logger
    env_logger::init();
    let args = Args::parse();
    let config = Config::from_yaml_file(&args.config_file).expect("Failed to read config file");
    let app = Router::new()
        .route("/power", get(get_power_status))
        .route("/power", post(power_control))
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

#[derive(Serialize, Deserialize, Debug)]
struct PowerControlMsg {
    action: String,
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
        "Chassis Power Control: Up/On" => Some(PowerStatus::On),
        "Chassis Power Control: Soft" => Some(PowerStatus::Off),
        _ => {
            warn!("Unexpected output from ipmitool: {}", output);
            None
        }
    }
}

async fn get_power_status(State(config): State<Config>) -> impl IntoResponse {
    info!("Got request for power status");
    let resp = match power_action(PowerAction::Status, &config) {
        Some(PowerStatus::On) => (StatusCode::OK, "on"),
        Some(PowerStatus::Off) => (StatusCode::OK, "off"),
        None => (StatusCode::INTERNAL_SERVER_ERROR, "error"),
    };
    info!("Returning status: {}", resp.1);
    resp
}

async fn power_control(
    State(config): State<Config>,
    AuthBearer(token): AuthBearer,
    Json(payload): Json<PowerControlMsg>,
) -> impl IntoResponse {
    info!("Got request to power on");
    info!("Token: {}", token);
    if !config.validate_token(&token) {
        return (StatusCode::UNAUTHORIZED, "token not in config");
    };
    let action = match payload.action.as_str() {
        "on" => PowerAction::On,
        "off" => PowerAction::Off,
        _ => {
            warn!("Invalid action: {}", payload.action);
            return (StatusCode::BAD_REQUEST, "error");
        }
    };
    match power_action(action, &config) {
        Some(PowerStatus::On) => info!("Power is on"),
        Some(PowerStatus::Off) => info!("Power is off"),
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "error"),
    }
    (StatusCode::OK, "ok")
}
async fn default_404() -> impl IntoResponse {
    info!("Got request for unknown path");
    StatusCode::NOT_FOUND
}
