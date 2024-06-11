# NOTE: only a single worker needs to be deployed in k8s for this step.

FROM rustlang/rust:nightly-bullseye-slim@sha256:2be4bacfc86e0ec62dfa287949ceb47f9b6d9055536769bdee87b7c1788077a9 as builder

# Install jemalloc
RUN apt-get update && apt-get install -y libjemalloc2 libjemalloc-dev make clang-16

# Install cargo-pgo, used for building a binary with profiling enabled
RUN cargo install cargo-pgo

RUN \
    mkdir -p common/src  && touch common/src/lib.rs && \
    mkdir -p ops/src     && touch ops/src/lib.rs && \
    mkdir -p worker/src  && echo "fn main() {println!(\"YO!\");}" > worker/src/main.rs

COPY Cargo.toml .
RUN sed -i "2s/.*/members = [\"common\", \"ops\", \"worker\"]/" Cargo.toml
COPY Cargo.lock .

COPY common/Cargo.toml ./common/Cargo.toml
COPY ops/Cargo.toml ./ops/Cargo.toml
COPY worker/Cargo.toml ./worker/Cargo.toml

COPY ./rust-toolchain.toml ./

# do not need to specify `--release`, it is added automatically by `cargo pgo`.
RUN cargo pgo build -- --bin worker

COPY common ./common
COPY ops ./ops
COPY worker ./worker
RUN \
    touch common/src/lib.rs && \
    touch ops/src/lib.rs && \
    touch worker/src/main.rs

RUN cargo pgo build -- --bin worker

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates libjemalloc2
COPY --from=builder ./target/x86_64-unknown-linux-gnu/release/worker /usr/local/bin/worker

# TODO: should we specify the block to run profiling with in this command?
#   or leave that to the CICD?
# Recommended block=4825, checkpoint=4824
CMD ["worker"]

# NOTE: after deploying this and running it with an example block, the profiling data will be available here (default path), as a single file:
#   `./target/pgo-profiles/<SOME_RANDOM_HASH>.profraw`
# but you can configure the file path by setting this environment variable:
#   `export LLVM_PROFILE_FILE="./EXAMPLE/PATH/TO/PROFILING_DATA/%m.profraw"`
# This file will need to be uploaded to the CICD somehow, so that the `deploy-worker.Dockerfile` can download it and use it to compile the optimized worker.
