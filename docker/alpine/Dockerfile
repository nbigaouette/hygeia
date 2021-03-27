ARG OS_NAME=alpine
ARG OS_VERSION=3.10.3

FROM ${OS_NAME}:${OS_VERSION} as builder

LABEL hygeia_${OS_NAME}_${OS_VERSION}_builder=true

ARG DOCKER_GID=7926
ARG DOCKER_UID=7926
ARG RUST_VERSION=1.51.0
ARG RUSTFLAGS

ENV RUST_VERSION=${RUST_VERSION}
ENV RUST_LOG=hygeia=debug
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
    # hygeia dependencies
    openssl-dev \
    pkgconf
# -------------------------------------------------------------------------------



RUN addgroup -g ${DOCKER_GID} -S hygeia && adduser -u ${DOCKER_UID} -S hygeia -G hygeia && \
    chown hygeia:hygeia /home/hygeia

# Let user run sudo without password
RUN echo "hygeia ALL=(ALL) NOPASSWD:ALL" | EDITOR='tee -a' visudo

USER hygeia

WORKDIR /home/hygeia

# Install Rust (through rustup)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile=minimal --default-toolchain ${RUST_VERSION}
ENV PATH="/home/hygeia/.cargo/bin:${PATH}"


# -------------------------------------------------------------------------------
# Hygeia specific
# Copy a cache and extract
COPY --chown=hygeia:hygeia docker/${OS_NAME}/artifacts/docker_cargo_cache.tar.gz* Cargo.* ./
COPY --chown=hygeia:hygeia src ./src
COPY --chown=hygeia:hygeia tests ./tests
COPY --chown=hygeia:hygeia xtask ./xtask
COPY --chown=hygeia:hygeia hygeia_test_helpers ./hygeia_test_helpers
COPY --chown=hygeia:hygeia extra-packages-to-install.txt ./extra-packages-to-install.txt

RUN tar -zxf docker_cargo_cache.tar.gz || echo " ---> Cache file not found, skipping (please ignore tar error)."

RUN cargo build

# Create cache archive
RUN tar -zcf docker_cargo_cache.tar.gz .cargo target

RUN cargo run -- setup bash
# -------------------------------------------------------------------------------


# ########################################################################
FROM ${OS_NAME}:${OS_VERSION}

LABEL hygeia_${OS_NAME}_${OS_VERSION}_builder=false

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





RUN addgroup -g ${DOCKER_GID} -S hygeia && \
    adduser -u ${DOCKER_UID} -S hygeia -G hygeia
# Let user run sudo without password
RUN echo "hygeia ALL=(ALL) NOPASSWD:ALL" | EDITOR='tee -a' visudo

USER hygeia

COPY --chown=hygeia:hygeia --from=builder /home/hygeia/.hygeia /home/hygeia/.hygeia
COPY --chown=hygeia:hygeia --from=builder /home/hygeia/.bashrc /home/hygeia/.bashrc
COPY --chown=hygeia:hygeia --from=builder /home/hygeia/docker_cargo_cache.tar.gz /home/hygeia/docker_cargo_cache.tar.gz

WORKDIR /home/hygeia

ENV PATH="/home/hygeia/.hygeia/shims:${PATH}"
