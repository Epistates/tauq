FROM python:3.12-slim

WORKDIR /app

# Install build dependencies and Rust toolchain for tauq
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"

# Install Python dependencies
RUN pip install --no-cache-dir tiktoken toon-python tabulate

# Copy tauq source and build
COPY tauq_src /app/tauq_src
WORKDIR /app/tauq_src
RUN cargo build --release
WORKDIR /app

COPY . /app

CMD ["python3", "benchmark.py"]
