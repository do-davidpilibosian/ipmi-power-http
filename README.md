# IPMI Power Control API
This project provides a simple web service for controlling and querying the power state of a server using IPMI (Intelligent Platform Management Interface). The service is built using Rust, Axum, and Tokio for asynchronous operation. It reads configuration from a YAML file and exposes endpoints for power control and status checking.
It is intended to use with home-assistant.

## Features
Query power status of a server.
Control power state (turn on/off) of a server.
Authentication using tokens specified in the configuration file.
## Requirements
- Rust
- ipmitool installed on the system
## Configuration
Also see repo.
The service requires a YAML configuration file with the following structure:

```yaml
ipmi_address: "192.168.1.100"
username: "admin"
password: "password"
listen_port: 8080
tokens:
  - "your-secret-token"
  - "another-secret-token"
```

## Example Home Assistant Config
Also see repo.
```yaml
switch:
  - platform: rest
    name: My Beefy Server
    resource: http://127.0.0.1:6677/power
    body_on: '{"action": "on"}'
    body_off: '{"action": "off"}'
    is_on_template: "{{ value_json.is_on }}"
    headers:
      Content-Type: application/json
      Authorization: Bearer a_very_secure_token
```
## Running the Service
Create a configuration file (e.g., config.yaml) with the above structure.
Build and run the service:
```bash
cargo run -- --config-file config.yaml
```
The server will start and listen on the specified port.

## API Endpoints
 - GET /power
    Query the current power status of the server.

    Request:

    ```bash
    curl -X GET http://localhost:8080/power
    ```
    Response:

    200 OK with JSON {"is_on": true} or {"is_on": false}
    500 Internal Server Error if there's an issue querying the power status
 - POST /power
    Control the power state of the server. Requires an authentication token.

    Request:

    ```bash
    curl -X POST http://localhost:8080/power \
    -H "Authorization: Bearer your-secret-token" \
    -H "Content-Type: application/json" \
    -d '{"action": "on"}'
    ```
    action can be `on` or `off`.

    Response:
    200 OK with text ok if the action is successful
    400 Bad Request if the action is invalid
    401 Unauthorized if the token is not in the configuration
    500 Internal Server Error if there's an issue performing the action
    404 Default
    All other routes return a 404 Not Found.

## Logging
The service uses env_logger for logging. Ensure you have the environment variable RUST_LOG set to the appropriate log level (e.g., info, debug) to see logs.


```bash
RUST_LOG=info cargo run -- --config-file config.yaml
```

## License
This project is licensed under the MIT License.

## And
I am lazy so used chatgpt to generate the readme.