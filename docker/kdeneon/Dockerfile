ARG OS_NAME=kdeneon/all
ARG OS_VERSION=user

FROM ${OS_NAME}:${OS_VERSION} as builder

LABEL hygeia_${OS_NAME}_${OS_VERSION}_builder=true

ARG DOCKER_GID=792677
ARG DOCKER_UID=792677
ARG RUST_VERSION=1.40.0


ENV RUST_VERSION=${RUST_VERSION}
ENV RUST_LOG=hygeia=debug

USER root

# -------------------------------------------------------------------------------
# OS-specific
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get -y install \
    vim \
    sudo \
    # To download rustup
    curl \
    # hygeia dependencies
    build-essential \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*
# -------------------------------------------------------------------------------

USER neon

WORKDIR /home/neon

# Install Rust (through rustup)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile=minimal --default-toolchain ${RUST_VERSION}
ENV PATH="/home/neon/.cargo/bin:${PATH}"


# -------------------------------------------------------------------------------
# Hygeia specific
# Copy a cache and extract
COPY --chown=neon:neon docker/${OS_NAME}/artifacts/docker_cargo_cache.tar.gz* Cargo.* ./
COPY --chown=neon:neon src ./src
COPY --chown=neon:neon tests ./tests
COPY --chown=neon:neon xtask ./xtask
COPY --chown=neon:neon hygeia_test_helpers ./hygeia_test_helpers
COPY --chown=neon:neon extra-packages-to-install.txt ./extra-packages-to-install.txt

RUN tar -zxf docker_cargo_cache.tar.gz || echo " ---> Cache file not found, skipping (please ignore tar error)."

RUN cargo build

# Create cache archive
RUN tar -zcf docker_cargo_cache.tar.gz .cargo target

RUN cargo run -- setup bash
# -------------------------------------------------------------------------------


# ########################################################################
FROM ${OS_NAME}:${OS_VERSION}

USER root

LABEL hygeia_${OS_NAME}_${OS_VERSION}_builder=false

# -------------------------------------------------------------------------------
# OS-specific
# Enable 'source' repositories
RUN sed -i '/deb-src/s/^# //' /etc/apt/sources.list
# Python build dependencies
# See https://devguide.python.org/setup/#linux
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get -y build-dep \
    python3 && \
    DEBIAN_FRONTEND=noninteractive apt-get -y install \
    libssl-dev zlib1g-dev libncurses5-dev libncursesw5-dev libreadline-dev libsqlite3-dev \
    libgdbm-dev libdb5.3-dev libbz2-dev libexpat1-dev liblzma-dev libffi-dev uuid-dev \
    ca-certificates \
    sudo \
    vim \
    && rm -rf /var/lib/apt/lists/*
# -------------------------------------------------------------------------------

USER neon

COPY --chown=neon:neon --from=builder /home/neon/.hygeia /home/neon/.hygeia
COPY --chown=neon:neon --from=builder /home/neon/.bashrc /home/neon/.bashrc
COPY --chown=neon:neon --from=builder /home/neon/docker_cargo_cache.tar.gz /home/neon/docker_cargo_cache.tar.gz

WORKDIR /home/neon

ENV PATH="/home/neon/.hygeia/shims:${PATH}"
