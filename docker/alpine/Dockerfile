ARG OS_NAME=alpine
ARG OS_VERSION=3.10.3

FROM ${OS_NAME}:${OS_VERSION} as builder

LABEL pycors_${OS_NAME}_${OS_VERSION}_builder=true

ARG DOCKER_GID=7926
ARG DOCKER_UID=7926
ARG RUST_VERSION=1.39.0
ARG RUSTFLAGS

ENV RUST_VERSION=${RUST_VERSION}
ENV RUST_LOG=pycors=debug
ENV RUSTFLAGS=${RUSTFLAGS}

# -------------------------------------------------------------------------------
# OS-specific
RUN apk add --no-cache \
    vim \
    sudo \
    # To download rustup
    curl \
    # cargo dependencies
    build-base \
    # pycors dependencies
    openssl-dev \
    pkgconf
# -------------------------------------------------------------------------------



RUN addgroup -g ${DOCKER_GID} -S pycors && adduser -u ${DOCKER_UID} -S pycors -G pycors && \
    chown pycors:pycors /home/pycors

# Let user run sudo without password
RUN echo "pycors ALL=(ALL) NOPASSWD:ALL" | EDITOR='tee -a' visudo

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

RUN cargo build

# Create cache archive
RUN tar -zcf docker_cargo_cache.tar.gz .cargo target

RUN cargo run -- setup bash
# -------------------------------------------------------------------------------


# ########################################################################
FROM ${OS_NAME}:${OS_VERSION}

LABEL pycors_${OS_NAME}_${OS_VERSION}_builder=false

ARG DOCKER_GID=7926
ARG DOCKER_UID=7926

# -------------------------------------------------------------------------------
# OS-specific
# See https://devguide.python.org/setup/#linux
RUN apk add --no-cache \
    build-base \
    sudo \
    vim \
    # Python build dependencies
    # See https://git.alpinelinux.org/aports/tree/main/python3/APKBUILD?h=3.10-stable
    expat-dev openssl-dev zlib-dev ncurses-dev bzip2-dev xz-dev sqlite-dev libffi-dev \
    tcl-dev linux-headers gdbm-dev readline-dev
# -------------------------------------------------------------------------------





RUN addgroup -g ${DOCKER_GID} -S pycors && \
    adduser -u ${DOCKER_UID} -S pycors -G pycors
# Let user run sudo without password
RUN echo "pycors ALL=(ALL) NOPASSWD:ALL" | EDITOR='tee -a' visudo

USER pycors

COPY --chown=pycors:pycors --from=builder /home/pycors/.pycors /home/pycors/.pycors
COPY --chown=pycors:pycors --from=builder /home/pycors/.bashrc /home/pycors/.bashrc
COPY --chown=pycors:pycors --from=builder /home/pycors/docker_cargo_cache.tar.gz /home/pycors/docker_cargo_cache.tar.gz

WORKDIR /home/pycors

ENV PATH="/home/pycors/.pycors/shims:${PATH}"
