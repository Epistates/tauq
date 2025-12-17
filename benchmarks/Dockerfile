FROM python:3.12-slim

WORKDIR /app

# Install build dependencies and Rust toolchain for tauq
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"

# Install Python dependencies with latest tiktoken (o200k_base support)
RUN pip install --no-cache-dir tiktoken>=0.8.0

# Copy tauq source and build
COPY tauq_src /app/tauq_src
WORKDIR /app/tauq_src
RUN cargo build --release
WORKDIR /app

# Copy benchmark scripts
COPY benchmark_comprehensive.py /app/
COPY outputs/ /app/outputs/ 2>/dev/null || true

# Set environment for better output
ENV PYTHONUNBUFFERED=1

# Run comprehensive benchmark by default
CMD ["python3", "benchmark_comprehensive.py"]
