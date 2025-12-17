# Transport Helpers: Middleware & Utilities

Robust helpers for TQN/TBF transport across frameworks and platforms.

---

## Rust: Actix-web Middleware

### Automatic Format Detection & Conversion

```rust
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::{ok, Ready};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Format wrapper in request extensions
#[derive(Debug, Clone, Copy)]
pub enum DataFormat {
    Tqn,
    Tbf,
}

/// Middleware for automatic format detection
pub struct FormatDetectionMiddleware;

impl<S, B> Transform<S, ServiceRequest> for FormatDetectionMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = FormatDetectionMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(FormatDetectionMiddlewareService { service })
    }
}

pub struct FormatDetectionMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for FormatDetectionMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Detect input format
        let input_format = match req.content_type() {
            ct if ct.contains("tbf") || ct.contains("octet-stream") => DataFormat::Tbf,
            _ => DataFormat::Tqn,
        };

        // Detect output format preference
        let output_format = match req.headers()
            .get("Accept")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("") {
            accept if accept.contains("tbf") || accept.contains("octet-stream") => DataFormat::Tbf,
            _ => DataFormat::Tqn,
        };

        req.extensions_mut().insert(input_format);
        req.extensions_mut().insert(output_format);

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
```

### Helper Extractors

```rust
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use serde::de::DeserializeOwned;

/// Extractor that parses TQN or TBF automatically
pub struct TauqBody<T>(pub T);

impl<T> FromRequest for TauqBody<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let input_format = *req.extensions()
            .get::<DataFormat>()
            .unwrap_or(&DataFormat::Tqn);

        let payload = payload.take();

        Box::pin(async move {
            let body = actix_web::web::Bytes::from_request(req, &mut Payload::from(payload))
                .await
                .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

            match input_format {
                DataFormat::Tqn => {
                    let tqn_text = String::from_utf8(body.to_vec())
                        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

                    let json = tauq::compile_tauq(&tqn_text)
                        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

                    serde_json::from_value(json)
                        .map(TauqBody)
                        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))
                }
                DataFormat::Tbf => {
                    tauq::tbf::from_bytes::<T>(&body)
                        .map(TauqBody)
                        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))
                }
            }
        })
    }
}

/// Responder that serializes to TQN or TBF
pub struct TauqResponse<T>(pub T);

impl<T> actix_web::Responder for TauqResponse<T>
where
    T: serde::Serialize,
{
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, req: &HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let output_format = *req.extensions()
            .get::<DataFormat>()
            .unwrap_or(&DataFormat::Tqn);

        match output_format {
            DataFormat::Tqn => {
                match serde_json::to_value(&self.0) {
                    Ok(json) => {
                        let tqn = tauq::format_to_tauq(&json);
                        actix_web::HttpResponse::Ok()
                            .content_type("text/tauq; charset=utf-8")
                            .body(tqn)
                    }
                    Err(e) => actix_web::HttpResponse::InternalServerError()
                        .body(format!("Serialization error: {}", e)),
                }
            }
            DataFormat::Tbf => {
                match tauq::tbf::to_bytes(&self.0) {
                    Ok(bytes) => actix_web::HttpResponse::Ok()
                        .content_type("application/tbf")
                        .body(bytes),
                    Err(e) => actix_web::HttpResponse::InternalServerError()
                        .body(format!("TBF encoding error: {}", e)),
                }
            }
        }
    }
}
```

### Handler Using Helpers

```rust
use actix_web::{web, post};

#[post("/api/users")]
async fn create_user(req: HttpRequest, body: TauqBody<CreateUserRequest>) -> impl actix_web::Responder {
    let user = User {
        id: 1,
        name: body.0.name,
        email: body.0.email,
        age: body.0.age,
    };

    TauqResponse(user)
}
```

---

## Python: Flask Decorators

