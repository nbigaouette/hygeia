ARG OS_NAME=fedora
ARG OS_VERSION=31

FROM ${OS_NAME}:${OS_VERSION} as builder

LABEL pycors_${OS_NAME}_${OS_VERSION}_builder=true

ARG RUST_VERSION=1.39.0
ARG DOCKER_USER=pycors


ENV RUST_VERSION=${RUST_VERSION}
ENV RUST_LOG=pycors=debug


# -------------------------------------------------------------------------------
# OS-specific
# SEE https://docs.fedoraproject.org/en-US/containers/guidelines/creation/
RUN dnf --setopt=tsflags=nodocs -y groupinstall "Development Tools" && \
    dnf --setopt=tsflags=nodocs -y install \
    # To download rustup
    curl \
    # pycors dependencies
    openssl-devel \
    && \
    dnf clean all
# -------------------------------------------------------------------------------


RUN groupadd --system ${DOCKER_USER} && \
    useradd --create-home --system --gid ${DOCKER_USER} ${DOCKER_USER} && \
    chown ${DOCKER_USER}:${DOCKER_USER} /home/${DOCKER_USER}
USER ${DOCKER_USER}

WORKDIR /home/${DOCKER_USER}

# Install Rust (through rustup)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile=minimal --default-toolchain ${RUST_VERSION}
ENV PATH="/home/pycors/.cargo/bin:${PATH}"


# -------------------------------------------------------------------------------
# Pycors specific
# Copy a cache and extract
COPY --chown=${DOCKER_USER}:${DOCKER_USER} docker/${OS_NAME}/artifacts/docker_cargo_cache.tar.gz* Cargo.* ./
COPY --chown=${DOCKER_USER}:${DOCKER_USER} src ./src
COPY --chown=${DOCKER_USER}:${DOCKER_USER} tests ./tests
COPY --chown=${DOCKER_USER}:${DOCKER_USER} xtask ./xtask
COPY --chown=${DOCKER_USER}:${DOCKER_USER} extra-packages-to-install.txt ./extra-packages-to-install.txt

RUN tar -zxf docker_cargo_cache.tar.gz || echo " ---> Cache file not found, skipping (please ignore tar error)."

RUN cargo build && \
    cargo run -- setup bash

# Create cache archive
RUN tar -zcf docker_cargo_cache.tar.gz .cargo target
# -------------------------------------------------------------------------------


# ########################################################################
FROM ${OS_NAME}:${OS_VERSION}

LABEL pycors_${OS_NAME}_${OS_VERSION}_builder=false

ARG DOCKER_USER=pycors

# -------------------------------------------------------------------------------
# OS-specific
# Python build dependencies
# See https://devguide.python.org/setup/#linux
RUN dnf --setopt=tsflags=nodocs -y install yum-utils && \
    dnf --setopt=tsflags=nodocs -y builddep python3 && \
    dnf --setopt=tsflags=nodocs -y groupinstall "Development Tools"
# -------------------------------------------------------------------------------



RUN groupadd --system ${DOCKER_USER} && \
    useradd --create-home --system --gid ${DOCKER_USER} ${DOCKER_USER}
USER ${DOCKER_USER}

COPY --chown=${DOCKER_USER}:${DOCKER_USER} --from=builder /home/${DOCKER_USER}/.pycors /home/${DOCKER_USER}/.pycors
COPY --chown=${DOCKER_USER}:${DOCKER_USER} --from=builder /home/${DOCKER_USER}/.bashrc /home/${DOCKER_USER}/.bashrc
COPY --chown=${DOCKER_USER}:${DOCKER_USER} --from=builder /home/${DOCKER_USER}/docker_cargo_cache.tar.gz /home/${DOCKER_USER}/docker_cargo_cache.tar.gz

ENV PATH="/home/${DOCKER_USER}/.pycors/shims:${PATH}"

WORKDIR /home/${DOCKER_USER}
