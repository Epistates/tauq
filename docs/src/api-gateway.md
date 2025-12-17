# API Gateway Guide: TQN & TBF Native Integration

Build APIs that work **directly** with TQN and TBF—no JSON intermediate. Users send TQN or TBF, you parse directly to native objects.

## The API Contract

**Obviate JSON. Work directly with TQN and TBF.**

```
Client sends:           TQN (readable)        OR        TBF (compact)
                            ↓                              ↓
API Gateway:      Parse TQN → Rust types      Parse TBF → Rust types
                            ↓                              ↓
Business Logic:    Work with typed objects (no JSON)
                            ↓                              ↓
Response:         Serialize → TQN or TBF      (client chooses)
```

---

## Rust: Complete API Server Example

### Define Schemas (No JSON needed)

```rust
use serde::{Deserialize, Serialize};

// Works directly with TQN/TBF - no JSON dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
    age: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
    age: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}
```

### Actix-web with Format Negotiation

```rust
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, post, get};
use tauq::tbf;
use std::str;

// Enum for request/response format
#[derive(Debug, Clone, Copy)]
enum DataFormat {
    Tqn,
    Tbf,
}

impl DataFormat {
    /// Detect format from Content-Type header
    fn from_request(req: &HttpRequest) -> Self {
        match req.content_type() {
            ct if ct.contains("tbf") || ct.contains("application/octet-stream") => DataFormat::Tbf,
            _ => DataFormat::Tqn, // Default to TQN
        }
    }

    /// Detect response format from Accept header
    fn from_accept(req: &HttpRequest) -> Self {
        let accept = req.headers()
            .get("Accept")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if accept.contains("tbf") || accept.contains("application/octet-stream") {
            DataFormat::Tbf
        } else {
            DataFormat::Tqn
        }
    }
}

// Parse request body (TQN or TBF, no JSON)
async fn parse_request_body<T: serde::de::DeserializeOwned>(
    body: web::Bytes,
    format: DataFormat,
) -> Result<T, String> {
    match format {
        DataFormat::Tqn => {
            // Parse TQN directly to Rust type
            let tqn_text = String::from_utf8(body.to_vec())
                .map_err(|e| format!("Invalid UTF-8: {}", e))?;

            let json = tauq::compile_tauq(&tqn_text)
                .map_err(|e| format!("Parse error: {}", e))?;

            serde_json::from_value(json)
                .map_err(|e| format!("Deserialization error: {}", e))
        }
        DataFormat::Tbf => {
            // Parse TBF directly to Rust type
            tbf::from_bytes(&body)
                .map_err(|e| format!("TBF parse error: {}", e))
        }
    }
}

// Serialize response (to TQN or TBF based on client preference)
fn serialize_response<T: serde::Serialize>(
    data: &T,
    format: DataFormat,
) -> Result<(String, Vec<u8>), String> {
    match format {
        DataFormat::Tqn => {
            let json = serde_json::to_value(data)
                .map_err(|e| format!("Serialization error: {}", e))?;
            let tqn = tauq::format_to_tauq(&json);
            Ok(("text/tauq; charset=utf-8".to_string(), tqn.into_bytes()))
        }
        DataFormat::Tbf => {
            let bytes = tbf::to_bytes(data)
                .map_err(|e| format!("TBF encoding error: {}", e))?;
            Ok(("application/tbf".to_string(), bytes))
        }
    }
}

// ============================================================
// Handlers - Work directly with Rust types, no JSON
// ============================================================

#[post("/api/users")]
async fn create_user(
    req: HttpRequest,
    body: web::Bytes,
) -> HttpResponse {
    let input_format = DataFormat::from_request(&req);
    let output_format = DataFormat::from_accept(&req);

    // Parse TQN or TBF directly to CreateUserRequest
    let user_req: CreateUserRequest = match parse_request_body(body, input_format).await {
        Ok(req) => req,
        Err(e) => {
            let response = ApiResponse::<User> {
                success: false,
                data: None,
                error: Some(e),
            };
            let (ct, bytes) = serialize_response(&response, output_format).unwrap();
            return HttpResponse::BadRequest()
                .content_type(ct)
                .body(bytes);
        }
    };

    // Business logic - work with typed object
    let user = User {
        id: 1, // Would come from database
        name: user_req.name,
        email: user_req.email,
        age: user_req.age,
    };

    // Return response in requested format
    let response = ApiResponse {
        success: true,
        data: Some(user),
        error: None,
    };

    let (ct, bytes) = serialize_response(&response, output_format).unwrap();
    HttpResponse::Created()
        .content_type(ct)
        .body(bytes)
}

#[get("/api/users/{id}")]
async fn get_user(
    req: HttpRequest,
    id: web::Path<u32>,
) -> HttpResponse {
    let output_format = DataFormat::from_accept(&req);

    // Business logic
    let user = User {
        id: id.into_inner(),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        age: 30,
    };

    let response = ApiResponse {
        success: true,
        data: Some(user),
        error: None,
    };

    let (ct, bytes) = serialize_response(&response, output_format).unwrap();
    HttpResponse::Ok()
        .content_type(ct)
        .body(bytes)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/api/users", web::post().to(create_user))
            .route("/api/users/{id}", web::get().to(get_user))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Client Code (Rust)

```rust
use reqwest::Client;

