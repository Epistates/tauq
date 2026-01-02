# Complete Usage Guide: Tauq Workflows

This guide shows how to use **TQQ**, **TQN**, and **TBF** together as a cohesive data processing system.

## The Three Components

| Component | Purpose | Best For |
|-----------|---------|----------|
| **TQN** | Text notation with 54% token savings | LLM inputs, config files, human editing |
| **TBF** | Binary format with 83% size savings | Transport, storage, databases, APIs |
| **TQQ** | Query language for transformations | Data pipelines, filtering, aggregation |

## The Core Pattern: TQN ↔ TBF

**The most important workflow: TBF as transparent transport**

```
Write TQN        Convert to TBF     Send/Store       Convert back      Read TQN
(readable) ----→ (compact)    ----→ (83% smaller) →   to TQN    ---→ (readable)

User sees TQN at both ends. TBF handles the transport invisibly.
```

**Why this matters:**
- ✅ Users always work with readable TQN
- ✅ Transport is transparent and 83% smaller
- ✅ No format mismatch between endpoints
- ✅ Both sides see the same data structure

---

## Workflow Patterns

### Pattern 1: TQN ↔ TBF Transport (Most Common)

**Scenario**: Write configuration/data in human-readable TQN, transport as compact TBF, read as TQN on other end

**Sender side:**
```bash
# 1. Write data in TQN (readable, 54% fewer tokens than JSON)
$ cat config.tqn
!def Server host port enabled
api.example.com 443 true
cache.internal 6379 true

# 2. Convert to TBF for transport (83% smaller than JSON)
$ tauq build config.tqn --format tbf -o config.tbf

# 3. Transport/store compact TBF
$ scp config.tbf remote-server:/etc/
# 14 KB instead of 100+ KB - your bandwidth thank you
```

**Receiver side:**
```bash
# 1. Receive compact TBF
$ scp remote-server:/etc/config.tbf .

# 2. Convert back to readable TQN
$ tauq format config.tbf --output tqn -o config.tqn

# 3. Read/edit human-friendly TQN
$ cat config.tqn
!def Server host port enabled
api.example.com 443 true
cache.internal 6379 true
```

**Rust integration:**
```rust
use tauq::tbf;

// Sender: TQN → TBF
let tqn_text = std::fs::read_to_string("config.tqn")?;
let json = tauq::compile_tauq(&tqn_text)?;
let users: Vec<User> = serde_json::from_value(json)?;
let tbf_compact = tbf::to_bytes(&users)?;
std::fs::write("config.tbf", &tbf_compact)?;

// Receiver: TBF → TQN
let tbf_bytes = std::fs::read("config.tbf")?;
let users: Vec<User> = tbf::from_bytes(&tbf_bytes)?;
let json = serde_json::to_value(&users)?;
let tqn_readable = tauq::format_to_tauq(&json);
println!("{}", tqn_readable);
```

**Why this workflow:**
- ✅ Both sides work with readable TQN
- ✅ Transport is transparent and compact
- ✅ 83% size reduction without user involvement
- ✅ Easy to version control (TQN in git)
- ✅ Zero parsing overhead (TBF is fast)

---

### Pattern 2: TQQ Transform → Multi-Format Output

**Scenario**: Process data with TQQ, output in both TQN (for display) and TBF (for storage)

```bash
# 1. Input data in TQN
$ cat users.tqn
!def User id name age department
1 Alice 30 Engineering
2 Bob 28 Sales
3 Carol 35 Engineering

# 2. Transform with TQQ
$ cat transform.tqq
# Filter to engineering only
pipe(
  select(department == "Engineering"),
  sort_by(age),
  project(id, name, age)
)

# 3. Output as TQN for humans to review
$ tauq exec transform.tqq < users.tqn --format tqn
!def User id name age
1 Alice 30
3 Carol 35

# 4. Output as TBF for storage/database
$ tauq exec transform.tqq < users.tqn --format tbf > engineering.tbf

# 5. Store TBF in database or send over wire (61% smaller than JSON)
```

**Complete example in Rust:**
```rust
use serde::Deserialize;
use tauq::tbf;

#[derive(Deserialize)]
struct User {
    id: u32,
    name: String,
    age: u32,
    department: String,
}

fn main() {
    // Parse TQN text
    let tqn_text = r#"!def User id name age department
1 Alice 30 Engineering
2 Bob 28 Sales
3 Carol 35 Engineering"#;

    let json: serde_json::Value = tauq::compile_tauq(tqn_text).unwrap();
    let users: Vec<User> = serde_json::from_value(json).unwrap();

    // Filter and process
    let engineering: Vec<User> = users
        .into_iter()
        .filter(|u| u.department == "Engineering")
        .collect();

    // Output as TBF (compact)
    let tbf_bytes = tbf::to_bytes(&engineering).unwrap();
    println!("TBF size: {} bytes", tbf_bytes.len());

    // Or output as TQN (readable)
    let json = serde_json::to_value(&engineering).unwrap();
    let tqn = tauq::format_to_tauq(&json);
    println!("{}", tqn);
}
```