```python
from functools import wraps
from flask import request, g
from tauq import tbf, compile_tauq, format_to_tauq
import json

def detect_format(content_type: str) -> str:
    """Detect TQN or TBF from Content-Type"""
    if "tbf" in content_type or "octet-stream" in content_type:
        return "tbf"
    return "tqn"

def accept_tauq(*accepted_formats):
    """
    Decorator: Automatically detect input/output formats

    Usage:
        @app.route("/api/users", methods=["POST"])
        @accept_tauq("tqn", "tbf")
        def create_user(user_req):
            # user_req is already parsed to dict/object
            ...
    """
    def decorator(f):
        @wraps(f)
        def decorated_function(*args, **kwargs):
            # Detect input format
            input_format = detect_format(request.content_type or "")
            output_format = detect_format(request.headers.get("Accept", ""))

            # Store in g for access in handler
            g.input_format = input_format
            g.output_format = output_format

            # Parse request
            try:
                if input_format == "tbf":
                    parsed_data = tbf.from_bytes(request.data)
                else:
                    tqn_text = request.data.decode("utf-8")
                    json_obj = compile_tauq(tqn_text)
                    parsed_data = json_obj

                g.parsed_data = parsed_data
            except Exception as e:
                from flask import jsonify
                return {"success": False, "error": str(e)}, 400

            # Call handler
            result = f(parsed_data, *args, **kwargs)
            return result

        return decorated_function
    return decorator

def tauq_response(f):
    """
    Decorator: Serialize response to TQN or TBF

    Usage:
        @tauq_response
        def create_user(user_req):
            return {"success": True, "data": {...}}
    """
    @wraps(f)
    def decorated_function(*args, **kwargs):
        result = f(*args, **kwargs)

        if isinstance(result, tuple):
            data, status_code = result
        else:
            data = result
            status_code = 200

        output_format = getattr(g, "output_format", "tqn")

        if output_format == "tbf":
            response_bytes = tbf.to_bytes(data)
            content_type = "application/tbf"
        else:
            tqn_str = format_to_tauq(data)
            response_bytes = tqn_str.encode("utf-8")
            content_type = "text/tauq; charset=utf-8"

        from flask import Response
        return Response(response_bytes, status=status_code, content_type=content_type)

    return decorated_function

# Usage example
@app.route("/api/users", methods=["POST"])
@accept_tauq("tqn", "tbf")
@tauq_response
def create_user(user_req):
    user = {
        "id": 1,
        "name": user_req["name"],
        "email": user_req["email"],
        "age": user_req["age"],
    }
    return {"success": True, "data": user}, 201
```

---

## JavaScript: Express Middleware

```typescript
import express from "express";
import * as tauq from "tauq";

interface TauqOptions {
  defaultFormat?: "tqn" | "tbf";
  streaming?: boolean;
}

export function tauqMiddleware(options: TauqOptions = {}) {
  const defaultFormat = options.defaultFormat || "tqn";

  return express.raw({ type: "*/*" }, (req: any, res: any, next: any) => {
    // Detect input format
    const contentType = req.get("Content-Type") || "";
    req.tauqInputFormat = contentType.includes("tbf") ? "tbf" : "tqn";

    // Detect output format
    const accept = req.get("Accept") || "";
    req.tauqOutputFormat = accept.includes("tbf") ? "tbf" : defaultFormat;

    // Wrap JSON to parse TQN/TBF
    const originalBody = req.body;

    req.tauqParse = async <T>(Type?: any): Promise<T> => {
      try {
        if (req.tauqInputFormat === "tbf") {
          return tauq.tbf.decode(new Uint8Array(originalBody)) as T;
        } else {
          const tqnText = originalBody.toString("utf-8");
          const jsonObj = tauq.compileTauq(tqnText);
          return jsonObj as T;
        }
      } catch (error) {
        throw new Error(`Parse error: ${error}`);
      }
    };

    req.tauqSerialize = <T>(data: T): Buffer | string => {
      if (req.tauqOutputFormat === "tbf") {
        return Buffer.from(tauq.tbf.encode(data as any));
      } else {
        return tauq.formatToTauq(data as any);
      }
    };

    next();
  });
}

export function tauqResponse<T>(data: T, req: any, res: any) {
  const bytes = req.tauqSerialize(data);
  const contentType =
    req.tauqOutputFormat === "tbf"
      ? "application/tbf"
      : "text/tauq; charset=utf-8";

  res.type(contentType).send(bytes);
}

// Usage
app.use(tauqMiddleware({ defaultFormat: "tqn" }));

app.post("/api/users", async (req: any, res: any) => {
  try {
    const userReq = await req.tauqParse<CreateUserRequest>();

    const user: User = {
      id: 1,
      name: userReq.name,
      email: userReq.email,
      age: userReq.age,
    };

    tauqResponse({ success: true, data: user }, req, res);
  } catch (error) {
    tauqResponse({ success: false, error: (error as Error).message }, req, res);
  }
});
```