#[tokio::main]
async fn main() {
    let client = Client::new();

    // Send request as TQN, receive as TBF
    let tqn_request = r#"!def CreateUserRequest name email age
Alice alice@example.com 30"#;

    let response = client
        .post("http://localhost:8080/api/users")
        .header("Content-Type", "text/tauq")
        .header("Accept", "application/tbf")
        .body(tqn_request)
        .send()
        .await
        .unwrap();

    // Parse TBF response directly
    let bytes = response.bytes().await.unwrap();
    let user: ApiResponse<User> = tauq::tbf::from_bytes(&bytes).unwrap();

    println!("{:?}", user);
}
```

---

## Python: Direct TQN/TBF Parsing

```python
from flask import Flask, request, Response
from tauq import tbf, compile_tauq
import json
from dataclasses import dataclass

app = Flask(__name__)

@dataclass
class User:
    id: int
    name: str
    email: str
    age: int

@dataclass
class CreateUserRequest:
    name: str
    email: str
    age: int

@dataclass
class ApiResponse:
    success: bool
    data: dict = None
    error: str = None

def detect_input_format(request):
    """Detect if request is TQN or TBF"""
    content_type = request.content_type or ""
    if "tbf" in content_type or "octet-stream" in content_type:
        return "tbf"
    return "tqn"

def detect_output_format(request):
    """Detect preferred response format from Accept header"""
    accept = request.headers.get("Accept", "")
    if "tbf" in accept or "octet-stream" in accept:
        return "tbf"
    return "tqn"

def parse_request(data: bytes, format: str):
    """Parse TQN or TBF directly to dict (native type)"""
    if format == "tbf":
        # Parse TBF directly
        return tbf.from_bytes(data)
    else:
        # Parse TQN directly
        tqn_text = data.decode("utf-8")
        json_obj = compile_tauq(tqn_text)
        return json_obj

def serialize_response(data: dict, format: str):
    """Serialize to TQN or TBF"""
    if format == "tbf":
        return tbf.to_bytes(data)
    else:
        # Serialize to TQN
        from tauq import format_to_tauq
        return format_to_tauq(data).encode("utf-8")

@app.route("/api/users", methods=["POST"])
def create_user():
    input_format = detect_input_format(request)
    output_format = detect_output_format(request)

    try:
        # Parse TQN or TBF directly
        user_data = parse_request(request.data, input_format)

        # Create user (business logic)
        user = {
            "id": 1,
            "name": user_data["name"],
            "email": user_data["email"],
            "age": user_data["age"],
        }

        # Response in requested format
        response_data = {
            "success": True,
            "data": user,
            "error": None,
        }

        response_bytes = serialize_response(response_data, output_format)
        content_type = "application/tbf" if output_format == "tbf" else "text/tauq"

        return Response(response_bytes, status=201, content_type=content_type)

    except Exception as e:
        response_data = {
            "success": False,
            "data": None,
            "error": str(e),
        }
        response_bytes = serialize_response(response_data, output_format)
        return Response(response_bytes, status=400)

@app.route("/api/users/<int:user_id>", methods=["GET"])
def get_user(user_id):
    output_format = detect_output_format(request)

    user = {
        "id": user_id,
        "name": "Alice",
        "email": "alice@example.com",
        "age": 30,
    }

    response_data = {
        "success": True,
        "data": user,
        "error": None,
    }

    response_bytes = serialize_response(response_data, output_format)
    content_type = "application/tbf" if output_format == "tbf" else "text/tauq"

    return Response(response_bytes, status=200, content_type=content_type)

if __name__ == "__main__":
    app.run(debug=True)
```

### Python Client

```python
import requests
from tauq import tbf, format_to_tauq

# Send TQN request, get TBF response
tqn_request = """!def CreateUserRequest name email age
Alice alice@example.com 30"""

response = requests.post(
    "http://localhost:5000/api/users",
    data=tqn_request.encode("utf-8"),
    headers={
        "Content-Type": "text/tauq",
        "Accept": "application/tbf",
    }
)

# Parse TBF response directly
user_data = tbf.from_bytes(response.content)
print(user_data)
```

---

## JavaScript/TypeScript: Native Format Support

```typescript
import express from "express";
import * as tauq from "tauq";

