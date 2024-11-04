# IPMI Power Control HTTP API

A Rust application that provides an HTTP API for controlling power operations (`on`, `off`, `reset`, `cycle`) on IPMI-enabled devices. The application supports multiple endpoints grouped by tokens, allowing for secure and organized access control.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
    - [Starting the Server](#starting-the-server)
    - [API Endpoints](#api-endpoints)
        - [Authentication](#authentication)
        - [Get Power Status](#get-power-status)
        - [Power Control Actions](#power-control-actions)
- [Examples](#examples)
    - [Get Power Status Example](#get-power-status-example)
    - [Power Control Example](#power-control-example)
- [Logging](#logging)
- [Security Considerations](#security-considerations)
- [Deployment](#deployment)
- [License](#license)

## Features

- **Multiple IPMI Endpoints**: Manage power operations on multiple IPMI-enabled devices.
- **Group-Based Access Control**: Organize endpoints into groups, each secured with a unique token.
- **Supported Power Actions**: `on`, `off`, `reset`, `cycle`, and `status`.
- **Standardized Error Responses**: Returns consistent error messages without exposing sensitive information.
- **Logging**: Detailed server-side logging for troubleshooting.

## Prerequisites

- **Rust**: Install Rust (version 1.54 or higher recommended) from [rust-lang.org](https://www.rust-lang.org/tools/install).
- **ipmitool**: Ensure `ipmitool` is installed on the server where the application will run.
- **Cargo**: Comes with Rust installation; used for building and running the application.

## Installation

1. **Clone the Repository**:

   ```bash
   git clone https://github.com/yourusername/ipmi-power-http.git
   cd ipmi-power-http
   ```

2. **Set Up Dependencies**:

   Ensure your `Cargo.toml` has the following dependencies:

   ```toml
   [dependencies]
   axum = "0.6"
   axum-auth = "0.3"
   tokio = { version = "1", features = ["full"] }
   clap = { version = "4", features = ["derive"] }
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   serde_yaml = "0.9"
   env_logger = "0.10"
   log = "0.4"
   ```

3. **Build the Application**:

   ```bash
   cargo build --release
   ```

## Configuration

Create a YAML configuration file (e.g., `config.yaml`) with the following structure:

```yaml
listen_port: 8080
groups:
  - name: "group1"
    token: "your_token_for_group1"
    endpoints:
      - name: "endpoint1"
        ipmi_address: "192.168.1.100"
        username: "admin"
        password: "password"
      - name: "endpoint2"
        ipmi_address: "192.168.1.101"
        username: "admin"
        password: "password"
  - name: "group2"
    token: "your_token_for_group2"
    endpoints:
      - name: "endpoint3"
        ipmi_address: "192.168.1.102"
        username: "admin"
        password: "password"
```

**Configuration Parameters**:

- `listen_port`: The port on which the server will listen.
- `groups`: A list of groups, each with:
    - `name`: A friendly name for the group.
    - `token`: A unique token used for authenticating requests to this group's endpoints.
    - `endpoints`: A list of IPMI endpoints within the group.
        - `name`: A unique identifier for the endpoint.
        - `ipmi_address`: The IP address or hostname of the IPMI interface.
        - `username`: Username for IPMI authentication.
        - `password`: Password for IPMI authentication.

**Note**: Keep the configuration file secure, as it contains sensitive information.

## Usage

### Starting the Server

Run the application, specifying the path to your configuration file:

```bash
./target/release/ipmi-power-http --config-file config.yaml
```

**Command-Line Arguments**:

- `--config-file`: Path to the YAML configuration file.

### API Endpoints

The application provides two main endpoints for each IPMI endpoint:

- `GET /power/:endpoint_id`: Get the power status of the endpoint.
- `POST /power/:endpoint_id`: Perform a power control action on the endpoint.

#### Authentication

All endpoints require a Bearer Token for authentication, provided in the `Authorization` header:

```
Authorization: Bearer your_token_here
```

Use the token associated with the group that contains the endpoint you are accessing.

#### Get Power Status

- **Endpoint**: `GET /power/:endpoint_id`
- **Description**: Retrieves the current power status of the specified endpoint.
- **Response**:

    - **Success (200 OK)**:

      ```json
      { "status": "on" }
      ```

      or

      ```json
      { "status": "off" }
      ```

    - **Error (4xx or 5xx)**:

      ```json
      { "error": "Error message" }
      ```

#### Power Control Actions

- **Endpoint**: `POST /power/:endpoint_id`
- **Description**: Performs a power control action (`on`, `off`, `reset`, `cycle`) on the specified endpoint.
- **Request Body**:

  ```json
  { "action": "on" }  // or "off", "reset", "cycle"
  ```

- **Response**:

    - **Success (200 OK)**:

      ```
      ok
      ```

    - **Error (4xx or 5xx)**:

      ```json
      { "error": "Error message" }
      ```

## Examples

### Get Power Status Example

**Request**:

```http
GET /power/endpoint1 HTTP/1.1
Host: your_server_address
Authorization: Bearer your_token_for_group1
```

**Response**:

```json
{ "status": "on" }
```

### Power Control Example

**Request**:

```http
POST /power/endpoint1 HTTP/1.1
Host: your_server_address
Authorization: Bearer your_token_for_group1
Content-Type: application/json

{
  "action": "reset"
}
```

**Response**:

```
ok
```

**Error Response Example**:

```json
{ "error": "Command not supported in present state" }
```

## Logging

The application uses `env_logger` for logging. Logs include informational messages, warnings, and errors, which can be helpful for troubleshooting.

**Sample Log Output**:

```
INFO  Server started on port 8080
INFO  Got request for power status of endpoint endpoint1
INFO  Returning status for endpoint1: PowerStatus::On
```

**Log Levels**:

- `INFO`: General operational information about the application's state.
- `WARN`: Indications of potential issues or unexpected behavior.
- `ERROR`: Errors that occur during the execution of power actions or other operations.

## Security Considerations

- **Authentication Tokens**: Use strong, randomly generated tokens for group access. Avoid sharing tokens across groups.
- **Configuration File**: The configuration file contains sensitive information (tokens, IPMI credentials). Ensure it has appropriate file permissions and is not accessible to unauthorized users.
- **Network Security**: Run the application behind a firewall or reverse proxy. Use HTTPS to encrypt traffic, especially if transmitting over untrusted networks.
- **Error Messages**: The application returns standardized error messages to avoid exposing sensitive information. Server logs contain detailed errors for administrative purposes.

## Deployment

A production deployment configuration for serving ipmi-power-http via Caddy has been included via docker-compose.

create a configuration file for ipmi-power-http and modify the included docker-compose.yml file to point to it, then run
```bash
docker-compose up
```
and test. If everything works as expected daemonize docker compose
```bash
docker-compose up -d
```

## License

This project is licensed under the [MIT License](LICENSE).

---

**Disclaimer**: Use this application responsibly and ensure compliance with your organization's security policies and any relevant regulations. The authors are not responsible for any misuse or damages resulting from the use of this application.