---

## Go: HTTP Middleware

```go
package middleware

import (
	"io"
	"net/http"
	"strings"
	tauq "github.com/epistates/tauq-go"
)

type TauqContext struct {
	InputFormat  string
	OutputFormat string
}

const TauqContextKey = "tauq"

func TauqMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Detect formats
		inputFormat := "tqn"
		if strings.Contains(r.Header.Get("Content-Type"), "tbf") {
			inputFormat = "tbf"
		}

		outputFormat := "tqn"
		if strings.Contains(r.Header.Get("Accept"), "tbf") {
			outputFormat = "tbf"
		}

		// Store in context
		ctx := TauqContext{
			InputFormat:  inputFormat,
			OutputFormat: outputFormat,
		}
		r.Header.Set(TauqContextKey, "true")

		// Custom response writer to capture data
		bodyBytes, _ := io.ReadAll(r.Body)
		r.Body = io.NopCloser(strings.NewReader(string(bodyBytes)))

		// Helper functions
		r.Header.Set("tauq-parse", "true")
		r.Header.Set("tauq-serialize", "true")

		next.ServeHTTP(w, r.WithContext(
			context.WithValue(r.Context(), TauqContextKey, ctx),
		))
	})
}

func ParseRequest(r *http.Request, v interface{}) error {
	ctx := r.Context().Value(TauqContextKey).(TauqContext)
	data, _ := io.ReadAll(r.Body)

	if ctx.InputFormat == "tbf" {
		return tauq.UnmarshalTBF(data, v)
	} else {
		tqnText := string(data)
		jsonData, err := tauq.CompileTauq(tqnText)
		if err != nil {
			return err
		}
		return json.Unmarshal(jsonData, v)
	}
}

func WriteResponse(w http.ResponseWriter, r *http.Request, v interface{}) error {
	ctx := r.Context().Value(TauqContextKey).(TauqContext)

	var bytes []byte
	var ct string

	if ctx.OutputFormat == "tbf" {
		b, err := tauq.MarshalTBF(v)
		if err != nil {
			return err
		}
		bytes = b
		ct = "application/tbf"
	} else {
		jsonBytes, _ := json.Marshal(v)
		tqnStr := tauq.FormatToTauq(string(jsonBytes))
		bytes = []byte(tqnStr)
		ct = "text/tauq; charset=utf-8"
	}

	w.Header().Set("Content-Type", ct)
	w.Write(bytes)
	return nil
}

// Usage
func CreateUserHandler(w http.ResponseWriter, r *http.Request) {
	var userReq CreateUserRequest
	if err := ParseRequest(r, &userReq); err != nil {
		WriteResponse(w, r, map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		})
		return
	}

	user := User{
		ID:    1,
		Name:  userReq.Name,
		Email: userReq.Email,
		Age:   userReq.Age,
	}

	WriteResponse(w, r, map[string]interface{}{
		"success": true,
		"data":    user,
	})
}

// Register with middleware
func main() {
	http.Handle("/api/users", middleware.TauqMiddleware(
		http.HandlerFunc(CreateUserHandler),
	))
	http.ListenAndServe(":8080", nil)
}
```

---

## Client Helpers

### Rust Client

```rust
use reqwest::Client;

pub struct TauqClient {
    client: reqwest::Client,
    format: DataFormat,
}

impl TauqClient {
    pub fn new(format: DataFormat) -> Self {
        Self {
            client: Client::new(),
            format,
        }
    }

    pub async fn post<T, R>(&self, url: &str, data: &T) -> Result<R, Box<dyn std::error::Error>>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let body = match self.format {
            DataFormat::Tqn => {
                let json = serde_json::to_value(data)?;
                tauq::format_to_tauq(&json).into_bytes()
            }
            DataFormat::Tbf => tbf::to_bytes(data)?,
        };

        let content_type = match self.format {
            DataFormat::Tqn => "text/tauq",
            DataFormat::Tbf => "application/tbf",
        };

        let response = self
            .client
            .post(url)
            .header("Content-Type", content_type)
            .header("Accept", "application/tbf") // Request TBF response
            .body(body)
            .send()
            .await?;

        let bytes = response.bytes().await?;
        let result = tbf::from_bytes(&bytes)?;
        Ok(result)
    }
}

// Usage
let client = TauqClient::new(DataFormat::Tqn);
let user = User { /* ... */ };
let response: ApiResponse<User> = client.post("/api/users", &user).await?;
```