const app = express();

interface User {
  id: number;
  name: string;
  email: string;
  age: number;
}

interface CreateUserRequest {
  name: string;
  email: string;
  age: number;
}

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

function detectInputFormat(contentType: string): "tqn" | "tbf" {
  if (contentType?.includes("tbf") || contentType?.includes("octet-stream")) {
    return "tbf";
  }
  return "tqn";
}

function detectOutputFormat(accept: string): "tqn" | "tbf" {
  if (accept?.includes("tbf") || accept?.includes("octet-stream")) {
    return "tbf";
  }
  return "tqn";
}

async function parseRequest<T>(
  body: Buffer | string,
  format: "tqn" | "tbf"
): Promise<T> {
  if (format === "tbf") {
    // Parse TBF directly
    return tauq.tbf.decode(new Uint8Array(body as Buffer)) as T;
  } else {
    // Parse TQN directly
    const tqnText = typeof body === "string" ? body : body.toString("utf-8");
    const obj = tauq.compileTauq(tqnText);
    return obj as T;
  }
}

function serializeResponse<T>(
  data: T,
  format: "tqn" | "tbf"
): { bytes: Buffer | string; contentType: string } {
  if (format === "tbf") {
    const bytes = tauq.tbf.encode(data as any);
    return {
      bytes: Buffer.from(bytes),
      contentType: "application/tbf",
    };
  } else {
    const tqnStr = tauq.formatToTauq(data as any);
    return {
      bytes: tqnStr,
      contentType: "text/tauq; charset=utf-8",
    };
  }
}

app.post("/api/users", express.raw({ type: "*/*" }), async (req, res) => {
  const inputFormat = detectInputFormat(req.get("Content-Type") || "");
  const outputFormat = detectOutputFormat(req.get("Accept") || "");

  try {
    // Parse TQN or TBF directly to CreateUserRequest
    const userReq = await parseRequest<CreateUserRequest>(
      req.body,
      inputFormat
    );

    // Business logic
    const user: User = {
      id: 1,
      name: userReq.name,
      email: userReq.email,
      age: userReq.age,
    };

    // Serialize response
    const response: ApiResponse<User> = {
      success: true,
      data: user,
    };

    const { bytes, contentType } = serializeResponse(response, outputFormat);
    res.status(201).type(contentType).send(bytes);
  } catch (error) {
    const errorResponse: ApiResponse<null> = {
      success: false,
      error: (error as Error).message,
    };

    const { bytes, contentType } = serializeResponse(errorResponse, outputFormat);
    res.status(400).type(contentType).send(bytes);
  }
});

app.get("/api/users/:id", async (req, res) => {
  const outputFormat = detectOutputFormat(req.get("Accept") || "");

  const user: User = {
    id: parseInt(req.params.id),
    name: "Alice",
    email: "alice@example.com",
    age: 30,
  };

  const response: ApiResponse<User> = {
    success: true,
    data: user,
  };

  const { bytes, contentType } = serializeResponse(response, outputFormat);
  res.type(contentType).send(bytes);
});

app.listen(8080, () => {
  console.log("Server running on port 8080");
});
```

### TypeScript Client

```typescript
import fetch from "node-fetch";
import * as tauq from "tauq";

async function createUser() {
  // Create request in TQN
  const tqnRequest = `!def CreateUserRequest name email age
Alice alice@example.com 30`;

  // Send TQN, request TBF response
  const response = await fetch("http://localhost:8080/api/users", {
    method: "POST",
    headers: {
      "Content-Type": "text/tauq",
      Accept: "application/tbf",
    },
    body: tqnRequest,
  });

  // Parse TBF response directly
  const tbfBuffer = await response.arrayBuffer();
  const user = tauq.tbf.decode(new Uint8Array(tbfBuffer));

  console.log(user);
}

createUser();
```

---

## Go: Type-Safe Direct Parsing

```go
package main

import (
	"net/http"
	"io"
	tauq "github.com/epistates/tauq-go"
)

type User struct {
	ID    int    `json:"id"`
	Name  string `json:"name"`
	Email string `json:"email"`
	Age   int    `json:"age"`
}

type CreateUserRequest struct {
	Name  string `json:"name"`
	Email string `json:"email"`
	Age   int    `json:"age"`
}

type ApiResponse struct {
	Success bool        `json:"success"`
	Data    interface{} `json:"data"`
	Error   string      `json:"error"`
}

func detectInputFormat(r *http.Request) string {
	ct := r.Header.Get("Content-Type")
	if strings.Contains(ct, "tbf") || strings.Contains(ct, "octet-stream") {
		return "tbf"
	}
	return "tqn"
}

func detectOutputFormat(r *http.Request) string {
	accept := r.Header.Get("Accept")
	if strings.Contains(accept, "tbf") || strings.Contains(accept, "octet-stream") {
		return "tbf"
	}
	return "tqn"
}

