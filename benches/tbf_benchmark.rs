//! TBF (Tauq Binary Format) Benchmark
//!
//! Compares TBF against other binary formats: bitcode, bincode, postcard, msgpack
//!
//! Run with: cargo bench --bench tbf_benchmark

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use tauq::TauqError;
use tauq::tbf::{
    AdaptiveIntEncoder,
    AdaptiveStringEncoder,
    // Type-based schema API
    FieldEncoding,
    SCHEMA_MAGIC,
    TableEncode,
    TableSchema,
    encode_varint_fast,
};
use tauq::tbf::{BorrowedDictionary, StringDictionary, TbfDecode, TbfEncode};
use tauq::tbf::{
    ColumnCollectors, DirectStringEncoder, DirectU32Encoder, ULTRA_MAGIC, ULTRA_VERSION,
    UltraBuffer, UltraColumnType, UltraEncode, UltraEncodeDirect, encode_varint_to_ultra,
};
use tauq::tbf::{ColumnReader, ColumnType, ColumnarDecode, ColumnarEncode, ColumnarEncoder};
use tauq::tbf::{FastBorrowedDictionary, FastDecode, fast_decode_varint};
use tauq::tbf::{FastBuffer, FastEncode, FastStringDictionary, fast_encode_slice};
use tauq::tbf::{decode_varint, encode_varint};

// ============================================================================
// Test Data
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct Employee {
    pub id: u32,
    pub name: String,
    pub age: u32,
    pub city: String,
    pub department: String,
    pub salary: u32,
    pub experience: u32,
    pub project_count: u32,
}

// Manual TbfEncode/TbfDecode implementation for Employee (simulating derive macro output)
impl TbfEncode for Employee {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        encode_varint(self.id as u64, buf);
        let idx = dict.intern(&self.name);
        encode_varint(idx as u64, buf);
        encode_varint(self.age as u64, buf);
        let idx = dict.intern(&self.city);
        encode_varint(idx as u64, buf);
        let idx = dict.intern(&self.department);
        encode_varint(idx as u64, buf);
        encode_varint(self.salary as u64, buf);
        encode_varint(self.experience as u64, buf);
        encode_varint(self.project_count as u64, buf);
    }
}

impl TbfDecode for Employee {
    fn tbf_decode_from(
        buf: &[u8],
        pos: &mut usize,
        dict: &BorrowedDictionary,
    ) -> Result<Self, TauqError> {
        let (id, len) = decode_varint(&buf[*pos..])?;
        *pos += len;

        let (name_idx, len) = decode_varint(&buf[*pos..])?;
        *pos += len;
        let name = dict
            .get(name_idx as u32)
            .ok_or_else(|| tauq::error::InterpretError::new("Invalid string index"))?
            .to_string();

        let (age, len) = decode_varint(&buf[*pos..])?;
        *pos += len;

        let (city_idx, len) = decode_varint(&buf[*pos..])?;
        *pos += len;
        let city = dict
            .get(city_idx as u32)
            .ok_or_else(|| tauq::error::InterpretError::new("Invalid string index"))?
            .to_string();

        let (dept_idx, len) = decode_varint(&buf[*pos..])?;
        *pos += len;
        let department = dict
            .get(dept_idx as u32)
            .ok_or_else(|| tauq::error::InterpretError::new("Invalid string index"))?
            .to_string();

        let (salary, len) = decode_varint(&buf[*pos..])?;
        *pos += len;

        let (experience, len) = decode_varint(&buf[*pos..])?;
        *pos += len;

        let (project_count, len) = decode_varint(&buf[*pos..])?;
        *pos += len;

        Ok(Employee {
            id: id as u32,
            name,
            age: age as u32,
            city,
            department,
            salary: salary as u32,
            experience: experience as u32,
            project_count: project_count as u32,
        })
    }
}

// Columnar encoding implementation for Employee
impl ColumnarEncode for Employee {
    fn define_columns(encoder: &mut ColumnarEncoder) {
        encoder.add_column("id", ColumnType::U32);
        encoder.add_column("name", ColumnType::String);
        encoder.add_column("age", ColumnType::U32);
        encoder.add_column("city", ColumnType::String);
        encoder.add_column("department", ColumnType::String);
        encoder.add_column("salary", ColumnType::U32);
        encoder.add_column("experience", ColumnType::U32);
        encoder.add_column("project_count", ColumnType::U32);
    }

