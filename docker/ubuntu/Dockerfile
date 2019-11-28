ARG OS_NAME=ubuntu
ARG OS_VERSION=16.04

FROM ${OS_NAME}:${OS_VERSION} as builder

LABEL pycors_${OS_NAME}_${OS_VERSION}_builder=true

ARG DOCKER_GID=999
ARG DOCKER_UID=999
ARG RUST_VERSION=1.39.0


ENV RUST_VERSION=${RUST_VERSION}
ENV RUST_LOG=pycors=debug


# -------------------------------------------------------------------------------
# OS-specific
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get -y install \
    # To download rustup
    curl \
    # pycors dependencies
    build-essential \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*
# -------------------------------------------------------------------------------



RUN groupadd --system pycors --gid ${DOCKER_GID} && \
    useradd --create-home --system --gid pycors --uid ${DOCKER_UID} pycors && \
    chown pycors:pycors /home/pycors
USER pycors

WORKDIR /home/pycors

# Install Rust (through rustup)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile=minimal --default-toolchain ${RUST_VERSION}
ENV PATH="/home/pycors/.cargo/bin:${PATH}"


# -------------------------------------------------------------------------------
# Pycors specific
# Copy a cache and extract
COPY --chown=pycors:pycors docker/${OS_NAME}/artifacts/docker_cargo_cache.tar.gz* Cargo.* ./
COPY --chown=pycors:pycors src ./src
COPY --chown=pycors:pycors tests ./tests
COPY --chown=pycors:pycors xtask ./xtask
COPY --chown=pycors:pycors extra-packages-to-install.txt ./extra-packages-to-install.txt

RUN tar -zxf docker_cargo_cache.tar.gz || echo " ---> Cache file not found, skipping (please ignore tar error)."

RUN cargo build && \
    cargo run -- setup bash

# Create cache archive
RUN tar -zcf docker_cargo_cache.tar.gz .cargo target
# -------------------------------------------------------------------------------


# ########################################################################
FROM ${OS_NAME}:${OS_VERSION}

LABEL pycors_${OS_NAME}_${OS_VERSION}_builder=false

ARG DOCKER_GID=999
ARG DOCKER_UID=999

# -------------------------------------------------------------------------------
# OS-specific
# Enable 'source' repositories
RUN sed -i '/deb-src/s/^# //' /etc/apt/sources.list
# Python build dependencies
# See https://devguide.python.org/setup/#linux
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get -y build-dep \
    python3 && \
    && rm -rf /var/lib/apt/lists/*
# -------------------------------------------------------------------------------

RUN groupadd --system pycors --gid ${DOCKER_GID} && \
    useradd --create-home --system --gid pycors --uid ${DOCKER_UID} pycors
# Let user run sudo without password
RUN echo "pycors ALL=(ALL) NOPASSWD:ALL" | EDITOR='tee -a' visudo

USER pycors

COPY --chown=pycors:pycors --from=builder /home/pycors/.pycors /home/pycors/.pycors
COPY --chown=pycors:pycors --from=builder /home/pycors/.bashrc /home/pycors/.bashrc
COPY --chown=pycors:pycors --from=builder /home/pycors/docker_cargo_cache.tar.gz /home/pycors/docker_cargo_cache.tar.gz

WORKDIR /home/pycors

ENV PATH="/home/pycors/.pycors/shims:${PATH}"