func parseRequest(body io.Reader, format string, v interface{}) error {
	data, _ := io.ReadAll(body)

	if format == "tbf" {
		// Parse TBF directly to Go struct
		return tauq.UnmarshalTBF(data, v)
	} else {
		// Parse TQN directly to Go struct
		json_data, err := tauq.CompileTauq(string(data))
		if err != nil {
			return err
		}
		return json.Unmarshal(json_data, v)
	}
}

func serializeResponse(v interface{}, format string) ([]byte, string) {
	if format == "tbf" {
		bytes, _ := tauq.MarshalTBF(v)
		return bytes, "application/tbf"
	} else {
		// Serialize to TQN
		json_bytes, _ := json.Marshal(v)
		tqn_str := tauq.FormatToTauq(string(json_bytes))
		return []byte(tqn_str), "text/tauq; charset=utf-8"
	}
}

func createUserHandler(w http.ResponseWriter, r *http.Request) {
	inputFormat := detectInputFormat(r)
	outputFormat := detectOutputFormat(r)

	// Parse TQN or TBF directly to CreateUserRequest
	var userReq CreateUserRequest
	if err := parseRequest(r.Body, inputFormat, &userReq); err != nil {
		response := ApiResponse{
			Success: false,
			Error:   err.Error(),
		}
		bytes, ct := serializeResponse(response, outputFormat)
		w.Header().Set("Content-Type", ct)
		w.WriteHeader(http.StatusBadRequest)
		w.Write(bytes)
		return
	}

	// Business logic
	user := User{
		ID:    1,
		Name:  userReq.Name,
		Email: userReq.Email,
		Age:   userReq.Age,
	}

	// Serialize response
	response := ApiResponse{
		Success: true,
		Data:    user,
	}

	bytes, ct := serializeResponse(response, outputFormat)
	w.Header().Set("Content-Type", ct)
	w.WriteHeader(http.StatusCreated)
	w.Write(bytes)
}

func main() {
	http.HandleFunc("/api/users", createUserHandler)
	http.ListenAndServe(":8080", nil)
}
```

---

## Format Negotiation Matrix

| Client Sends | Accept Header | Server Returns |
|--------------|---------------|-----------------|
| TQN | `text/tauq` | TQN (readable) |
| TQN | `application/tbf` | TBF (compact) |
| TBF | `text/tauq` | TQN (readable) |
| TBF | `application/tbf` | TBF (compact) |
| TQN | (default) | TQN (readable) |
| TBF | (default) | TBF (compact) |

---

## Error Handling: Type-Safe

```rust
#[derive(Debug)]
enum TauqApiError {
    ParseError(String),
    ValidationError(String),
    SerializationError(String),
    NotFound,
}

impl From<tauq::error::TauqError> for TauqApiError {
    fn from(err: tauq::error::TauqError) -> Self {
        TauqApiError::ParseError(err.to_string())
    }
}

// Return errors in requested format
fn error_response(err: TauqApiError, format: DataFormat) -> (u16, String, Vec<u8>) {
    let message = match err {
        TauqApiError::ParseError(m) => m,
        TauqApiError::ValidationError(m) => m,
        TauqApiError::SerializationError(m) => m,
        TauqApiError::NotFound => "Not found".to_string(),
    };

    let response = ApiResponse::<()> {
        success: false,
        data: None,
        error: Some(message),
    };

    let (ct, bytes) = serialize_response(&response, format).unwrap();
    let status = match err {
        TauqApiError::NotFound => 404,
        TauqApiError::ValidationError(_) => 400,
        _ => 500,
    };

    (status, ct, bytes)
}
```

---

## Why This Matters

### ❌ Old Way (JSON middleware)
```
TQN → JSON → Parse → Business Logic → Serialize → JSON → TBF
  ↑                                                         ↑
User never sees readable format                 Conversion overhead
```

### ✅ New Way (Direct Parsing)
```
TQN → Parse → Business Logic → Serialize → TQN/TBF (user chooses)
  ↑                                              ↑
Readable                                  Efficient, no conversion
```

### Benefits

- **Zero JSON dependency**: Parse TQN directly to native types
- **Format agnostic**: Accept TQN or TBF, return either
- **Type-safe**: All parsing happens with serde, full type checking
- **Fast**: No intermediate representations
- **Streaming-friendly**: Parse large datasets incrementally
- **Error recovery**: Parse failures include context

---

## Next Steps

- **See**: [TBF vs Protobuf](comparison/protobuf.md) - Why TBF wins
- **Learn**: [Transport Helpers](transport-helpers.md) - Middleware and utilities
- **Implement**: [Language Bindings](bindings/README.md) - All supported languages