---

### Pattern 3: API Gateway: TBF as Wire Format

**Scenario**: Receive user data as TQN, validate, store as TBF, transmit as TBF

```rust
use tauq::tbf;
use actix_web::{web, HttpResponse};

#[post("/api/users")]
async fn create_users(body: web::Bytes) -> HttpResponse {
    // 1. Request comes in as TQN (54% fewer tokens from LLM)
    let tqn_string = String::from_utf8(body.to_vec()).unwrap();

    // 2. Parse TQN to JSON structure
    let json = match tauq::compile_tauq(&tqn_string) {
        Ok(j) => j,
        Err(e) => return HttpResponse::BadRequest().body(format!("{}", e)),
    };

    // 3. Validate and process
    let users: Vec<User> = match serde_json::from_value(json) {
        Ok(u) => u,
        Err(e) => return HttpResponse::BadRequest().body(format!("{}", e)),
    };

    // 4. Store in database as TBF (83% smaller than JSON)
    for user in &users {
        let tbf_bytes = tbf::to_bytes(user).unwrap();
        database.insert(user.id, tbf_bytes);
    }

    // 5. Return as TBF for efficient transport (~35 bytes vs 150 bytes JSON)
    let response_tbf = tbf::to_bytes(&users).unwrap();
    HttpResponse::Ok()
        .content_type("application/tbf")
        .body(response_tbf)
}

#[get("/api/users/{id}")]
async fn get_user(id: web::Path<u32>) -> HttpResponse {
    // 1. Retrieve TBF from database
    let tbf_bytes = database.get(*id).unwrap();

    // 2. Parse TBF directly (zero-copy where possible)
    let user: User = tbf::from_bytes(&tbf_bytes).unwrap();

    // 3. Return as TBF (efficient)
    // Or convert to TQN if client requests: ?format=tqn
    let response_tbf = tbf::to_bytes(&user).unwrap();
    HttpResponse::Ok()
        .content_type("application/tbf")
        .body(response_tbf)
}
```

**Client code:**
```typescript
// JavaScript/TypeScript client
import * as tauq from 'tauq';

// 1. Create data in TQN format
const tqnData = `!def User id name email
1 Alice alice@example.com
2 Bob bob@example.com`;

// 2. Send to API (54% fewer tokens than JSON)
const response = await fetch('/api/users', {
  method: 'POST',
  body: tqnData,
  headers: { 'Content-Type': 'text/tauq' }
});

// 3. Receive TBF response
const tbfBuffer = await response.arrayBuffer();

// 4. Parse TBF to JavaScript objects
const users = tauq.tbf.decode(new Uint8Array(tbfBuffer));

// 5. Display to user
users.forEach(user => console.log(user.name));
```

---

### Pattern 4: Data Lake Integration

**Scenario**: Multi-format pipeline → TBF → Apache Iceberg

```rust
use tauq::tbf_iceberg::TbfFileWriter;
use arrow::record_batch::RecordBatch;

fn ingest_data_pipeline() {
    // 1. Read from multiple sources
    let csv_data = read_csv("sales.csv");
    let json_data = read_json("customers.json");
    let tqn_data = read_tqn("events.tqn");

    // 2. Normalize all to common schema
    let csv_records: Vec<SalesRecord> = csv_data.into_iter()
        .map(|row| SalesRecord::from_csv_row(row))
        .collect();

    let json_records: Vec<SalesRecord> = json_data.into_iter()
        .map(|obj| serde_json::from_value::<SalesRecord>(obj).unwrap())
        .collect();

    let tqn_records: Vec<SalesRecord> = {
        let json = tauq::compile_tauq(&tqn_data).unwrap();
        serde_json::from_value(json).unwrap()
    };

    // 3. Combine all records
    let mut all_records = vec![];
    all_records.extend(csv_records);
    all_records.extend(json_records);
    all_records.extend(tqn_records);

    // 4. Write to Iceberg as TBF (columnar, compressed)
    let schema = create_iceberg_schema();
    let mut writer = TbfFileWriter::new(schema)?;

    for batch in all_records.chunks(1000) {
        let record_batch = arrow::compute::concat_batches(
            &schema,
            &batch.iter()
                .map(|r| r.to_arrow_record())
                .collect::<Vec<_>>()
        )?;

        writer.write(&record_batch)?;
    }

    writer.finish()?;

    // 5. Results: TBF in Iceberg table
    // - 83% smaller than JSON
    // - Zero-copy deserialization in Rust
    // - Native Iceberg format for SQL queries
}
```

---

### Pattern 5: Configuration Management

**Scenario**: Define schema in TQN, validate structure, deploy as TBF

```bash
# 1. Define schema in TQN (human-readable)
$ cat schema.tqn
!def Server
  name        "production-api"
  host        "api.prod.example.com"
  port        443
  healthcheck "/health"
  replicas    5

!def Database
  name        "primary"
  host        "db-primary.internal"
  user        "app_user"
  password    "***"
  pool_size   20

# 2. Validate schema structure
$ tauq validate schema.tqn
✓ Schema valid

# 3. Build and convert to TBF for deployment
$ tauq build schema.tqn --format tbf -o schema.tbf

# 4. Deploy as single binary file
$ kubectl create configmap app-config --from-file=schema.tbf

# 5. Application loads at startup
$ cargo run
```