    fn encode_to_columns(&self, encoder: &mut ColumnarEncoder) {
        encoder.push_u32(0, self.id);
        encoder.push_string(1, &self.name);
        encoder.push_u32(2, self.age);
        encoder.push_string(3, &self.city);
        encoder.push_string(4, &self.department);
        encoder.push_u32(5, self.salary);
        encoder.push_u32(6, self.experience);
        encoder.push_u32(7, self.project_count);
    }
}

impl ColumnarDecode for Employee {
    fn decode_from_columns(readers: &mut [ColumnReader<'_>]) -> Option<Self> {
        Some(Employee {
            id: readers[0].next_u32()?,
            name: readers[1].next_string()?.to_string(),
            age: readers[2].next_u32()?,
            city: readers[3].next_string()?.to_string(),
            department: readers[4].next_string()?.to_string(),
            salary: readers[5].next_u32()?,
            experience: readers[6].next_u32()?,
            project_count: readers[7].next_u32()?,
        })
    }
}

// Fast encoding implementation for Employee (optimized)
impl FastEncode for Employee {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, dict: &mut FastStringDictionary) {
        buf.write_u32(self.id);
        buf.write_string(&self.name, dict);
        buf.write_u32(self.age);
        buf.write_string(&self.city, dict);
        buf.write_string(&self.department, dict);
        buf.write_u32(self.salary);
        buf.write_u32(self.experience);
        buf.write_u32(self.project_count);
    }

    fn estimated_size(&self) -> usize {
        // 5 u32 varints (max 5 bytes each) + 3 string indices (max 2 bytes each)
        5 * 5 + 3 * 2
    }
}