### Python Client

```python
import requests
from tauq import tbf, compile_tauq, format_to_tauq

class TauqClient:
    def __init__(self, format: str = "tqn"):
        self.format = format
        self.session = requests.Session()

    def post(self, url: str, data: dict, response_format: str = "tbf"):
        """Send request in TQN/TBF, receive in specified format"""
        if self.format == "tbf":
            body = tbf.to_bytes(data)
            ct = "application/tbf"
        else:
            tqn_str = format_to_tauq(data)
            body = tqn_str.encode("utf-8")
            ct = "text/tauq"

        response = self.session.post(
            url,
            data=body,
            headers={
                "Content-Type": ct,
                "Accept": "application/tbf" if response_format == "tbf" else "text/tauq",
            },
        )

        if response_format == "tbf":
            return tbf.from_bytes(response.content)
        else:
            tqn_text = response.text
            return compile_tauq(tqn_text)

# Usage
client = TauqClient(format="tqn")
response = client.post("http://localhost:8080/api/users", {"name": "Alice", "email": "alice@example.com", "age": 30})
print(response)
```

---

## Streaming Support

### Large File Upload (TQN Lines)

```rust
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::fs::File;

async fn stream_tqn_lines<T: serde::de::DeserializeOwned>(
    file: File,
) -> Result<impl Stream<Item = Result<T, Box<dyn std::error::Error>>>, Box<dyn std::error::Error>> {
    let reader = BufReader::new(file);
    let lines = reader.lines();

    Ok(stream::unfold(
        (lines, String::new()),
        |(mut lines, mut buffer)| async move {
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if line.starts_with("!def") {
                            buffer = line;
                        } else if !line.is_empty() {
                            let json = match tauq::compile_tauq(&format!("{}\n{}", buffer, line)) {
                                Ok(j) => j,
                                Err(e) => return Some((Err(Box::new(e) as _), (lines, buffer))),
                            };

                            let value = match serde_json::from_value::<T>(json) {
                                Ok(v) => v,
                                Err(e) => return Some((Err(Box::new(e) as _), (lines, buffer))),
                            };

                            return Some((Ok(value), (lines, buffer)));
                        }
                    }
                    Ok(None) => return None,
                    Err(e) => return Some((Err(Box::new(e) as _), (lines, buffer))),
                }
            }
        },
    ))
}

// Usage
let file = File::open("large_dataset.tqn").await?;
let mut stream = stream_tqn_lines::<MyStruct>(file).await?;

while let Some(result) = stream.next().await {
    match result {
        Ok(record) => process_record(record).await?,
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
```

---

## Error Handling

### Structured Errors

```rust
#[derive(Debug, serde::Serialize)]
struct TauqError {
    code: String,
    message: String,
    context: Option<String>,
}

impl TauqError {
    fn parse_error(msg: String) -> Self {
        Self {
            code: "PARSE_ERROR".to_string(),
            message: msg,
            context: None,
        }
    }

    fn validation_error(msg: String) -> Self {
        Self {
            code: "VALIDATION_ERROR".to_string(),
            message: msg,
            context: None,
        }
    }
}

// Return error in requested format
fn error_response<T: serde::Serialize>(
    error: TauqError,
    format: DataFormat,
) -> (u16, String, Vec<u8>) {
    let status = match error.code.as_str() {
        "PARSE_ERROR" => 400,
        "VALIDATION_ERROR" => 422,
        _ => 500,
    };

    let response = serde_json::json!({
        "success": false,
        "error": error,
    });

    let (ct, bytes) = match format {
        DataFormat::Tqn => {
            let tqn = tauq::format_to_tauq(&response);
            ("text/tauq".to_string(), tqn.into_bytes())
        }
        DataFormat::Tbf => {
            let bytes = tbf::to_bytes(&response).unwrap();
            ("application/tbf".to_string(), bytes)
        }
    };

    (status, ct, bytes)
}
```

---

## Next Steps

- **See**: [API Gateway Examples](api-gateway.md)
- **Compare**: [TBF vs Protobuf](comparison-protobuf.md)
- **Implement**: [Language Bindings](bindings/README.md)