**Runtime loading:**
```rust
use tauq::tbf;

#[derive(serde::Deserialize)]
struct Config {
    server: Server,
    database: Database,
}

#[tokio::main]
async fn main() {
    // 1. Load TBF from configmap/file
    let config_bytes = std::fs::read("/etc/config/schema.tbf").unwrap();

    // 2. Parse efficiently (no JSON intermediate)
    let config: Config = tbf::from_bytes(&config_bytes).unwrap();

    // 3. Use immediately
    start_server(config.server).await;
}
```

---

## Format Selection Matrix

Choose the right format for each step:

| Use Case | Format | Why |
|----------|--------|-----|
| **Write data** | TQN | 54% fewer tokens, human-readable |
| **Transform data** | TQQ (input) → TQN/TBF (output) | Flexible pipeline |
| **Display data** | TQN | Easy to read and edit |
| **Store in database** | TBF | 83% smaller, indexed efficiently |
| **Send over network** | TBF | 83% smaller, faster parse |
| **LLM context** | TQN | 54% fewer tokens = lower cost |
| **Config files** | TQN | Easy to version control and review |
| **Data lake (Iceberg)** | TBF | Native columnar, analytical queries |
| **Runtime parsing** | TBF | Zero-copy, optimal performance |

---

## Complete Example: E-Commerce Order Processing

Here's how all three components work together:

### Input: Order from LLM (TQN format - 54% fewer tokens)
```tqn
!def Order id customer_id total_amount items status
ORD-001 CUST-123 249.99 [
  { sku PRD-456 qty 2 price 99.99 }
  { sku PRD-789 qty 1 price 49.99 }
] pending
```

### Transform: Filter and aggregate (TQQ)
```bash
$ cat process_orders.tqq
# Filter orders over $200, calculate discount
pipe(
  filter(total_amount >= 200),
  project(
    id,
    customer_id,
    total_amount,
    discount = total_amount * 0.1,
    final_amount = total_amount * 0.9
  ),
  sort_by(total_amount)
)

$ tauq exec process_orders.tqq < orders.tqn --format tqn
# Output: Processed orders in TQN
```

### Storage: Convert to TBF
```bash
$ tauq build processed_orders.tqn --format tbf -o orders.tbf
# Result: 16 KB instead of 92 KB (83% smaller)
```

### Transport: Send TBF
```rust
// API server
#[post("/orders")]
async fn process_orders(body: web::Bytes) -> HttpResponse {
    let orders: Vec<Order> = tauq::tbf::from_bytes(&body).unwrap();

    // Process...

    HttpResponse::Ok().body(tauq::tbf::to_bytes(&results).unwrap())
}
```

### Persistence: Store in Iceberg
```rust
use tauq::tbf_iceberg::TbfFileWriter;

let writer = TbfFileWriter::new(iceberg_schema)?;
writer.write_records(&orders)?;
```

---

## Common Patterns by Use Case

### **Microservices Communication**
```
Service A → TBF (compact) → Network → TBF → Service B
                                              ↓
                                          Parse to Rust types
                                              ↓
                                          Process
```

### **LLM Integration**
```
Data → TQN (54% fewer tokens) → LLM
                                   ↓
                          LLM returns TQN
                                   ↓
                        Validate & parse to JSON
                                   ↓
                            Store as TBF
```

### **Time-Series Analytics**
```
Sensors → TQN (log format)
    ↓
Aggregate with TQQ
    ↓
Output as TBF → Iceberg
    ↓
SQL queries on columnar data
```

### **Config Management**
```
Write TQN → Validate → Build TBF
                           ↓
                      Deploy
                           ↓
                      Load as TBF
                           ↓
                      Zero-copy parse
```

---

## Performance Characteristics

### Token Usage (Lower = Cheaper for LLMs)
```
JSON:      24,005 tokens (1000 records)
TQN:       11,012 tokens (54% savings) ✅
TOON:      12,002 tokens (50% savings)
```

### Binary Size (Lower = Faster transmission)
```
JSON:      92 KB (1000 records)
TBF (gen): 41 KB (55% reduction)     ✅
TBF (schema): 16 KB (83% reduction)  ✅✅
```

### Parse Performance (Stream vs. Full)
```
Full parse:    ~2.5ms (JSON)
Streaming:     ~0.3ms per record (TQN)
Binary parse:  <0.1ms (TBF, zero-copy)
```

---

## Next Steps

- **New to Tauq?** Start with [Introduction](introduction.md)
- **Writing TQN?** See [TQN Documentation](tauq/README.md)
- **Using TBF?** See [TBF Documentation](tbf/README.md)
- **Transform data?** See [TQQ Documentation](tauqq/README.md)
- **Building APIs?** See [Integration Guide](integration.md)