// Fast decoding implementation for Employee (optimized)
impl FastDecode for Employee {
    fn fast_decode_from(
        bytes: &[u8],
        dict: &FastBorrowedDictionary,
    ) -> Result<(Self, usize), TauqError> {
        let mut pos = 0;

        let (id, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;

        let (name_idx, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;
        let name = dict
            .get(name_idx as u32)
            .ok_or_else(|| tauq::error::InterpretError::new("Invalid string index"))?
            .to_string();

        let (age, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;

        let (city_idx, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;
        let city = dict
            .get(city_idx as u32)
            .ok_or_else(|| tauq::error::InterpretError::new("Invalid string index"))?
            .to_string();

        let (dept_idx, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;
        let department = dict
            .get(dept_idx as u32)
            .ok_or_else(|| tauq::error::InterpretError::new("Invalid string index"))?
            .to_string();

        let (salary, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;

        let (experience, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;

        let (project_count, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;

        Ok((
            Employee {
                id: id as u32,
                name,
                age: age as u32,
                city,
                department,
                salary: salary as u32,
                experience: experience as u32,
                project_count: project_count as u32,
            },
            pos,
        ))
    }
}

// Ultra encoding implementation for Employee (bitcode-style columnar + adaptive packing)
impl UltraEncode for Employee {
    fn column_count() -> usize {
        8 // id, name, age, city, department, salary, experience, project_count
    }

    fn collect_columns(items: &[Self], collectors: &mut ColumnCollectors) {
        // Initialize columns with correct types
        let capacity = items.len();
        collectors.init_column(0, UltraColumnType::U32, capacity); // id
        collectors.init_column(1, UltraColumnType::String, capacity); // name
        collectors.init_column(2, UltraColumnType::U32, capacity); // age
        collectors.init_column(3, UltraColumnType::String, capacity); // city
        collectors.init_column(4, UltraColumnType::String, capacity); // department
        collectors.init_column(5, UltraColumnType::U32, capacity); // salary
        collectors.init_column(6, UltraColumnType::U32, capacity); // experience
        collectors.init_column(7, UltraColumnType::U32, capacity); // project_count

        // Collect all values column by column
        for emp in items {
            collectors.push_u32(0, emp.id);
            collectors.push_string(1, &emp.name);
            collectors.push_u32(2, emp.age);
            collectors.push_string(3, &emp.city);
            collectors.push_string(4, &emp.department);
            collectors.push_u32(5, emp.salary);
            collectors.push_u32(6, emp.experience);
            collectors.push_u32(7, emp.project_count);
        }
    }
}

// Ultra Direct encoding - uses direct encoders without intermediate typed vectors
impl UltraEncodeDirect for Employee {
    fn ultra_encode_direct(items: &[Self]) -> Vec<u8> {
        if items.is_empty() {
            let mut buf = UltraBuffer::with_capacity(16);
            buf.extend(&ULTRA_MAGIC);
            buf.push(ULTRA_VERSION);
            buf.push(0);
            encode_varint_to_ultra(0, &mut buf);
            return buf.into_vec();
        }

        let n = items.len();

        // Create direct encoders - no intermediate Vec<String> allocations
        let mut id_enc = DirectU32Encoder::with_capacity(n);
        let mut name_enc = DirectStringEncoder::with_capacity(n);
        let mut age_enc = DirectU32Encoder::with_capacity(n);
        let mut city_enc = DirectStringEncoder::with_capacity(n);
        let mut dept_enc = DirectStringEncoder::with_capacity(n);
        let mut salary_enc = DirectU32Encoder::with_capacity(n);
        let mut exp_enc = DirectU32Encoder::with_capacity(n);
        let mut proj_enc = DirectU32Encoder::with_capacity(n);

        // Single pass through data
        for emp in items {
            id_enc.push(emp.id);
            name_enc.push(&emp.name);
            age_enc.push(emp.age);
            city_enc.push(&emp.city);
            dept_enc.push(&emp.department);
            salary_enc.push(emp.salary);
            exp_enc.push(emp.experience);
            proj_enc.push(emp.project_count);
        }

        // Estimate output size
        let estimated = n * 20 + 64;
        let mut buf = UltraBuffer::with_capacity(estimated);

        // Header
        buf.extend(&ULTRA_MAGIC);
        buf.push(ULTRA_VERSION);
        buf.push(0);
        encode_varint_to_ultra(n as u64, &mut buf);
        encode_varint_to_ultra(8, &mut buf); // 8 columns

        // Encode all columns
        id_enc.encode_to(&mut buf);
        name_enc.encode_to(&mut buf);
        age_enc.encode_to(&mut buf);
        city_enc.encode_to(&mut buf);
        dept_enc.encode_to(&mut buf);
        salary_enc.encode_to(&mut buf);
        exp_enc.encode_to(&mut buf);
        proj_enc.encode_to(&mut buf);

        buf.into_vec()
    }
}

// =============================================================================
// New Declarative Schema-Based Encoding
// =============================================================================

// Implement TableEncode with declarative schema - type-based API with offsets
impl TableEncode for Employee {
    fn schema() -> TableSchema {
        // Clean API with optimal compression using offsets where beneficial
        TableSchema::builder()
            .u16("id") // IDs 0-65535 fit in u16
            .string("name") // inline string (high cardinality)
            .u8_offset("age", 18) // ages 18-273 stored as 0-255
            .dict("city") // dictionary string (low cardinality)
            .dict("department") // dictionary string (low cardinality)
            .u32_offset("salary", 30_000) // salaries 30k+ with offset
            .u8("experience") // 0-255 years
            .u8("project_count") // 0-255 projects
            .build()
    }

    fn encode_with_schema(items: &[Self]) -> Vec<u8> {
        if items.is_empty() {
            let mut buf = UltraBuffer::with_capacity(16);
            buf.extend(&SCHEMA_MAGIC);
            buf.push(1);
            encode_varint_fast(0, &mut buf);
            return buf.into_vec();
        }

        let n = items.len();
        let schema = Self::schema();

        // Create adaptive encoders from schema
        let mut id_enc =
            AdaptiveIntEncoder::new(schema.encoding(0).unwrap_or(FieldEncoding::Auto), n);
        let mut name_enc =
            AdaptiveStringEncoder::new(schema.encoding(1).unwrap_or(FieldEncoding::Inline), n);
        let mut age_enc =
            AdaptiveIntEncoder::new(schema.encoding(2).unwrap_or(FieldEncoding::Auto), n);
        let mut city_enc =
            AdaptiveStringEncoder::new(schema.encoding(3).unwrap_or(FieldEncoding::Dictionary), n);
        let mut dept_enc =
            AdaptiveStringEncoder::new(schema.encoding(4).unwrap_or(FieldEncoding::Dictionary), n);
        let mut salary_enc =
            AdaptiveIntEncoder::new(schema.encoding(5).unwrap_or(FieldEncoding::Auto), n);
        let mut exp_enc =
            AdaptiveIntEncoder::new(schema.encoding(6).unwrap_or(FieldEncoding::Auto), n);
        let mut proj_enc =
            AdaptiveIntEncoder::new(schema.encoding(7).unwrap_or(FieldEncoding::Auto), n);

        // Single pass through data
        for emp in items {
            id_enc.push_u32(emp.id);
            name_enc.push(&emp.name);
            age_enc.push_u32(emp.age);
            city_enc.push(&emp.city);
            dept_enc.push(&emp.department);
            salary_enc.push_u32(emp.salary);
            exp_enc.push_u32(emp.experience);
            proj_enc.push_u32(emp.project_count);
        }

        // Estimate output size
        let estimated = n * 30 + 512;
        let mut buf = UltraBuffer::with_capacity(estimated);

        // Header
        buf.extend(&SCHEMA_MAGIC);
        buf.push(1);
        encode_varint_fast(n as u64, &mut buf);
        encode_varint_fast(8, &mut buf);

        // Encode all columns - encoders adapt based on schema + actual data
        id_enc.encode_to(&mut buf);
        name_enc.encode_to(&mut buf);
        age_enc.encode_to(&mut buf);
        city_enc.encode_to(&mut buf);
        dept_enc.encode_to(&mut buf);
        salary_enc.encode_to(&mut buf);
        exp_enc.encode_to(&mut buf);
        proj_enc.encode_to(&mut buf);

        buf.into_vec()
    }
}

fn generate_employees(count: usize, seed: u64) -> Vec<Employee> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let first_names = [
        "Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Hank", "Ivy", "Jack", "Kate",
        "Liam", "Maya", "Noah", "Olivia", "Pete",
    ];
    let cities = [
        "NYC",
        "LA",
        "Chicago",
        "Houston",
        "Phoenix",
        "Philadelphia",
        "Seattle",
        "Denver",
        "Boston",
        "Austin",
    ];
    let departments = [
        "Engineering",
        "Sales",
        "Marketing",
        "HR",
        "Finance",
        "Operations",
        "Support",
        "Legal",
        "Product",
        "Design",
    ];

    (0..count)
        .map(|i| {
            let first_name = first_names[rng.random_range(0..first_names.len())];
            let suffix = format!("{}{:03}", (b'A' + (i / 1000) as u8 % 26) as char, i % 1000);
            Employee {
                id: i as u32 + 1,
                name: format!("{} {}", first_name, suffix),
                age: rng.random_range(22..65),
                city: cities[rng.random_range(0..cities.len())].to_string(),
                department: departments[rng.random_range(0..departments.len())].to_string(),
                salary: rng.random_range(40000..180000),
                experience: rng.random_range(0..35),
                project_count: rng.random_range(1..50),
            }
        })
        .collect()
}

// ============================================================================
// TBF Benchmarks (Direct Serde Integration)
// ============================================================================

fn bench_tbf(c: &mut Criterion) {
    let mut group = c.benchmark_group("tbf_serde");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let tbf_bytes = tauq::tbf::to_bytes(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        // Direct serde serialization (fast path)
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| tauq::tbf::to_bytes(black_box(data)).unwrap()),
        );

        // Direct serde deserialization (fast path)
        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &tbf_bytes,
            |b, data| b.iter(|| tauq::tbf::from_bytes::<Vec<Employee>>(black_box(data)).unwrap()),
        );
    }

    group.finish();
}

// ============================================================================
// TBF Benchmarks (Traits-based - No Type Tags, Schema-Aware)
// ============================================================================

fn bench_tbf_traits(c: &mut Criterion) {
    let mut group = c.benchmark_group("tbf_traits");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let tbf_bytes = TbfEncode::tbf_encode_slice(&employees);

        group.throughput(Throughput::Elements(size as u64));

        // Traits-based serialization (no type tags)
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| TbfEncode::tbf_encode_slice(black_box(data))),
        );

        // Traits-based deserialization (no type tags)
        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &tbf_bytes,
            |b, data| b.iter(|| Employee::tbf_decode_slice(black_box(data)).unwrap()),
        );
    }

