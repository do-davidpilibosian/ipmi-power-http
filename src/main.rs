use axum::{
    body::Body,
    extract::{Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_auth::AuthBearer;
use clap::Parser;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    config_file: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    listen_port: u16,
    groups: Vec<Group>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Group {
    name: String,
    token: String,
    endpoints: Vec<IpmiEndpoint>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct IpmiEndpoint {
    name: String,
    ipmi_address: String,
    username: String,
    password: String,
}

impl Config {
    fn from_yaml_file(file: &str) -> anyhow::Result<Self> {
        let file = std::fs::File::open(file)?;
        let reader = std::io::BufReader::new(file);
        let config = serde_yaml::from_reader(reader)?;
        Ok(config)
    }

    // Method to get a group by its token
    fn get_group_by_token(&self, token: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.token == token)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PowerControlMsg {
    action: String,
}

#[derive(Debug)]
enum PowerAction {
    On,
    Off,
    Reset,
    Cycle,
    Status,
}

#[derive(Debug)]
enum PowerStatus {
    On,
    Off,
    Reset,
    Cycle,
}

#[derive(Debug)]
enum PowerError {
    CommandNotSupported,
    InvalidState,
    AuthenticationFailed,
    ConnectionFailed,
    UnexpectedOutput,
    UnknownError,
}

enum ActionContext {
    Status,
    Control,
}

fn power_action(action: PowerAction, endpoint: &IpmiEndpoint) -> Result<PowerStatus, PowerError> {
    let action_str = match action {
        PowerAction::On => "on",
        PowerAction::Off => "off",
        PowerAction::Reset => "reset",
        PowerAction::Cycle => "cycle",
        PowerAction::Status => "status",
    };
    let command = format!(
        "ipmitool -I lanplus -H {} -U {} -P '{}' power {}",
        endpoint.ipmi_address, endpoint.username, endpoint.password, action_str
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to run command: {}", stderr);

        // Map stderr to PowerError
        let error = if stderr.contains("Command not supported in present state") {
            PowerError::CommandNotSupported
        } else if stderr.contains("Invalid user name") || stderr.contains("authentication failure") {
            PowerError::AuthenticationFailed
        } else if stderr.contains("Unable to establish IPMI v2 / RMCP+ session") {
            PowerError::ConnectionFailed
        } else if stderr.contains("Invalid command") {
            PowerError::InvalidState
        } else {
            PowerError::UnknownError
        };
        return Err(error);
    }

    let command_out = output.stdout;
    let output = String::from_utf8_lossy(&command_out);
    let output = output.trim();

    let status = match output {
        // Status outputs
        "Chassis Power is on" => PowerStatus::On,
        "Chassis Power is off" => PowerStatus::Off,
        // Action outputs
        "Chassis Power Control: Up/On" => PowerStatus::On,
        "Chassis Power Control: Down/Off" => PowerStatus::Off,
        "Chassis Power Control: Reset" => PowerStatus::Reset,
        "Chassis Power Control: Cycle" => PowerStatus::Cycle,
        "Chassis Power Control: Soft" => PowerStatus::Off,
        "Chassis Power Control: On" => PowerStatus::On,
        "Chassis Power Control: Off" => PowerStatus::Off,
        _ => {
            warn!("Unexpected output from ipmitool: '{}'", output);
            return Err(PowerError::UnexpectedOutput);
        }
    };

    Ok(status)
}

fn handle_power_action_result(
    result: Result<PowerStatus, PowerError>,
    endpoint_id: &str,
    context: ActionContext,
) -> Response<Body> {
    match result {
        Ok(status) => match context {
            ActionContext::Status => {
                info!("Returning status for {}: {:?}", endpoint_id, status);
                let status_str = match status {
                    PowerStatus::On => "on",
                    PowerStatus::Off => "off",
                    PowerStatus::Reset => "reset",
                    PowerStatus::Cycle => "cycle",
                };
                (StatusCode::OK, Json(json!({ "status": status_str }))).into_response()
            }
            ActionContext::Control => {
                match status {
                    PowerStatus::On => info!("Power is on"),
                    PowerStatus::Off => info!("Power is off"),
                    PowerStatus::Reset => info!("Power reset"),
                    PowerStatus::Cycle => info!("Power cycle"),
                }
                (StatusCode::OK, "ok").into_response()
            }
        },
        Err(err) => map_power_error_to_response(err),
    }
}

fn map_power_error_to_response(err: PowerError) -> Response<Body> {
    let error_message = match err {
        PowerError::CommandNotSupported => "Command not supported in present state",
        PowerError::InvalidState => "Invalid system state for this command",
        PowerError::AuthenticationFailed => "Authentication failed",
        PowerError::ConnectionFailed => "Unable to connect to IPMI endpoint",
        PowerError::UnexpectedOutput => "Unexpected response from IPMI endpoint",
        PowerError::UnknownError => "An unknown error occurred",
    };
    error!("Power action error: {:?}", err);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": error_message })),
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    // Setup logger
    env_logger::init();
    let args = Args::parse();
    let config =
        Config::from_yaml_file(&args.config_file).expect("Failed to read config file");
    let app = Router::new()
        .route("/power/:endpoint_id", get(get_power_status))
        .route("/power/:endpoint_id", post(power_control))
        .with_state(config.clone())
        .fallback(default_404);
    let addr = format!("0.0.0.0:{}", config.listen_port);
    info!("Server started on port {}", config.listen_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

async fn get_power_status(
    State(config): State<Config>,
    AuthBearer(token): AuthBearer,
    Path(endpoint_id): Path<String>,
) -> impl IntoResponse {
    info!("Got request for power status of endpoint {}", endpoint_id);

    // Use the helper function to get the endpoint
    let endpoint = match get_endpoint_from_token(&config, &token, &endpoint_id) {
        Ok(endpoint) => endpoint,
        Err(response) => return response,
    };

    // Proceed with the power action
    let result = power_action(PowerAction::Status, endpoint);
    handle_power_action_result(result, &endpoint_id, ActionContext::Status)
}

async fn power_control(
    State(config): State<Config>,
    AuthBearer(token): AuthBearer,
    Path(endpoint_id): Path<String>,
    Json(payload): Json<PowerControlMsg>,
) -> impl IntoResponse {
    info!("Got request to power control endpoint {}", endpoint_id);
    info!("Token: {}", token);

    // Use the helper function to get the endpoint
    let endpoint = match get_endpoint_from_token(&config, &token, &endpoint_id) {
        Ok(endpoint) => endpoint,
        Err(response) => return response,
    };

    let action = match payload.action.as_str() {
        "on" => PowerAction::On,
        "off" => PowerAction::Off,
        "reset" => PowerAction::Reset,
        "cycle" => PowerAction::Cycle,
        _ => {
            warn!("Invalid action: {}", payload.action);
            return (StatusCode::BAD_REQUEST, "Invalid action").into_response();
        }
    };

    let result = power_action(action, endpoint);
    handle_power_action_result(result, &endpoint_id, ActionContext::Control)
}

fn get_endpoint_from_token<'a>(
    config: &'a Config,
    token: &str,
    endpoint_id: &str,
) -> Result<&'a IpmiEndpoint, Response<Body>> {
    // Find the group associated with the token
    let group = match config.get_group_by_token(token) {
        Some(group) => group,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid token",
            )
                .into_response());
        }
    };

    // Find the endpoint in the group
    let endpoint = match group.endpoints.iter().find(|e| e.name == endpoint_id) {
        Some(e) => e,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                format!("Endpoint '{}' not found", endpoint_id),
            )
                .into_response());
        }
    };

    Ok(endpoint)
}

async fn default_404() -> impl IntoResponse {
    info!("Got request for unknown path");
    StatusCode::NOT_FOUND
}