    group.finish();
}

// ============================================================================
// TBF Benchmarks (Columnar - Best Compression)
// ============================================================================

fn bench_tbf_columnar(c: &mut Criterion) {
    let mut group = c.benchmark_group("tbf_columnar");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let columnar_bytes = Employee::columnar_encode_slice(&employees);

        group.throughput(Throughput::Elements(size as u64));

        // Columnar serialization
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| Employee::columnar_encode_slice(black_box(data))),
        );

        // Columnar deserialization
        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &columnar_bytes,
            |b, data| b.iter(|| Employee::columnar_decode_slice(black_box(data)).unwrap()),
        );
    }

    group.finish();
}

// ============================================================================
// TBF Benchmarks (Fast - Optimized Serialization and Deserialization)
// ============================================================================

fn bench_tbf_fast(c: &mut Criterion) {
    let mut group = c.benchmark_group("tbf_fast");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let fast_bytes = fast_encode_slice(&employees);

        group.throughput(Throughput::Elements(size as u64));

        // Fast serialization (optimized hash + varint)
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| fast_encode_slice(black_box(data))),
        );

        // Fast deserialization (optimized varint + pre-resolved dictionary)
        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &fast_bytes,
            |b, data| b.iter(|| Employee::fast_decode_slice(black_box(data)).unwrap()),
        );
    }

    group.finish();
}

// ============================================================================
// TBF Benchmarks (Ultra - Bitcode-style columnar + adaptive packing)
// ============================================================================

fn bench_tbf_ultra(c: &mut Criterion) {
    let mut group = c.benchmark_group("tbf_ultra");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);

        group.throughput(Throughput::Elements(size as u64));

        // Ultra Direct serialization (direct encoders, no intermediate allocations)
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| Employee::ultra_encode_direct(black_box(data))),
        );
    }

    group.finish();
}

// ============================================================================
// TBF Benchmarks (Schema - Type-based declarative encoding)
// ============================================================================

fn bench_tbf_schema(c: &mut Criterion) {
    let mut group = c.benchmark_group("tbf_schema");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);

        group.throughput(Throughput::Elements(size as u64));

        // Type-based schema encoding with adaptive strategies
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| Employee::encode_with_schema(black_box(data))),
        );
    }

    group.finish();
}

// ============================================================================
// Comparison: JSON
// ============================================================================

fn bench_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("json");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let json_str = serde_json::to_string(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| serde_json::to_string(black_box(data)).unwrap()),
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &json_str,
            |b, data| b.iter(|| serde_json::from_str::<Vec<Employee>>(black_box(data)).unwrap()),
        );
    }

    group.finish();
}

// ============================================================================
// Comparison: Bitcode
// ============================================================================

fn bench_bitcode(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitcode");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = bitcode::encode(&employees);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| bitcode::encode(black_box(data))),
        );

        group.bench_with_input(BenchmarkId::new("deserialize", size), &bytes, |b, data| {
            b.iter(|| bitcode::decode::<Vec<Employee>>(black_box(data)).unwrap())
        });
    }

    group.finish();
}

// ============================================================================
// Comparison: Bincode
// ============================================================================

fn bench_bincode(c: &mut Criterion) {
    let mut group = c.benchmark_group("bincode");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = bincode::serde::encode_to_vec(&employees, bincode::config::standard()).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| {
                b.iter(|| {
                    bincode::serde::encode_to_vec(black_box(data), bincode::config::standard())
                        .unwrap()
                })
            },
        );

        group.bench_with_input(BenchmarkId::new("deserialize", size), &bytes, |b, data| {
            b.iter(|| {
                bincode::serde::decode_from_slice::<Vec<Employee>, _>(
                    black_box(data),
                    bincode::config::standard(),
                )
                .unwrap()
            })
        });
    }

    group.finish();
}

// ============================================================================
// Comparison: Postcard
// ============================================================================

fn bench_postcard(c: &mut Criterion) {
    let mut group = c.benchmark_group("postcard");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = postcard::to_allocvec(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| postcard::to_allocvec(black_box(data)).unwrap()),
        );

        group.bench_with_input(BenchmarkId::new("deserialize", size), &bytes, |b, data| {
            b.iter(|| postcard::from_bytes::<Vec<Employee>>(black_box(data)).unwrap())
        });
    }

    group.finish();
}

// ============================================================================
// Comparison: MessagePack
// ============================================================================

fn bench_msgpack(c: &mut Criterion) {
    let mut group = c.benchmark_group("msgpack");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = rmp_serde::to_vec(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| b.iter(|| rmp_serde::to_vec(black_box(data)).unwrap()),
        );

        group.bench_with_input(BenchmarkId::new("deserialize", size), &bytes, |b, data| {
            b.iter(|| rmp_serde::from_slice::<Vec<Employee>>(black_box(data)).unwrap())
        });
    }

    group.finish();
}

// ============================================================================
// Size Comparison
// ============================================================================

fn bench_size_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("size_comparison");

    for size in [100, 1000] {
        let employees = generate_employees(size, 42);

        // Calculate sizes
        let json_size = serde_json::to_string(&employees).unwrap().len();
        let tbf_serde_size = tauq::tbf::to_bytes(&employees).unwrap().len();
        let tbf_traits_size = TbfEncode::tbf_encode_slice(&employees).len();
        let tbf_fast_size = fast_encode_slice(&employees).len();
        let tbf_ultra_size = Employee::ultra_encode_direct(&employees).len();
        let tbf_schema_size = Employee::encode_with_schema(&employees).len();
        let tbf_columnar_size = Employee::columnar_encode_slice(&employees).len();
        let bitcode_size = bitcode::encode(&employees).len();
        let bincode_size = bincode::serde::encode_to_vec(&employees, bincode::config::standard())
            .unwrap()
            .len();
        let postcard_size = postcard::to_allocvec(&employees).unwrap().len();
        let msgpack_size = rmp_serde::to_vec(&employees).unwrap().len();

        println!("\n=== Size Comparison ({} records) ===", size);
        println!("JSON:              {:>8} bytes (baseline 100%)", json_size);
        println!(
            "TBF-schema:        {:>8} bytes ({:.1}% of JSON)  <- type-based declarative API",
            tbf_schema_size,
            (tbf_schema_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "TBF-ultra:         {:>8} bytes ({:.1}% of JSON)  <- columnar + adaptive packing",
            tbf_ultra_size,
            (tbf_ultra_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "TBF-columnar:      {:>8} bytes ({:.1}% of JSON)  <- columnar storage",
            tbf_columnar_size,
            (tbf_columnar_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "TBF-fast:          {:>8} bytes ({:.1}% of JSON)  <- optimized encode/decode",
            tbf_fast_size,
            (tbf_fast_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "TBF-traits:        {:>8} bytes ({:.1}% of JSON)  <- row-based, schema-aware",
            tbf_traits_size,
            (tbf_traits_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "Bitcode:           {:>8} bytes ({:.1}% of JSON)",
            bitcode_size,
            (bitcode_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "TBF-serde:         {:>8} bytes ({:.1}% of JSON)  <- serde-based",
            tbf_serde_size,
            (tbf_serde_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "Postcard:          {:>8} bytes ({:.1}% of JSON)",
            postcard_size,
            (postcard_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "Bincode:           {:>8} bytes ({:.1}% of JSON)",
            bincode_size,
            (bincode_size as f64 / json_size as f64) * 100.0
        );
        println!(
            "MsgPack:           {:>8} bytes ({:.1}% of JSON)",
            msgpack_size,
            (msgpack_size as f64 / json_size as f64) * 100.0
        );

        // Dummy benchmark for group
        group.bench_function(BenchmarkId::new("calc", size), |b| {
            b.iter(|| black_box(json_size + tbf_columnar_size))
        });
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    tbf_benches,
    bench_tbf,
    bench_tbf_traits,
    bench_tbf_fast,
    bench_tbf_ultra,
    bench_tbf_schema,
    bench_tbf_columnar,
    bench_json,
    bench_bitcode,
    bench_bincode,
    bench_postcard,
    bench_msgpack,
    bench_size_comparison,
);

criterion_main!(tbf_benches);
